#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![feature(min_specialization)]

pub mod factory;
pub mod traits;

pub use factory::*;
pub use traits::*;
