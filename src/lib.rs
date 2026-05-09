#![no_std]
#![feature(portable_simd)]

extern crate alloc;

#[cfg(test)]
extern crate std;

pub mod common;
pub mod core;
pub mod frontend;
pub mod types;
pub mod uf;
pub mod min_parser;