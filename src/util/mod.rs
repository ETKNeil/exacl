//! Utility functions and constants for the underlying system API.
//!
//! This module combines code from `util_common`, `util_macos`, `util_linux`,
//! and `util_freebsd`.

mod util_common;

#[cfg(target_os = "freebsd")]
mod util_freebsd;

#[cfg(target_os = "linux")]
mod util_linux;

#[cfg(target_os = "macos")]
mod util_macos;

// Re-export acl_entry_t and acl_t from crate::sys.
pub use crate::sys::{acl_entry_t, acl_t};

pub use util_common::{
    xacl_create_entry, xacl_entry_count, xacl_foreach, xacl_free, xacl_from_text, xacl_get_perm,
    xacl_init, xacl_is_empty, xacl_set_perm, xacl_set_tag_type, xacl_to_text,
};

#[cfg(target_os = "freebsd")]
pub use util_freebsd::{
    xacl_get_file, xacl_get_flags, xacl_get_tag_qualifier, xacl_set_file, xacl_set_flags,
    xacl_set_tag_qualifier, xacl_is_posix,
};

#[cfg(target_os = "linux")]
pub use util_linux::{
    xacl_get_file, xacl_get_flags, xacl_get_tag_qualifier, xacl_set_file, xacl_set_flags,
    xacl_set_tag_qualifier, xacl_is_posix,
};

#[cfg(target_os = "macos")]
pub use util_macos::{
    xacl_get_acl_flags, xacl_get_file, xacl_get_flags, xacl_get_tag_qualifier, xacl_set_acl_flags,
    xacl_set_file, xacl_set_flags, xacl_set_tag_qualifier, xacl_is_posix,
};

#[cfg(test)]
#[cfg(target_os = "freebsd")]
mod util_freebsd_test;

#[cfg(test)]
#[cfg(target_os = "linux")]
mod util_linux_test;

#[cfg(test)]
#[cfg(target_os = "macos")]
mod util_macos_test;
