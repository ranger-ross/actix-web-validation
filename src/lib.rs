#![doc = include_str!("../README.md")]

#[cfg(feature = "custom")]
pub mod custom;
#[cfg(feature = "garde")]
pub mod garde;
#[cfg(feature = "validator")]
pub mod validator;

#[cfg(all(feature = "validator", not(feature = "garde"), not(feature = "custom")))]
pub use crate::validator::Validated;

#[cfg(all(feature = "garde", not(feature = "validator"), not(feature = "custom")))]
pub use crate::garde::Validated;

#[cfg(all(feature = "custom", not(feature = "validator"), not(feature = "garde")))]
pub use crate::custom::Validated;
