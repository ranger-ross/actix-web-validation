#[cfg(feature = "garde")]
pub mod garde;
#[cfg(feature = "validator")]
pub mod validator;

#[cfg(all(feature = "validator", feature = "exclusive"))]
pub use validator::Validated;

#[cfg(all(feature = "garde", feature = "exclusive"))]
pub use garde::Validated;
