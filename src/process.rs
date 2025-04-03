use std::env::current_dir;
use std::ffi::{c_void, CString};
use std::path::PathBuf;
use std::sync::Mutex;
use nix::errno::Errno;
use std::os::raw::c_char;
use crate::*;
use libc::{ setenv, unlink, EXIT_FAILURE, O_APPEND, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY, STDIN_FILENO, STDOUT_FILENO};
use nix::sys::wait::{self, WaitPidFlag, WaitStatus};
use nix::unistd::{chdir, fork, execvp, pipe, Pid, ForkResult};

use thread_local::ThreadLocal;
use std::cell::RefCell;

thread_local! {
    static DIR_STACK: RefCell<Vec<PathBuf>> = RefCell::new(Vec::new());
}

#[derive(Clone)]
struct Entry {
    pid: Pid,
    status: WaitStatus,
}

struct Shell {
    dir_stack: Vec<String>,
}

static BUILT_INS: [&str; 3] = [
    "pushd",
    "popd",
    "cd",
];

// use crate::syscall::*;
pub fn r_process(_cmdList: Arc<CMD>) -> u32 {
    let exit_status = handle_any(&_cmdList);
    match wait::waitpid(Pid::from_raw(-1), Some(WaitPidFlag::WNOHANG)) {
        Ok(WaitStatus::Exited(pid, status)) => {
            eprintln!("Completed: {} ({})", pid, status);
        },
        _ => {}
    }
    return exit_status;
}

fn handle_any(_cmdList: &Arc<CMD>) -> u32 {
    let exit_status = match _cmdList.node {
        x if x == Type::SIMPLE as u32 => {
            handle_simple(&_cmdList) as u32
        },
        x if x == Type::PIPE as u32 => {
            handle_pipe(&_cmdList) as u32
        },
        x if x == Type::SEP_AND as u32 || x == Type::SEP_OR as u32 => {
            handle_cond(&_cmdList) as u32
        },
        x if x == Type::SEP_BG as u32 => {
            handle_bg(&_cmdList) as u32
        },
        x if x == Type::SEP_END as u32 => {
            handle_sep_end(&_cmdList) as u32
        },
        x if x == Type::SUBCMD as u32 => {
            handle_subcmd(&_cmdList) as u32
        },
        _ => 0
    };
    let name_cstr = string2CStr("?");
    let val_cstr = string2CStr(exit_status.to_string().as_str());
    unsafe { setenv(name_cstr.as_ptr(), val_cstr.as_ptr(), 1); }
    return exit_status;
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

fn handle_locals(_cmdList: &Arc<CMD>) -> Result<(), Errno>  {
    for n in 0.._cmdList.nLocal as usize {
        let (name, val) = match (_cmdList.locVar[n].as_ref(), _cmdList.locVal[n].as_ref()) {
            (Some(n), Some(v)) => (n, v),
            _ => break,
        };
        let name_cstr = string2CStr(name);
        let val_cstr = string2CStr(val);
        unsafe {
            if setenv(name_cstr.as_ptr(), val_cstr.as_ptr(), 1) < 0 {
                return Err(Errno::last());
            }
        }
    }
    Ok(())
}

fn cd_dir_name(dirName: &PathBuf) -> Result<(), Errno> {
    match chdir(dirName) {
        Ok(_) => (),
        Err(_) => {
            unsafe {
                libc::perror(std::ffi::CString::new("chdir failed").unwrap().as_ptr());
                return Err(Errno::last());
            }
        }
    }
    Ok(())
}
fn process_cd(_cmdList: &Arc<CMD>) -> u32 {
    match _cmdList.argv.len() {
        // "cd"
        1 => {
            let key = "HOME";
            match std::env::var(key) {
                Ok(path) => {
                    // println!("path: {}", path);
                    match chdir(PathBuf::from(path).as_path()) {
                        Ok(_) => 0,
                        Err(_) => {
                            unsafe {
                                libc::perror(std::ffi::CString::new("chdir failed").unwrap().as_ptr());
                            }
                            return Errno::last() as u32;
                        }
                    }
                },
                Err(_) => {
                    unsafe {
                        libc::perror(std::ffi::CString::new("undefined").unwrap().as_ptr());
                    }
                    1
                }
            }
        },
        // "cd [dir]"
        2 => {
            if let Some(dir) = _cmdList.argv[1].as_ref() { 
                if let Err(e) = cd_dir_name(&PathBuf::from(dir)) {
                    return e as u32;
                };
            }
            return 1;
        },
        _ => {
            unsafe {
                libc::perror(std::ffi::CString::new("too many argument").unwrap().as_ptr());
            }
            return 1;
        }
    }
}

fn process_pushd(_cmdList: &Arc<CMD>) -> u32 {
    if _cmdList.argc != 2 {
        return 1;
    } else {
        if let Some(dirName) = _cmdList.argv[1].as_ref() {
            // get current directory
            let currDir = match current_dir() {
                Ok(path) => path,
                Err(_) => return Errno::last() as u32
            };
            // push curr dir to dir stack
            DIR_STACK.with(|stack| {
                // clone to avoid borrowing
                stack.borrow_mut().push(currDir.clone());
            });
            // cd to dirName
            if let Err(e) = cd_dir_name(&PathBuf::from(dirName)){
                return e as u32;
            };
            // print
            println!("{}", currDir.display());
            DIR_STACK.with(|stack| {
                for dir in stack.borrow_mut().iter() {
                    println!("{} ", dir.display());
                }
            });
        } else {
            return Errno::EINVAL as u32;
        }
    }
    0
}

fn process_popd(_cmdList: &Arc<CMD>) -> u32 {
    if _cmdList.argc != 2 {
        return 1;
    } else {
         // pdir is None if stack is empty
        let pdir = DIR_STACK.with(|stack| {
            stack.borrow_mut().pop()
        });
        if let Some(dir) = pdir {
            if let Err(e) = cd_dir_name(&dir) {
                return e as u32;
            };
        }
        0
    }
}

fn exec_simple(_cmdList: &Arc<CMD>) -> Result<(), Errno> {
     // 1. Handle Locals
     handle_locals(&_cmdList)?;
     // 2. prepare program and args
     let (program, args) = get_program_and_args(&_cmdList);
     // 3. Handle redirection (if necessary)
     handle_redirection(&_cmdList)?;
     // 4. EXECVP CALL
     match execvp(&program, &args) {
         Ok(_) => (),
         Err(_) => {
             unsafe { 
                 libc::perror(std::ffi::CString::new("Execvp failed").unwrap().as_ptr());
                 libc::_exit(Errno::last() as i32);
             }
         }
     }
     Ok(())
}

fn process_simple(_cmdList: &Arc<CMD>) -> u32 {
    match unsafe{fork()} {
        Ok(ForkResult::Parent { child, .. }) => {
            let status = wait::waitpid(child, None).unwrap();
            match status {
                WaitStatus::Exited(_, code) => code as u32,
                WaitStatus::Signaled(_, signal, _) => 128 + signal as u32,
                // no errors!
                _ => 0,
            }
        },
        Ok(ForkResult::Child) => {
            // return propagated error in setup phase first
            if let Err(e) = exec_simple(_cmdList) {
                return e as u32;
            // if no propagated error, move on to parent handling
            } else {
                std::process::exit(0);
            }
        },
        Err(_) => {
            unsafe { 
                libc::perror(std::ffi::CString::new("Fork failed").unwrap().as_ptr());
                libc::_exit(EXIT_FAILURE);
            }
        },
    }
}

fn process_built_in_simple(_cmdList: &Arc<CMD>, cmd: &str) -> u32 {
    match cmd {
        "cd" => return process_cd(_cmdList),
        "pushd" => return process_pushd(_cmdList),
        "popd" => return process_popd(_cmdList),
        _ => 1
    }
}

pub fn handle_simple(_cmdList: &Arc<CMD>) -> u32 {
    if let Some(cmd) =_cmdList.argv[0].as_ref() {
        let command = cmd.as_str();
        if BUILT_INS.contains(&command) {
            return process_built_in_simple(_cmdList, &command);
        } else {
            return process_simple(_cmdList);
        }
    } else {
        1
    }
}

fn handle_sep_end(_cmdList: &Arc<CMD>) -> u32 {
    let mut left_status = 0;
    let mut right_status = 0;
    if let Some(left) = _cmdList.left.as_ref() {
        left_status = handle_any(left);
    }
    if let Some(right) = _cmdList.right.as_ref() {
        right_status = handle_any(right);
    }
    if left_status != 0 {
        return left_status;
    } else if right_status != 0 {
        return right_status;
    } else {
        return left_status;
    }
}

fn handle_heredoc(_cmdList: &Arc<CMD>) -> Result<i32, Errno> {
    let temp = "/tmp/Bash_heredoc_XXXXXX";
    let template = CString::new(temp).unwrap();
    let mut template_bytes = template.into_bytes_with_nul();
    let fd = unsafe { libc::mkstemp(template_bytes.as_mut_ptr() as *mut c_char) };
    if fd < 0 {
        return Err(Errno::last());
    }
    let mut flag = b"w+".to_vec();
    let fp = unsafe { libc::fdopen(fd, flag.as_mut_ptr() as *const c_char) };
    if fp.is_null() {
        let err = CString::new("fdopen failed").unwrap();
        unsafe {
            libc::perror(err.as_ptr());
            libc::_exit(EXIT_FAILURE);
        }
    }
    // write to temp file
    if let Some(hd) = _cmdList.fromFile.as_ref() {
        unsafe { 
            if libc::fwrite(hd.as_ptr() as *mut c_void, 1, hd.len(), fp) != hd.len() {
                return Err(Errno::last());
            }
        };
    }
    // flush the stream
    if unsafe { libc::fflush(fp) } < 0 {
        return Err(Errno::last());
    }
    // reset file offset to beginning
    if unsafe { libc::lseek(fd, 0, libc::SEEK_SET) } < 0 {
        return Err(Errno::last());
    };
    
    if unsafe { unlink(template_bytes.as_ptr() as *const i8) } < 0 {
        return Err(Errno::last());
    };
    Ok(fd)
}

pub fn handle_redirection(_cmdList: &Arc<CMD>) -> Result<(), Errno> {
    if _cmdList.fromType != Type::NONE as u32 || _cmdList.toType != Type::NONE as u32 {
    // HERE DOC
        if _cmdList.fromType == Type::RED_IN_HERE as u32 {
            let hfd = handle_heredoc(&_cmdList)?;
            let _ = dup2_safe_simple(hfd, STDIN_FILENO);
        } else {
            // redirecting, replacing stdin or stdout with the appropriate file
            match (_cmdList.fromFile.as_ref(), _cmdList.toFile.as_ref()) {
                (Some(fromFile), _) => {
                        let ifd = unsafe { libc::open(string2CStr(&fromFile).as_ptr(), O_RDONLY, 0o644)};
                        if ifd < 0 {
                            unsafe {
                                libc::perror(std::ffi::CString::new("open error").unwrap().as_ptr());
                                libc::_exit(EXIT_FAILURE);
                            }
                        }
                        let _ = dup2_safe_simple(ifd, STDIN_FILENO);
                }
                (_, Some(toFile)) => {
                    // let ofd = open(string2CStr(&toFile).as_ptr(), )
                    let flags = O_WRONLY | O_CREAT | O_TRUNC;
                    let ofd = unsafe { libc::open(string2CStr(&toFile).as_ptr(), flags as i32, 0o644)};
                    if ofd < 0 {
                        unsafe {
                            libc::perror(std::ffi::CString::new("open error").unwrap().as_ptr());
                            libc::_exit(EXIT_FAILURE);
                        }
                    }
                    // overwrite stdout with ofd
                    let _ = dup2_safe_simple(ofd, STDOUT_FILENO);
                }
                _ => {
                    Some(());
                }
            }
        }
    }
    Ok(())
}

// recursive
fn create_pipe_cmd_array(_cmdList: &Arc<CMD>, cmdVec: &mut Vec<Arc<CMD>>) -> () {
    if _cmdList.node == Type::SIMPLE as u32 {
        let cmdListClone = _cmdList.clone();
        cmdVec.push(cmdListClone);
    } else {
        if let Some(left) = _cmdList.left.as_ref() {
            create_pipe_cmd_array(left, cmdVec);
        }
        if let Some(right) = _cmdList.right.as_ref() {
            create_pipe_cmd_array(right, cmdVec);
        }
    }
}

fn dup2_safe_simple(source: i32, target: i32) -> Result<(), Errno> {
    unsafe {
        if libc::dup2(source, target) < 0 {
            return Err(Errno::last());
        }
        if libc::close(source) < 0 {
            return Err(Errno::last());
        }
    }
    Ok(())
}

fn dup2_safe_pipe(source: i32, target: i32) -> () {
    unsafe {
        if libc::dup2(source, target) < 0 {
            libc::_exit(Errno::last() as i32);
        };
        if libc::close(source) < 0 {
            libc::_exit(Errno::last() as i32);
        };
    }
}

// function to handle pipes: iterative
pub fn handle_pipe(_cmdList: &Arc<CMD>) -> i32 {
    let mut cmdVec: Vec<Arc<CMD>> = Vec::new();
    
    create_pipe_cmd_array(&_cmdList, &mut cmdVec);
    
    let mut table: Vec<Entry> = vec![Entry {pid: Pid::from_raw(0), status: WaitStatus::StillAlive}; cmdVec.len()];
    let mut fdin = 0;

    // loop through cmd array and process each simple cmd
    let mut i = 0;
    while i < cmdVec.len() - 1 {
        if let Ok((fdr, fdw)) = pipe() {
            match unsafe { fork() } {
                Ok(ForkResult::Parent { child, .. }) => {
                    table[i].pid = child;
                    if i > 0 {
                        unsafe { if libc::close(fdin) < 0 {
                            libc::_exit(Errno::last() as i32);
                        }};
                    }
                    fdin = fdr;
                    unsafe { if libc::close(fdw) < 0 {
                        libc::_exit(Errno::last() as i32);
                    }};
                },
                Ok(ForkResult::Child) => {
                    unsafe {
                        if libc::close(fdr) < 0 {
                            libc::_exit(Errno::last() as i32);
                        }
                        if fdin != 0 {
                            dup2_safe_pipe(fdin, STDIN_FILENO);
                        }
                        if fdw != 1 {
                            dup2_safe_pipe(fdw, STDOUT_FILENO);
                        }
                        if let Err(e) = exec_simple(&cmdVec[i]) {
                            libc::_exit(e as i32);
                        }
                    }
                },
                Err(_) => {
                    unsafe { 
                        libc::perror(std::ffi::CString::new("Fork failed").unwrap().as_ptr());
                        libc::_exit(EXIT_FAILURE);
                    }
                }
            };
        } else {
            unsafe { 
                libc::perror(std::ffi::CString::new("Fork failed").unwrap().as_ptr());
                libc::_exit(EXIT_FAILURE);
            }
        }
        i += 1;
    }
    
    // LAST PROCESS
    match unsafe{fork()} {
        Ok(ForkResult::Parent { child, .. }) => {
            table[cmdVec.len() - 1].pid = child;
            if i > 0 {
                unsafe { if libc::close(fdin) < 0 {
                    libc::_exit(Errno::last() as i32);
                }};
            }
        }
        Ok(ForkResult::Child) => {
            if fdin != 0 {
                dup2_safe_pipe(fdin, STDIN_FILENO);
            }
            if let Err(e) = exec_simple(&cmdVec[cmdVec.len() - 1]) {
                unsafe {libc::_exit(e as i32); }
            }
        }
        Err(_) => {
            unsafe { 
                libc::perror(std::ffi::CString::new("Fork failed").unwrap().as_ptr());
                libc::_exit(EXIT_FAILURE);
            }
        }
    }
    
    // WAIT AND COLLECT
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
                    _ => ()
                }
            }
            Err(_) => ()
        }
    }
    // RETURN RIGHTMOST STATUS
    for i in (0..cmdVec.len()).rev() {
        match table[i].status {
            WaitStatus::Exited(_, status) if status != 0 => {
                return status;
            },
            _ => ()
        }
    }
    0
}

fn handle_cond(_cmdList: &Arc<CMD>) -> u32 {
    if let Some(left) = _cmdList.left.as_ref() {
        let left_status = handle_any(&left);
        match _cmdList.node {
            x if x == Type::SEP_AND as u32 => {
                if left_status != 0 {
                    return left_status;
                }
                if let Some(right) = _cmdList.right.as_ref() {
                    return handle_any(&right);
                } else {
                    return 1;
                }
            },
            x if x == Type::SEP_OR as u32 => {
                if left_status == 0 {
                    return left_status;
                }
                if let Some(right) = _cmdList.right.as_ref() {
                    return handle_any(&right);
                } else {
                    return 1;
                }
            },
            _ => 1
        }
    } else {
        1
    }
}


// recursive
fn create_bg_cmd_arrays(_cmdList: &Arc<CMD>, bgVec: &mut Vec<bool>, cmdVec: &mut Vec<Arc<CMD>>) -> () {
    if let Some(left) = _cmdList.left.as_ref() {
        create_bg_cmd_arrays(left, bgVec, cmdVec);
    }
    // if sep_bg "&" AND the length of the vectors are more than 0
    if _cmdList.node == Type::SEP_BG as u32 && bgVec.len() > 0 {
        // set the previous bg value to true
        let prev_i = bgVec.len() - 1;
        bgVec[prev_i] = true;
    }
    // 
    if _cmdList.node != Type::SEP_BG as u32 && _cmdList.node != Type::SEP_END as u32 {
        let cmdListClone = _cmdList.clone();
        cmdVec.push(cmdListClone);
        bgVec.push(false);
    }
    if let Some(right) = _cmdList.right.as_ref() {
        create_bg_cmd_arrays(right, bgVec, cmdVec);
    }
}

fn background(_cmdList: &Arc<CMD>) {
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child, .. }) => {
            eprintln!("Backgrounded: {}", child);
        }
        Ok(ForkResult::Child) => {
            handle_any(&_cmdList);
            unsafe { libc::_exit(0)};
        }
        Err(_) => {
        }
    }
}

fn handle_bg(_cmdList: &Arc<CMD>) -> u32 {
    let mut bgVec: Vec<bool> = Vec::new();
    let mut cmdVec: Vec<Arc<CMD>> = Vec::new();
    create_bg_cmd_arrays(&_cmdList, &mut bgVec, &mut cmdVec);
    // println!("Background Vector: {:?}", bgVec);
    // print!("Command Vector: [");
    // for cmd in cmdVec.iter() {
    //     print!("{:?}, ", cmd.node);
    // }
    // println!("]");

    for i in 0..bgVec.len() {
        if bgVec[i] == true {
            background(&cmdVec[i]);
        } else {
            handle_any(&cmdVec[i]);
        }
    }
    0
}

fn handle_subcmd(_cmdList: &Arc<CMD>) -> u32 {
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child, .. }) => {
            let status = wait::waitpid(child, None).unwrap();
            match status {
                WaitStatus::Exited(_, code) => code as u32,
                WaitStatus::Signaled(_, signal, _) => 128 + signal as u32,
                // no errors!
                _ => 0,
            }
        }
        Ok(ForkResult::Child) => {
            // 1. Handle Locals
            let _ = handle_locals(&_cmdList);
            // 3. Handle redirection (if necessary)
            let _ = handle_redirection(&_cmdList);
            if let Some(left) = _cmdList.left.as_ref() {
                unsafe { libc::_exit(handle_any(left) as i32) };
            } else {
                unsafe { libc::_exit(EXIT_FAILURE);}
            }
        }
        Err(_) => {
            unsafe { libc::_exit(EXIT_FAILURE);}
        }
    }
}