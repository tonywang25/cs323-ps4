#![crate_type="staticlib"]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

extern crate libc;

use libc::c_char;
use std::ffi::CStr;
use std::str;
use std::sync::Arc;

mod process;
use process::r_process;

pub enum Type {
  // Token types used by tokenize() et al.

  SIMPLE,           // Maximal contiguous sequence ... (as above)

  RED_IN,           // <    Redirect stdin to file
  RED_IN_HERE,      // <<   Redirect stdin to HERE document

  RED_OUT,          // >    Redirect stdout to file
  RED_OUT_APP,      // >>   Append stdout to file
  RED_OUT_ERR,      // &>   Redirect stdout and stderr to file (UNUSED)

  RED_ERR,          // 2>   Redirect stderr to file (UNUSED)
  RED_ERR_APP,      // 2>>  Append stderr to file (UNUSED)

  PIPE,             // |

  SEP_AND,          // &&
  SEP_OR,           // ||

  SEP_END,          // ;
  SEP_BG,           // &

  PAR_LEFT,         // (
  PAR_RIGHT,        // )

  // Other types used by parse()

  NONE,             // Nontoken: Did not find a token
  ERROR,            // Nontoken: Encountered an error
  SUBCMD            // Nontoken: CMD struct for subcommand
}

pub struct CMD {
  pub node: u32,
  pub argc: u32,
  pub argv: Vec<Option<String>>,
  pub nLocal: u32,
  pub locVar: Vec<Option<String>>,
  pub locVal: Vec<Option<String>>,
  pub fromType: u32,
  pub fromFile: Option<String>,
  pub toType: u32,
  pub toFile: Option<String>,
  pub errType: u32,
  pub errFile: Option<String>,
  pub left: Option<Arc<CMD>>,
  pub right: Option<Arc<CMD>>,
}

extern {
  fn get_type(addr: u64) -> u32;
  fn get_argc(addr: u64) -> u32;
  fn get_argv(addr: u64) -> u64;
  fn get_nLocal(addr: u64) -> u32;
  fn get_locVar(addr: u64) -> u64;
  fn get_locVal(addr: u64) -> u64;
  fn get_fromType(addr: u64) -> u32;
  fn get_fromFile(addr: u64) -> u64;
  fn get_toType(addr: u64) -> u32;
  fn get_toFile(addr: u64) -> u64;
  fn get_errType(addr: u64) -> u32;
  fn get_errFile(addr: u64) -> u64;
  fn get_left(addr: u64) -> u64;
  fn get_right(addr: u64) -> u64;
  fn get_cstr(addr: u64, idx: u32) -> u64;
}

fn cstr2String(addr: u64) -> Option<String> {
  if addr == 0 { return None; }
  let c_buf: *const c_char = addr as *const c_char;
  let c_str: &CStr = unsafe { CStr::from_ptr(c_buf) };
  let str_slice: &str = c_str.to_str().unwrap();
  Some(str_slice.to_owned())
}

fn get_cstr_array(n: u32, addr: u64) -> Vec<Option<String>> {
  let mut vec: Vec<Option<String>> = Vec::with_capacity(n as usize);
  for i in 0..n {
    vec.push(cstr2String(unsafe { get_cstr(addr, i) }));
  }
  vec
}

fn translate(raw_CMD: u64) -> Option<Arc<CMD>> {
  if raw_CMD == 0 { return None; }
  let node = unsafe { get_type(raw_CMD) };
  let argc = unsafe { get_argc(raw_CMD) };
  let argv = get_cstr_array(argc, unsafe { get_argv(raw_CMD) });
  let nLocal = unsafe { get_nLocal(raw_CMD) };
  let locVar = get_cstr_array(nLocal, unsafe { get_locVar(raw_CMD) });
  let locVal = get_cstr_array(nLocal, unsafe { get_locVal(raw_CMD) });
  let fromType = unsafe { get_fromType(raw_CMD) };
  let fromFile = cstr2String(unsafe { get_fromFile(raw_CMD) });
  let toType = unsafe { get_toType(raw_CMD) };
  let toFile = cstr2String(unsafe { get_toFile(raw_CMD) });
  let errType = unsafe { get_errType(raw_CMD) };
  let errFile = cstr2String(unsafe { get_errFile(raw_CMD) });
  let left = translate(unsafe { get_left(raw_CMD) });
  let right = translate(unsafe { get_right(raw_CMD) });
  Some(Arc::new(CMD {
    node: node,
    argc: argc,
    argv: argv,
    nLocal: nLocal,
    locVar: locVar,
    locVal: locVal,
    fromType: fromType,
    fromFile: fromFile,
    toType: toType,
    toFile: toFile,
    errType: errType,
    errFile: errFile,
    left: left,
    right: right,
  }))
}

#[no_mangle]
pub extern fn process(raw_CMD: u64) -> u32 {
  if let Some(CMD) = translate(raw_CMD) {
    r_process(CMD)
  }
  else {
    0
  }
}
