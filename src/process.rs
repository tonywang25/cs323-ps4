use std::ffi::{c_void, CString};
use nix::fcntl::{open, OFlag};
use nix::sys::stat::Mode;
use std::os::raw::c_char;
use std::path::Path;

use crate::*;
use libc::{unlink, O_APPEND, O_CREAT, O_TRUNC, O_RDONLY, O_WRONLY, STDIN_FILENO, STDOUT_FILENO};
use nix::sys::wait::{self, WaitStatus};
use nix::unistd::{fork, execvp, pipe, getpid, Pid, ForkResult};

// use crate::syscall::*;
pub fn r_process(_cmdList: Arc<CMD>) -> u32 {
    if _cmdList.node == Type::SIMPLE as u32 {
        handle_simple(&_cmdList);
    } else {
        handle_pipe(&_cmdList);
    }
    0
}

fn string2CStr (s: &str) -> CString {
    return std::ffi::CString::new(s).unwrap();
}

fn get_program_and_args(_cmdList: &Arc<CMD>) -> (CString, Vec<CString>) {
    let program = match &_cmdList.argv[0] {
        Some(prog) => string2CStr(prog),
        None => {
            eprintln!("No command");
            unsafe { libc::_exit(1)};
        }
    };
    // if all arguments aren't valid Some values, return error
    if !_cmdList.argv.iter().all(|arg| arg.is_some()) {
        eprintln!("Some args are missing");
        unsafe { libc::_exit(1)};
    }
    // convert Option<String> items in args vector to CString
    let args: Vec<std::ffi::CString> = _cmdList.argv.iter().filter_map(|arg| match &arg {
        Some(a) => Some(string2CStr(a)), // String -> CString if some
        None => None
    }).collect(); // converts the op
    return (program, args)
}

fn handle_locals(_cmdList: &Arc<CMD>) -> () {
    for n in 0.._cmdList.nLocal as usize {
        let (name, val) = match (_cmdList.locVar[n].as_ref(), _cmdList.locVal[n].as_ref()) {
            (Some(n), Some(v)) => (n, v),
            _ => break,
        };
        let name_cstr = string2CStr(name);
        let name_ptr = name_cstr.as_ptr();
        let val_cstr = string2CStr(val);
        let val_ptr = val_cstr.as_ptr();
        unsafe {
            libc::setenv(name_ptr, val_ptr, 1);
        }
    }
}

fn process_simple(_cmdList: &Arc<CMD>) -> () {
    // 1. Handle Locals
    handle_locals(&_cmdList);
    // 2. prepare program and args
    let (program, args) = get_program_and_args(&_cmdList);
    // 3. Handle redirection (if necessary)
    handle_redirection(&_cmdList);
    // 4. EXECVP CALL
    match execvp(&program, &args) {
        Ok(_) => (),
        Err(_) => {
            unsafe { libc::perror(std::ffi::CString::new("Execvp failed").unwrap().as_ptr()) };
        }
    }
    unsafe { libc::_exit(0) };
}

pub fn handle_simple(_cmdList: &Arc<CMD>) -> u32 {
    match unsafe{fork()} {
        Ok(ForkResult::Parent { child, .. }) => {
            wait::waitpid(child, None).unwrap();
            0
        }
        Ok(ForkResult::Child) => {
            process_simple(_cmdList);
            0
        }
        Err(_) => {
            unsafe { libc::perror(std::ffi::CString::new("Fork failed").unwrap().as_ptr()) };
            1
        }
    }
}

fn handle_heredoc(_cmdList: &Arc<CMD>) -> () {
    let temp = "/tmp/Bash_heredoc_XXXXXX";
    let template = CString::new(temp).unwrap();
    let mut template_bytes = template.into_bytes_with_nul();
    let fd = unsafe { libc::mkstemp(template_bytes.as_mut_ptr() as *mut c_char) };
    if fd < 0 {
        let err = CString::new("mkstemp failed").unwrap();
        unsafe { libc::perror(err.as_ptr()) };
        unsafe { libc::_exit(1) };
    }
    let mut flag = b"w+".to_vec();
    let fp = unsafe { libc::fdopen(fd, flag.as_mut_ptr() as *const c_char) };
    if fp.is_null() {
        let err = CString::new("fdopen failed").unwrap();
        unsafe { libc::perror(err.as_ptr()) };
        unsafe { libc::_exit(1) };
    }
    // write to temp file
    if let Some(hd) = _cmdList.fromFile.as_ref() {
        unsafe { libc::fwrite(hd.as_ptr() as *mut c_void, 1, hd.len(), fp) };
    }
    // close temp file
    unsafe { libc::fclose(fp) };
    // replace stdin
    let ifd = unsafe { libc::open(template_bytes.as_ptr() as *const i8, O_RDONLY)};
    // flush the stream
    unsafe { libc::fflush(fp) };
    // reset file offset to beginning
    unsafe { libc::lseek(fd, 0, libc::SEEK_SET) };
    // unlink
    let _ = unsafe { unlink(template_bytes.as_ptr() as *const i8) };
    // overwrite stdin with ifd
    unsafe {
        libc::dup2(ifd, STDIN_FILENO); 
        libc::close(ifd);
    };
}

pub fn handle_redirection(_cmdList: &Arc<CMD>) -> u32 {
    if _cmdList.fromType != Type::NONE as u32 || _cmdList.toType != Type::NONE as u32 {
    // HERE DOC
        if _cmdList.fromType == Type::RED_IN_HERE as u32 {
            handle_heredoc(&_cmdList);
        }
        // redirecting, replacing stdin or stdout with the appropriate file
        match (_cmdList.fromFile.as_ref(), _cmdList.toFile.as_ref()) {
            (Some(fromFile), _) => {
                let ifd = unsafe { libc::open(string2CStr(&fromFile).as_ptr(), O_RDONLY, 0644)};
                // overwrite stdin with ifd
                unsafe {
                    libc::dup2(ifd, STDIN_FILENO); 
                    libc::close(ifd);
                };
            }
            (_, Some(toFile)) => {
                // let ofd = open(string2CStr(&toFile).as_ptr(), )
                let flags = OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_TRUNC;
                let mode = Mode::from_bits_truncate(0o644);
                let Ok(ofd) = open(Path::new(&toFile), flags, mode) else {
                    eprintln!("Error opening file {}", toFile);
                    unsafe { libc::_exit(1) }
                };
                // let ofd = open(Path::new(&toFile), flags, mode);
                // overwrite stdout with ofd
                unsafe {
                    libc::dup2(ofd, STDOUT_FILENO);
                    libc::close(ofd);
                }
            }
            _ => {
                unsafe { libc::perror(std::ffi::CString::new("redirection argument error").unwrap().as_ptr()) };
            }
        }
    }
    1
}

// recursive
pub fn create_cmd_array(_cmdList: &Arc<CMD>, cmdVec: &mut Vec<Arc<CMD>>) -> () {
    if _cmdList.node == Type::SIMPLE as u32 {
        let cmdListClone = _cmdList.clone();
        cmdVec.push(cmdListClone);
    } else {
        if let Some(left) = _cmdList.left.as_ref() {
            create_cmd_array(left, cmdVec);
        }
        if let Some(right) = _cmdList.right.as_ref() {
            create_cmd_array(right, cmdVec);
        }
    }
}

// function to handle pipes: iterative
pub fn handle_pipe(_cmdList: &Arc<CMD>) -> () {
    let mut cmdVec: Vec<Arc<CMD>> = Vec::new();
    create_cmd_array(&_cmdList, &mut cmdVec);
    // println!("----FLATTENED ARRAY----");
    // for cmd in cmdVec.iter() {
    //     for arg in cmd.argv.iter() {
    //         if let Some(s) = arg.as_ref() {
    //             print!("{} ", s);
    //         }
    //     }
    //     print!(" | ")
    // }
    // println!();
    // println!("-----------------------");
    #[derive(Clone)]
    struct Entry {
        pid: Pid,
        status: WaitStatus,
    }
    let mut table: Vec<Entry> = vec![Entry {pid: Pid::from_raw(0), status: WaitStatus::StillAlive}; cmdVec.len()];
    let mut fdin = 0;
    let len = cmdVec.len() - 1;

    for i in 0..len {
        if let Ok((fdr, fdw)) = pipe() {
            match unsafe { fork() } {
                Ok(ForkResult::Parent { child, .. }) => {
                    table[i].pid = child;
                    if i > 0 {
                        unsafe { libc::close(fdin) };
                    }
                    fdin = fdr;
                    unsafe { libc:: close(fdw) };
                },
                Ok(ForkResult::Child) => {
                    unsafe {
                        libc::close(fdr);
                        if fdin != 0 {
                            libc::dup2(fdin, STDIN_FILENO);
                            libc::close(fdin);
                        }
                        if fdw != 1 {
                            libc::dup2(fdw, STDOUT_FILENO);
                            libc::close(fdw);
                        }
                        process_simple(&cmdVec[i]);
                    }
                },
                Err(_) => {
                    unsafe { libc::perror(std::ffi::CString::new("Fork failed").unwrap().as_ptr()) };
                }
            };
        } else {
            unsafe { libc::perror(std::ffi::CString::new("Pipe failed").unwrap().as_ptr()) };
            ()
        }
    }
    
    // LAST PROCESS
    match unsafe{fork()} {
        Ok(ForkResult::Parent { child, .. }) => {
            table[len].pid = child;
        }
        Ok(ForkResult::Child) => {
            if fdin != 0 {
                unsafe {
                    libc::dup2(fdin, STDIN_FILENO);
                    libc::close(fdin);
                }
            }
            process_simple(&cmdVec[len]);
        }
        Err(_) => {
            unsafe { libc::perror(std::ffi::CString::new("Fork failed").unwrap().as_ptr()) };
            ()
        }
    }
    
    let mut i = 0;
    while i < cmdVec.len() {
        match wait::wait() {
            Ok(status) => {
                match status {
                    WaitStatus::Exited(pid, _) => {
                        let mut j = 0;
                        while j < cmdVec.len() && table[j].pid != pid {
                            j += 1;
                        }
                        if j < cmdVec.len() {
                            table[j].status = status;
                            i += 1;
                        }
                    },
                    _ => {}
                }
            }
            Err(e) => {
                unsafe { libc::perror(std::ffi::CString::new("Wait failed").unwrap().as_ptr()) };
            }
        }
    }
}