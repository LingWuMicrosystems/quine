#![no_std]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod common;
pub mod related_egraph;
pub mod rule;
pub mod table;
pub mod types;
pub mod uf;
