//! API Tests for exacl module.

use ctor::ctor;
use exacl::{getfacl, setfacl, Acl, AclEntry, AclOption, Flag, Perm};
use log::debug;
use std::io;

#[ctor]
fn init() {
    env_logger::init();
}

fn log_acl(acl: &[AclEntry]) {
    for entry in acl {
        if entry.allow {
            debug!(
                "{:?}:{} allow {:?} {:?}",
                entry.kind, entry.name, entry.perms, entry.flags
            );
        } else {
            debug!(
                "{:?}:{} deny {:?} {:?}",
                entry.kind, entry.name, entry.perms, entry.flags
            );
        }
    }
}

#[test]
fn test_read_acl() -> io::Result<()> {
    let file = tempfile::NamedTempFile::new()?;
    let acl = Acl::read(file.as_ref(), AclOption::empty())?;
    let entries = acl.entries()?;

    #[cfg(target_os = "macos")]
    assert_eq!(entries.len(), 0);

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    assert_eq!(entries.len(), 3);

    log_acl(&entries);

    Ok(())
}

#[test]
#[cfg(target_os = "macos")]
fn test_write_acl_macos() -> io::Result<()> {
    let mut entries = Vec::<AclEntry>::new();
    let rwx = Perm::READ | Perm::WRITE | Perm::EXECUTE;

    entries.push(AclEntry::allow_group("_spotlight", rwx, None));
    entries.push(AclEntry::allow_user("11501", rwx, None));
    entries.push(AclEntry::allow_user("11502", rwx, None));
    entries.push(AclEntry::allow_user("11503", rwx, None));
    entries.push(AclEntry::deny_group(
        "11504",
        rwx,
        Flag::FILE_INHERIT | Flag::DIRECTORY_INHERIT,
    ));

    log_acl(&entries);

    let file = tempfile::NamedTempFile::new()?;
    let acl = Acl::from_entries(&entries)?;
    assert!(!acl.is_empty());
    acl.write(file.as_ref(), AclOption::empty())?;

    // Even though the last entry is a group, the `acl_to_text` representation
    // displays it as `user`.
    assert_eq!(
        acl.to_platform_text()?,
        r#"!#acl 1
group:ABCDEFAB-CDEF-ABCD-EFAB-CDEF00000059:_spotlight:89:allow:read,write,execute
user:FFFFEEEE-DDDD-CCCC-BBBB-AAAA00002CED:::allow:read,write,execute
user:FFFFEEEE-DDDD-CCCC-BBBB-AAAA00002CEE:::allow:read,write,execute
user:FFFFEEEE-DDDD-CCCC-BBBB-AAAA00002CEF:::allow:read,write,execute
user:AAAABBBB-CCCC-DDDD-EEEE-FFFF00002CF0:::deny,file_inherit,directory_inherit:read,write,execute
"#
    );

    let acl2 = Acl::read(file.as_ref(), AclOption::empty())?;
    let entries2 = acl2.entries()?;

    assert_eq!(entries2, entries);

    Ok(())
}

#[test]
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn test_write_acl_linux() -> io::Result<()> {
    let mut entries = Vec::<AclEntry>::new();
    let rwx = Perm::READ | Perm::WRITE | Perm::EXECUTE;

    entries.push(AclEntry::allow_group("bin", rwx, None));
    entries.push(AclEntry::allow_user("11501", rwx, None));
    entries.push(AclEntry::allow_user("11502", rwx, None));
    entries.push(AclEntry::allow_user("11503", rwx, None));
    entries.push(AclEntry::allow_user("", rwx, None));
    entries.push(AclEntry::allow_group("", rwx, None));
    entries.push(AclEntry::allow_other(rwx, None));
    // We do not add a mask entry. One will be automatically added.

    log_acl(&entries);

    let file = tempfile::NamedTempFile::new()?;
    let acl = Acl::from_entries(&entries)?;
    acl.write(file.as_ref(), AclOption::empty())?;

    assert_eq!(
        acl.to_platform_text()?,
        r#"user::rwx
user:11501:rwx
user:11502:rwx
user:11503:rwx
group::rwx
group:bin:rwx
mask::rwx
other::rwx
"#
    );

    let acl2 = Acl::read(file.as_ref(), AclOption::empty())?;
    let mut entries2 = acl2.entries()?;

    // Before doing the comparison, add the mask entry.
    entries.push(AclEntry::allow_mask(rwx, None));

    entries.sort();
    entries2.sort();
    assert_eq!(entries2, entries);

    Ok(())
}
#[test]
#[cfg(target_os = "macos")]
fn test_write_acl_big() -> io::Result<()> {
    let mut entries = Vec::<AclEntry>::new();
    let rwx = Perm::READ | Perm::WRITE | Perm::EXECUTE;

    for _ in 0..128 {
        entries.push(AclEntry::allow_user("11501", rwx, None));
    }

    let file = tempfile::NamedTempFile::new()?;
    let acl = Acl::from_entries(&entries)?;
    acl.write(file.as_ref(), AclOption::empty())?;

    let acl2 = Acl::read(file.as_ref(), AclOption::empty())?;
    let entries2 = acl2.entries()?;

    assert_eq!(entries2, entries);

    Ok(())
}

#[test]
#[cfg(target_os = "macos")]
fn test_write_acl_too_big() {
    let mut entries = Vec::<AclEntry>::new();
    let rwx = Perm::READ | Perm::WRITE | Perm::EXECUTE;

    for _ in 0..129 {
        entries.push(AclEntry::allow_user("11501", rwx, None));
    }

    let err = Acl::from_entries(&entries).err().unwrap();
    assert_eq!(err.to_string(), "Too many ACL entries");
}

#[test]
#[cfg(target_os = "macos")]
fn test_from_platform_text() {
    let text = r#"!#acl 1
user:FFFFEEEE-DDDD-CCCC-BBBB-AAAA00002CED:::allow:read,write,execute
user:FFFFEEEE-DDDD-CCCC-BBBB-AAAA00002CEE:::allow:read,write,execute
user:FFFFEEEE-DDDD-CCCC-BBBB-AAAA00002CEF:::allow:read,write,execute
user:AAAABBBB-CCCC-DDDD-EEEE-FFFF00002CF0:::deny,file_inherit,directory_inherit:read,write,execute
"#;

    let acl = Acl::from_platform_text(text).unwrap();
    assert_eq!(acl.to_platform_text().unwrap(), text);

    let input = r#"!#acl 1
group::_spotlight::allow:read,write,execute
"#;
    let output = r#"!#acl 1
group:ABCDEFAB-CDEF-ABCD-EFAB-CDEF00000059:_spotlight:89:allow:read,write,execute
"#;
    let acl = Acl::from_platform_text(input).unwrap();
    assert_eq!(acl.to_platform_text().unwrap(), output);

    // Giving bad input can result in bad output.
    let bad_input = r#"!#acl 1
group:_spotlight:::allow:read,write,execute
"#;
    let bad_output = r#"!#acl 1
user:00000000-0000-0000-0000-000000000000:::allow:read,write,execute
"#;
    let acl = Acl::from_platform_text(bad_input).unwrap();
    assert_eq!(acl.to_platform_text().unwrap(), bad_output);

    log_acl(&acl.entries().unwrap());
}

#[test]
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn test_from_platform_text() {
    let text = r#"user::rwx
user:11501:rwx
user:11502:rwx
user:11503:rwx
group::rwx
group:bin:rwx
mask::rwx
other::rwx
"#;

    let acl = Acl::from_platform_text(text).unwrap();
    assert_eq!(acl.to_platform_text().unwrap(), text);
}

#[test]
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn test_read_default_acl() -> io::Result<()> {
    let dir = tempfile::tempdir()?;
    let default_acl = Acl::read(dir.as_ref(), AclOption::DEFAULT_ACL)?;
    assert!(default_acl.is_empty());

    Ok(())
}

#[test]
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn test_write_default_acl() -> io::Result<()> {
    let mut entries = Vec::<AclEntry>::new();
    let rwx = Perm::READ | Perm::WRITE | Perm::EXECUTE;

    entries.push(AclEntry::allow_user("", rwx, None));
    entries.push(AclEntry::allow_group("", rwx, None));
    entries.push(AclEntry::allow_other(rwx, None));
    entries.push(AclEntry::allow_group("bin", rwx, None));
    entries.push(AclEntry::allow_mask(rwx, None));

    let dir = tempfile::tempdir()?;
    let path = dir.as_ref();
    let acl = Acl::from_entries(&entries)?;
    acl.write(path, AclOption::DEFAULT_ACL)?;

    let acl2 = Acl::read(path, AclOption::empty())?;
    assert_ne!(acl.to_platform_text()?, acl2.to_platform_text()?);

    let default_acl = Acl::read(path, AclOption::DEFAULT_ACL)?;
    assert_eq!(default_acl.to_platform_text()?, acl.to_platform_text()?);

    let default_entries = default_acl.entries()?;
    for entry in &default_entries {
        assert_eq!(entry.flags, Flag::DEFAULT);
    }

    // Test deleting a default ACL by passing an empty acl.
    debug!("Test deleting a default ACL");
    let empty_acl = Acl::from_entries(&[])?;
    empty_acl.write(path, AclOption::DEFAULT_ACL)?;
    assert!(Acl::read(path, AclOption::DEFAULT_ACL)?.is_empty());

    Ok(())
}

#[test]
fn test_empty_acl() -> io::Result<()> {
    let acl = Acl::from_entries(&[])?;
    assert!(acl.is_empty());
    Ok(())
}

#[test]
fn test_getfacl_file() -> io::Result<()> {
    let file = tempfile::NamedTempFile::new()?;
    let entries = getfacl(&file, None)?;

    #[cfg(target_os = "macos")]
    assert_eq!(entries.len(), 0);

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    assert_eq!(entries.len(), 3);

    log_acl(&entries);

    // Test default ACL on macOS (should fail).
    #[cfg(target_os = "macos")]
    {
        let result = getfacl(&file, AclOption::DEFAULT_ACL);
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("macOS does not support default ACL"));
    }

    // Test default ACL (should be error; files don't have default ACL).
    #[cfg(target_os = "linux")]
    {
        let result = getfacl(&file, AclOption::DEFAULT_ACL);
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Permission denied"));
    }

    // Test default ACL (should be error; files don't have default ACL).
    #[cfg(target_os = "freebsd")]
    {
        let result = getfacl(&file, AclOption::DEFAULT_ACL);
        assert!(result.unwrap_err().to_string().contains("Invalid argument"));
    }

    Ok(())
}

#[test]
fn test_from_entries() {
    // 0 entries should result in empty acl.
    let acl = Acl::from_entries(&[]).unwrap();
    assert!(acl.is_empty());

    // Test named user on MacOS.
    #[cfg(target_os = "macos")]
    {
        let entries = vec![AclEntry::allow_user("500", Perm::EXECUTE, None)];
        let acl = Acl::from_entries(&entries).unwrap();
        assert_eq!(
            acl.to_platform_text().unwrap(),
            "!#acl 1\nuser:FFFFEEEE-DDDD-CCCC-BBBB-AAAA000001F4:::allow:execute\n"
        );
    }

    // Test named user on Linux. It should add correct mask.
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        let mut entries = vec![
            AclEntry::allow_group("", Perm::READ, None),
            AclEntry::allow_other(Perm::READ, None),
            AclEntry::allow_user("500", Perm::EXECUTE, None),
        ];

        let err = Acl::from_entries(&entries).err().unwrap();
        assert_eq!(err.to_string(), "missing required entry \"user\"");

        entries.push(AclEntry::allow_user("", Perm::READ, None));
        let acl = Acl::from_entries(&entries).unwrap();

        #[cfg(target_os = "linux")]
        let expected = "user::r--\nuser:500:--x\ngroup::r--\nmask::r-x\nother::r--\n";
        #[cfg(target_os = "freebsd")]
        let expected = "group::r--\nother::r--\nuser:500:--x\nuser::r--\nmask::r-x\n";
        assert_eq!(acl.to_platform_text().unwrap(), expected);

        entries.push(AclEntry::allow_group("", Perm::WRITE, None));
        let err = Acl::from_entries(&entries).err().unwrap();
        assert_eq!(err.to_string(), "entry 4: duplicate entry for \"group\"");
    }
}

#[test]
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn test_from_unified_entries() {
    // 0 entries should result in empty acls.
    let (a, d) = Acl::from_unified_entries(&[]).unwrap();
    assert!(a.is_empty());
    assert!(d.is_empty());

    let mut entries = vec![
        AclEntry::allow_user("500", Perm::EXECUTE, None),
        AclEntry::allow_user("501", Perm::EXECUTE, Flag::DEFAULT),
    ];

    // Missing required entries.
    let err = Acl::from_unified_entries(&entries).err().unwrap();
    assert_eq!(err.to_string(), "missing required entry \"user\"");

    entries.push(AclEntry::allow_group("", Perm::WRITE, None));
    entries.push(AclEntry::allow_user("", Perm::READ, None));
    entries.push(AclEntry::allow_other(Perm::empty(), None));

    // Missing required default entries.
    let err = Acl::from_unified_entries(&entries).err().unwrap();
    assert_eq!(err.to_string(), "missing required default entry \"user\"");

    entries.push(AclEntry::allow_group("", Perm::WRITE, Flag::DEFAULT));
    entries.push(AclEntry::allow_user("", Perm::READ, Flag::DEFAULT));
    entries.push(AclEntry::allow_other(Perm::empty(), Flag::DEFAULT));

    let (a, d) = Acl::from_unified_entries(&entries).unwrap();

    #[cfg(target_os = "linux")]
    let expected1 = "user::r--\nuser:500:--x\ngroup::-w-\nmask::-wx\nother::---\n";
    #[cfg(target_os = "freebsd")]
    let expected1 = "user:500:--x\ngroup::-w-\nuser::r--\nother::---\nmask::-wx\n";
    assert_eq!(a.to_platform_text().unwrap(), expected1);

    #[cfg(target_os = "linux")]
    let expected2 = "user::r--\nuser:501:--x\ngroup::-w-\nmask::-wx\nother::---\n";
    #[cfg(target_os = "freebsd")]
    let expected2 = "user:501:--x\ngroup::-w-\nuser::r--\nother::---\nmask::-wx\n";
    assert_eq!(d.to_platform_text().unwrap(), expected2);

    entries.push(AclEntry::allow_group("", Perm::WRITE, Flag::DEFAULT));

    let err = Acl::from_unified_entries(&entries).err().unwrap();
    assert_eq!(
        err.to_string(),
        "entry 8: duplicate default entry for \"group\""
    );
}

#[test]
#[cfg(target_os = "linux")]
fn test_too_many_entries() -> io::Result<()> {
    use exacl::setfacl;

    // This test depends on the type of file system. With ext* systems, we
    // expect ACL's with 508 entries to fail.
    let mut entries = vec![
        AclEntry::allow_user("", Perm::READ, None),
        AclEntry::allow_group("", Perm::READ, None),
        AclEntry::allow_other(Perm::empty(), None),
        AclEntry::allow_mask(Perm::READ, None),
    ];

    for i in 500..1003 {
        entries.push(AclEntry::allow_user(&i.to_string(), Perm::READ, None));
    }

    let files = [tempfile::NamedTempFile::new()?];

    // 507 entries are okay.
    setfacl(&files, &entries, None)?;
    debug!("{} entries is okay", entries.len());

    // Add 508th entry.
    entries.push(AclEntry::allow_user("1500", Perm::READ, None));

    // 508th entry is one too many.
    let err = setfacl(&files, &entries, None).unwrap_err();
    assert!(err.to_string().contains("No space left on device"));

    Ok(())
}

#[test]
#[cfg(target_os = "macos")]
fn test_set_acl_flags() -> io::Result<()> {
    let file = tempfile::NamedTempFile::new()?;
    let entries = vec![AclEntry::allow_user("600", Perm::READ, Flag::empty())];

    let mut acl = Acl::from_entries(&entries)?;
    assert_eq!(acl.flags()?, Flag::empty());

    acl.set_flags(Flag::NO_INHERIT)?;
    assert_eq!(acl.flags()?, Flag::NO_INHERIT);

    // Setting the flag in memory has no effect on the file.
    let acl2 = Acl::read(file.as_ref(), AclOption::empty())?;
    assert_eq!(acl2.flags()?, Flag::empty());

    // Writing the ACL will change the file.
    acl.write(file.as_ref(), AclOption::empty())?;

    // The NO_INHERIT flag only seems to persist if the ACL is not empty.
    let acl3 = Acl::read(file.as_ref(), AclOption::empty())?;
    assert_eq!(acl3.flags()?, Flag::NO_INHERIT);

    Ok(())
}

#[test]
fn test_reader_writer() -> io::Result<()> {
    let input = r#"
    u:aaa:rwx#comment
    g:bbb:rwx
    u:ccc:rx
    "#;

    let entries = exacl::from_str(input)?;
    let actual = exacl::to_string(&entries)?;

    let expected = r#"allow::user:aaa:read,write,execute
allow::group:bbb:read,write,execute
allow::user:ccc:read,execute
"#;
    assert_eq!(expected, actual);

    Ok(())
}

#[test]
#[cfg(target_os = "linux")]
fn test_exclusive_acloptions() {
    let path = "/tmp";

    let err1 = getfacl(&path, AclOption::ACCESS_ACL | AclOption::DEFAULT_ACL).unwrap_err();
    assert_eq!(
        err1.to_string(),
        "ACCESS_ACL and DEFAULT_ACL are mutually exclusive options"
    );

    let err2 = setfacl(&[path], &[], AclOption::ACCESS_ACL | AclOption::DEFAULT_ACL).unwrap_err();
    assert_eq!(
        err2.to_string(),
        "ACCESS_ACL and DEFAULT_ACL are mutually exclusive options"
    );
}
