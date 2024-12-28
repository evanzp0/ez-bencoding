#![feature(string_from_utf8_lossy_owned)]
#![feature(ptr_as_ref_unchecked)]
#![feature(str_from_raw_parts)]

mod error;
mod decode;

pub use error::*;
pub use decode::*;

type BdecodeResult<T> = std::result::Result<T, BdecodeError>;