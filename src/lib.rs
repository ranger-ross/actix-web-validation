#[cfg(all(feature = "exclusive", feature = "validator", feature = "garde"))]
compile_error!("\n\n\nIMPORTANT: actix-web-validation `exclusive` feature is enabled with multiple other features. Check your Cargo.toml file. \n\n\n\n");

#[cfg(feature = "garde")]
pub mod garde;
#[cfg(feature = "validator")]
pub mod validator;

#[cfg(all(feature = "validator", feature = "exclusive"))]
pub use validator::Validated;

#[cfg(all(feature = "garde", feature = "exclusive"))]
pub use garde::Validated;
