//! Provides `Acl` and `AclOption` implementation.

use crate::aclentry::AclEntry;
use crate::failx::{fail_custom, path_err};
#[cfg(target_os = "linux")]
use crate::flag::Flag;
use crate::util::*;

use bitflags::bitflags;
use scopeguard::{self, ScopeGuard};
use std::io;
use std::path::Path;

bitflags! {
    /// Controls how ACL's are accessed.
    #[derive(Default)]
    pub struct AclOption : u32 {
        /// Get/set the ACL of the symlink itself (macOS only).
        const SYMLINK_ACL = 0x01;

        /// Get/set the default ACL (Linux only).
        const DEFAULT_ACL = 0x02;

        /// Ignore expected error when using DEFAULT_ACL on a file (Linux only).
        const IGNORE_EXPECTED_FILE_ERR = 0x10;
    }
}

/// Access Control List native object wrapper.
pub struct Acl {
    /// Native acl.
    acl: acl_t,

    /// Set to true if `acl` was set from the default ACL for a directory
    /// using DEFAULT_ACL option. Used to return entries with the `DEFAULT`
    /// flag set.
    #[cfg(target_os = "linux")]
    default_acl: bool,
}

impl Acl {
    /// Convenience function to construct an `Acl`.
    #[allow(unused_variables)]
    fn new(acl: acl_t, default_acl: bool) -> Acl {
        assert!(!acl.is_null());
        Acl {
            acl,
            #[cfg(target_os = "linux")]
            default_acl,
        }
    }

    /// Read ACL for specified file.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] on failure.
    pub fn read<P: AsRef<Path>>(path: P, options: AclOption) -> io::Result<Acl> {
        let symlink_acl = options.contains(AclOption::SYMLINK_ACL);
        let default_acl = options.contains(AclOption::DEFAULT_ACL);

        let result = xacl_get_file(path.as_ref(), symlink_acl, default_acl);
        match result {
            Ok(acl) => Ok(Acl::new(acl, default_acl)),
            Err(err) => {
                // Trying to access the default ACL of a file on Linux will
                // return an error. We can catch this error and return an empty
                // ACL instead; only if `IGNORE_EXPECTED_FILE_ERR` is set.
                if default_acl
                    && err.kind() == io::ErrorKind::PermissionDenied
                    && options.contains(AclOption::IGNORE_EXPECTED_FILE_ERR)
                {
                    // Return an empty acl (FIXME).
                    Ok(Acl::new(xacl_init(1)?, default_acl))
                } else {
                    Err(path_err(path.as_ref(), &err))
                }
            }
        }
    }

    /// Write ACL for specified file.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] on failure.  
    pub fn write<P: AsRef<Path>>(&self, path: P, options: AclOption) -> io::Result<()> {
        let symlink_acl = options.contains(AclOption::SYMLINK_ACL);
        let default_acl = options.contains(AclOption::DEFAULT_ACL);

        // Don't check ACL if it's an empty, default ACL (FIXME).
        if !(default_acl && self.is_empty()) {
            xacl_check(self.acl).map_err(|err| path_err(path.as_ref(), &err))?;
        }

        if let Err(err) = xacl_set_file(path.as_ref(), self.acl, symlink_acl, default_acl) {
            // Trying to access the default ACL of a file on Linux will
            // return an error. Ignore if `IGNORE_EXPECTED_FILE_ERR` is set.
            if !(default_acl
                && err.kind() == io::ErrorKind::PermissionDenied
                && options.contains(AclOption::IGNORE_EXPECTED_FILE_ERR))
            {
                return Err(path_err(path.as_ref(), &err));
            }
        }

        Ok(())
    }

    /// Construct ACL from slice of [`AclEntry`].
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] on failure.
    pub fn from_entries(entries: &[AclEntry]) -> io::Result<Acl> {
        let new_acl = xacl_init(entries.len())?;

        // Use the smart pointer form of scopeguard; `acl_p` can change value
        // when we create entries in it.
        let mut acl_p = scopeguard::guard(new_acl, |a| {
            xacl_free(a);
        });

        for (i, entry) in entries.iter().enumerate() {
            let entry_p = xacl_create_entry(&mut acl_p)?;
            if let Err(err) = entry.to_raw(entry_p) {
                return fail_custom(&format!("entry {}: {}", i, err));
            }
        }

        Ok(Acl::new(ScopeGuard::into_inner(acl_p), false))
    }

    /// Construct pair of ACL's from slice of [`AclEntry`].
    ///
    /// Separate regular access entries from default entries on Linux.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] on failure.
    #[cfg(target_os = "linux")]
    pub fn from_unified_entries(entries: &[AclEntry]) -> io::Result<(Acl, Acl)> {
        let new_access = xacl_init(entries.len())?;
        let new_default = xacl_init(entries.len())?;

        // Use the smart pointer form of scopeguard; acls can change value when
        // we create entries in them.
        let mut access_p = scopeguard::guard(new_access, |a| {
            xacl_free(a);
        });

        let mut default_p = scopeguard::guard(new_default, |a| {
            xacl_free(a);
        });

        for (i, entry) in entries.iter().enumerate() {
            let entry_p = if entry.flags.contains(Flag::DEFAULT) {
                xacl_create_entry(&mut default_p)?
            } else {
                xacl_create_entry(&mut access_p)?
            };
            if let Err(err) = entry.to_raw(entry_p) {
                return fail_custom(&format!("entry {}: {}", i, err));
            }
        }

        let access_acl = ScopeGuard::into_inner(access_p);
        let default_acl = ScopeGuard::into_inner(default_p);

        Ok((Acl::new(access_acl, false), Acl::new(default_acl, true)))
    }

    /// Return ACL as a vector of [`AclEntry`].
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] on failure.
    pub fn entries(&self) -> io::Result<Vec<AclEntry>> {
        let mut entries = Vec::<AclEntry>::with_capacity(xacl_entry_count(self.acl));

        xacl_foreach(self.acl, |entry_p| {
            let entry = AclEntry::from_raw(entry_p)?;
            entries.push(entry);
            Ok(())
        })?;

        #[cfg(target_os = "linux")]
        if self.default_acl {
            // Set DEFAULT flag on each entry.
            for entry in &mut entries {
                entry.flags |= Flag::DEFAULT;
            }
        }

        Ok(entries)
    }

    /// Construct ACL from platform-dependent textual description.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] on failure.
    pub fn from_platform_text(text: &str) -> io::Result<Acl> {
        let acl = xacl_from_text(text)?;
        Ok(Acl::new(acl, false))
    }

    /// Return platform-dependent textual description.
    #[must_use]
    pub fn to_platform_text(&self) -> String {
        xacl_to_text(self.acl)
    }

    /// Return true if ACL is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        xacl_entry_count(self.acl) == 0
    }
}

impl Drop for Acl {
    fn drop(&mut self) {
        xacl_free(self.acl);
    }
}
