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

macro_rules! validated_definition {
    () => {
        impl<T> Validated<T> {
            pub fn into_inner(self) -> T {
                self.0
            }
        }

        impl<T> std::ops::Deref for Validated<T> {
            type Target = T;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl<T> std::ops::DerefMut for Validated<T> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl<T> Debug for Validated<T>
        where
            T: Debug,
        {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple("Validated").field(&self.0).finish()
            }
        }
    };
}

pub(crate) use validated_definition;
