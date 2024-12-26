#![feature(string_from_utf8_lossy_owned)]
#![feature(ptr_as_ref_unchecked)]

mod error;
mod commons;
mod token;
mod bdecode_node;
pub mod utils;

pub use error::*;
pub use commons::*;
pub use token::*;
pub use bdecode_node::*;

type BdecodeResult<T> = std::result::Result<T, BdecodeError>;