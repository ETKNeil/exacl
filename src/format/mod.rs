//! Implements helper functions for the built-in `AclEntry` format.

#[cfg(feature = "serde")]
mod format_serde;

#[cfg(not(feature = "serde"))]
mod format_no_serde;

#[cfg(feature = "serde")]
pub use format_serde::{
    read_aclentrykind, read_flagname, read_permname, write_aclentrykind, write_flagname,
    write_permname, Error,
};

#[cfg(not(feature = "serde"))]
pub use format_no_serde::{
    read_aclentrykind, read_flagname, read_permname, write_aclentrykind, write_flagname,
    write_permname, Error,
};
