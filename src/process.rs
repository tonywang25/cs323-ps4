use crate::*;
use nix::sys::wait::waitpid;
use nix::unistd::{fork, execvp, ForkResult};

// use crate::syscall::*;
pub fn r_process(_cmdList: Arc<CMD>) -> u32 {
    handle_simple(_cmdList);
    0
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
                let name_cstr = std::ffi::CString::new(name.as_str()).unwrap();
                let name_ptr = name_cstr.as_ptr();
                let val_cstr = std::ffi::CString::new(val.as_str()).unwrap();
                let val_ptr = val_cstr.as_ptr();
                unsafe {
                    libc::setenv(name_ptr, val_ptr, 1);
                }
            }
            // 2. prepare program and args
            let program = match &_cmdList.argv[0] {
                Some(prog) => std::ffi::CString::new(prog.as_str()).unwrap(),
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
                Some(a) => Some(std::ffi::CString::new(a.as_str()).unwrap()), // String -> CString if some
                None => None
            }).collect(); // converts the op
            // 3. EXECVP CALL
            match execvp(&program, &args) {
                Ok(_) => (),
                Err(_) => {
                    unsafe {libc::perror(std::ffi::CString::new("Execvp failed").unwrap().as_ptr())};
                }
            }
            unsafe { libc::_exit(0) };
        }
        Err(_) => {
            unsafe {libc::perror(std::ffi::CString::new("Fork failed").unwrap().as_ptr())};
            1
        }
    }
}

// pub fn handle_redirection(_cmdList: Arc<CMD>) -> u32 {
//  1
// }
