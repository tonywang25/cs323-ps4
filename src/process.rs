use crate::*;
use nix::sys::wait::waitpid;
use nix::unistd::{fork, execvp, ForkResult, write};
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
            write(libc::STDOUT_FILENO, "New child process\n".as_bytes()).ok();
            // EXECVP CALL
            let _ = execvp(&program, &args);
            unsafe { libc::_exit(0) };
        }
        Err(_) => {
            println!("Fork failed");
            1
        }
    }
}

// pub fn handle_redirection(_cmdList: Arc<CMD>) -> u32 {
//  1
// }
