use std::ffi::{c_void, CString};
use nix::fcntl::{open, OFlag};
use nix::sys::stat::Mode;
use std::os::raw::c_char;
use std::path::Path;

use crate::*;
use libc::{O_APPEND, O_CREAT, O_TRUNC, O_RDONLY, O_WRONLY, STDIN_FILENO, STDOUT_FILENO};
use nix::sys::wait::waitpid;
use nix::unistd::{fork, execvp, ForkResult};

// use crate::syscall::*;
pub fn r_process(_cmdList: Arc<CMD>) -> u32 {
    handle_simple(_cmdList);
    0
}

fn string2CStr (s: &str) -> CString {
    return std::ffi::CString::new(s).unwrap();
}

pub fn handle_simple(_cmdList: Arc<CMD>) -> u32 {
    match unsafe{fork()} {
        Ok(ForkResult::Parent { child, .. }) => {
            waitpid(child, None).unwrap();
            0
        }
        Ok(ForkResult::Child) => {
            // 1. Handle Locals
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
            // 2. prepare program and args
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
            
            if _cmdList.fromType != Type::NONE as u32 || _cmdList.toType != Type::NONE as u32 {
                handle_redirection(_cmdList);
            }

            // 3. EXECVP CALL
            match execvp(&program, &args) {
                Ok(_) => (),
                Err(_) => {
                    unsafe { libc::perror(std::ffi::CString::new("Execvp failed").unwrap().as_ptr()) };
                }
            }
            unsafe { libc::_exit(0) };
        }
        Err(_) => {
            unsafe { libc::perror(std::ffi::CString::new("Fork failed").unwrap().as_ptr()) };
            1
        }
    }
}

pub fn handle_redirection(_cmdList: Arc<CMD>) -> u32 {
    // HERE DOC
    if _cmdList.fromType == Type::RED_IN_HERE as u32 {
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
        // overwrite stdin with ifd
        unsafe {
            libc::dup2(ifd, STDIN_FILENO); 
            libc::close(ifd);
        };
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
    1
}
