#![cfg_attr(all(not(test), not(feature = "std")), no_std)]
#![feature(
    maybe_uninit_uninit_array,
    maybe_uninit_array_assume_init,
    const_fn_trait_bound
)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod error;
mod kv;
mod queue;
mod stack;

pub use {error::*, kv::*, queue::*, stack::*};
