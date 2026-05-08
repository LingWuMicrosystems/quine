#![no_std]
#![feature(portable_simd)]

extern crate alloc;

pub mod check_and_compile;
pub mod common;
pub mod core;
pub mod env;
pub mod error;
pub mod frontend;
pub mod syntax;
pub mod types;
pub mod uf;
