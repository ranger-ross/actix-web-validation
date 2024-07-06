#![doc = include_str!("../README.md")]

#[cfg(feature = "garde")]
pub mod garde;
#[cfg(feature = "validator")]
pub mod validator;

#[cfg(feature = "docsrs")]
compile_error!("doc");
