#![doc = include_str!("../README.md")]

#[cfg(feature = "custom")]
pub mod custom;
#[cfg(feature = "garde")]
pub mod garde;
#[cfg(feature = "validator")]
pub mod validator;
