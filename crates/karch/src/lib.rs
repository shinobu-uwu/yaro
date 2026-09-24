#![no_std]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

#[cfg(target_arch = "x86_64")]
pub mod x64;
