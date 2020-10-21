#!/bin/sh

# Basic test suite for exacl tool.
#
# To run:  `shunit2 testsuite_basic.sh`

alias exacl=../target/debug/exacl

DIR="test_dir-mac_os-test_dir"
FILE1="$DIR/file1"
DIR1="$DIR/dir1"
LINK1="$DIR/link1"

ME=`id -un`
ME_NUM=`id -u`
MY_GROUP=`id -gn`
MY_GROUP_NUM=`id -g`

# Return true if file is readable.
isReadable() {
    cat "$1" > /dev/null 2>&1 
    return $?
}

# Return true if file is writable (tries to overwrite file).
isWritable() {
    echo "x" 2> /dev/null > "$1" 
    return $?
}

# Return true if directory is readable.
isReadableDir() {
    ls "$1" > /dev/null 2>&1
    return $?
}

# Return true if link is readable.
isReadableLink() {
    readlink "$1" > /dev/null 2>&1
    return $?
}

oneTimeSetUp() {
    # Create an empty temporary directory.
    if [ -d "$DIR" ]; then
        rm -rf "$DIR"
    fi

    mkdir "$DIR"

    # Create empty file, dir, and link.
    umask 077
    touch "$FILE1"
    mkdir "$DIR1"
    ln -s link1_to_nowhere "$LINK1"
}

# Put quotes back on JSON text.
quotifyJson() { 
    echo "$1" | sed -E -e 's/([a-z0-9_]+)/"\1"/g' -e 's/:"false"/:false/g' -e 's/:"true"/:true/g'
}

oneTimeTearDown() {
    # Delete our temporary directory.
    rm -rf "$DIR"
}

testReadAclFromMissingFile() {
    msg=`exacl $DIR/non_existant 2>&1`
    assertEquals 1 $?
    assertEquals \
        "File \"$DIR/non_existant\": No such file or directory (os error 2)" \
        "$msg"
}

testReadAclForFile1() {
    msg=`exacl $FILE1`
    assertEquals 0 $?
    assertEquals "[]" "$msg"

    isReadable "$FILE1" && isWritable "$FILE1"
    assertEquals 0 $?
    
    # Add ACL entry for current user to "deny read".
    chmod +a "$ME deny read" "$FILE1"

    msg=`exacl $FILE1`
    assertEquals 0 $?
    assertEquals \
        "[{kind:user,name:$ME,perms:[read],flags:[],allow:false}]" \
        "${msg//\"}"

    ! isReadable "$FILE1" && isWritable "$FILE1"
    assertEquals 0 $?

    # Remove user write perm.
    chmod u-w "$FILE1"
    ! isReadable "$FILE1" && ! isWritable "$FILE1"
    assertEquals 0 $?

    # Add ACL entry for current group to "allow write".
    chmod +a "$MY_GROUP allow write" "$FILE1"

    msg=`exacl $FILE1`
    assertEquals 0 $?
    assertEquals \
        "[{kind:user,name:$ME,perms:[read],flags:[],allow:false},{kind:group,name:$MY_GROUP,perms:[write],flags:[],allow:true}]" \
        "${msg//\"}"

    ! isReadable "$FILE1" && isWritable "$FILE1"
    assertEquals 0 $?

    # Re-add user write perm that we removed above.
    chmod u+w "$FILE1"
}

testReadAclForDir1() {
    msg=`exacl $DIR1`
    assertEquals 0 $?
    assertEquals "[]" "$msg"

    # Add ACL entry for current user to "deny read" with inheritance flags.
    chmod +a "$ME deny read,file_inherit,directory_inherit,only_inherit" "$DIR1"

    msg=`exacl $DIR1`
    assertEquals 0 $?
    assertEquals \
        "[{kind:user,name:$ME,perms:[read],flags:[file_inherit,directory_inherit,only_inherit],allow:false}]" \
        "${msg//\"}"

    isReadableDir "$DIR1"
    assertEquals 0 $?

    # Create subfile in DIR1.
    subfile="$DIR1/subfile"
    touch "$subfile"

    ! isReadable "$subfile" && isWritable "$subfile"
    assertEquals 0 $?

    msg=`exacl $subfile`
    assertEquals 0 $?
    assertEquals \
        "[{kind:user,name:$ME,perms:[read],flags:[inherited],allow:false}]" \
        "${msg//\"}"

    # Create subdirectory in DIR1.
    subdir="$DIR1/subdir"
    mkdir "$subdir"

    msg=`exacl $subdir`
    assertEquals 0 $?
    assertEquals \
        "[{kind:user,name:$ME,perms:[read],flags:[inherited,file_inherit,directory_inherit],allow:false}]" \
        "${msg//\"}"

    # Clear directory ACL's so we can delete them.
    chmod -a# 0 "$subdir"
    chmod -a# 0 "$DIR1"

    rmdir "$subdir"
    rm "$subfile"
}

testReadAclForLink1() {
    # Test symlink with no ACL.
    msg=`exacl $LINK1`
    assertEquals 0 $?
    assertEquals "[]" "$msg"

    # Add ACL entry for current user to "deny read".
    chmod -h +a "$ME deny read" "$LINK1"
    assertEquals 0 $?

    ! isReadableLink "$LINK1"
    assertEquals 0 $?

    msg=`exacl $LINK1`
    assertEquals 0 $?
    assertEquals \
        "[{kind:user,name:$ME,perms:[read],flags:[],allow:false}]" \
        "${msg//\"}"

    # It appears that you can't further modify the ACL of a symbolic link if
    # you don't have 'read' access to the link anymore.
    msg=`chmod -h -a# 0 "$LINK1" 2>&1`
    assertEquals 1 $?
    assertEquals \
        "chmod: No ACL present 'test_dir-mac_os-test_dir/link1'
chmod: Failed to set ACL on file 'test_dir-mac_os-test_dir/link1': Permission denied" \
        "$msg"

    # Recreate the symlink here.
    ln -fs link1_to_nowhere "$LINK1"
}

testWriteAclToMissingFile() {
    input="[]"
    msg=`echo "$input" | exacl --set $DIR/non_existant 2>&1`
    assertEquals 1 $?
    assertEquals \
        "File \"$DIR/non_existant\": No such file or directory (os error 2)" \
        "$msg"
}

testWriteAclToFile1() {
    # Set ACL to empty.
    input="[]"
    msg=`echo "$input" | exacl --set $FILE1 2>&1`
    assertEquals 0 $?
    assertEquals "" "$msg"

    # Verify it's empty.
    msg=`exacl $FILE1`
    assertEquals 0 $?
    assertEquals "[]" "$msg"

    isReadable "$FILE1" && isWritable "$FILE1"
    assertEquals 0 $?

    # Set ACL for current user to "deny read".
    input=`quotifyJson "[{kind:user,name:$ME,perms:[read],flags:[],allow:false}]"`
    msg=`echo "$input" | exacl --set $FILE1 2>&1`
    assertEquals 0 $?
    assertEquals "" "$msg"

    ! isReadable "$FILE1" && isWritable "$FILE1"
    assertEquals 0 $?

    # Check ACL using ls.
    msg=`ls -le $FILE1 | grep -E '^ \d+: '`
    assertEquals \
        " 0: user:$ME deny read" \
        "$msg"

    # Check ACL again using exacl.
    msg=`exacl $FILE1`
    assertEquals 0 $?
    assertEquals \
        "[{kind:user,name:$ME,perms:[read],flags:[],allow:false}]" \
        "${msg//\"}"
}

testWriteAclToDir1() {
    # Set ACL to empty.
    input="[]"
    msg=`echo "$input" | exacl --set $DIR1 2>&1`
    assertEquals 0 $?
    assertEquals "" "$msg"

    # Verify it's empty.
    msg=`exacl $DIR1`
    assertEquals 0 $?
    assertEquals "[]" "$msg"

    isReadableDir "$DIR1"
    assertEquals 0 $?

    # Set ACL for current user to "deny read".
    input=`quotifyJson "[{kind:user,name:$ME,perms:[read],flags:[],allow:false}]"`
    msg=`echo "$input" | exacl --set $DIR1 2>&1`
    assertEquals 0 $?
    assertEquals "" "$msg"

    ! isReadable "$DIR1"
    assertEquals 0 $?

    # Read ACL back.
    msg=`exacl $DIR1`
    assertEquals 0 $?
    assertEquals "$input" "$msg"
}

testWriteAclToLink1() {
    # Set ACL to empty.
    input="[]"
    msg=`echo "$input" | exacl --set $LINK1 2>&1`
    assertEquals 0 $?
    assertEquals \
        "" \
        "$msg"

    isReadableLink "$LINK1"
    assertEquals 0 $?

    # Set ACL for current user to "deny read".
    input=`quotifyJson "[{kind:user,name:$ME,perms:[read],flags:[],allow:false}]"`
    msg=`echo "$input" | exacl --set $LINK1 2>&1`
    assertEquals 0 $?
    assertEquals \
        "" \
        "$msg"

    ! isReadableLink "$LINK1"
    assertEquals 0 $?

    # Check ACL using ls.
    msg=`ls -le $LINK1 2> /dev/null | grep -E '^ \d+: '`
    assertEquals \
        " 0: user:$ME deny read" \
        "$msg"

    # Check ACL again using exacl.
    msg=`exacl $LINK1`
    assertEquals 0 $?
    assertEquals \
        "[{kind:user,name:$ME,perms:[read],flags:[],allow:false}]" \
        "${msg//\"}"
}

testWriteAllFilePerms() {
    all="read,write,execute,delete,append,delete_child,readattr,writeattr,readextattr,writeextattr,readsecurity,writesecurity,chown,sync"
    input=`quotifyJson "[{kind:user,name:$ME,perms:[$all],flags:[],allow:true}]"`
    msg=`echo "$input" | exacl --set $FILE1`
    assertEquals 0 $?
    assertEquals "" "$msg"

    msg=`exacl $FILE1`
    assertEquals 0 $?
    assertEquals \
        "[{kind:user,name:$ME,perms:[$all],flags:[],allow:true}]" \
        "${msg//\"}"

    # ls output omits delete_child and sync.
    ls_perms="read,write,execute,delete,append,readattr,writeattr,readextattr,writeextattr,readsecurity,writesecurity,chown"
    msg=`ls -le $FILE1 | grep -E '^ \d+: '`
    assertEquals \
        " 0: user:$ME allow $ls_perms" \
        "$msg"
}

testWriteAllFileFlags() {
    entry_flags="inherited,file_inherit,directory_inherit,limit_inherit,only_inherit"
    all="defer_inherit,no_inherit,$entry_flags"
    input=`quotifyJson "[{kind:user,name:$ME,perms:[read],flags:[$all],allow:true}]"`
    msg=`echo "$input" | exacl --set $FILE1`
    assertEquals 0 $?
    assertEquals "" "$msg"

    # N.B. "defer_inherit" flag is not returned.
    msg=`exacl $FILE1`
    assertEquals 0 $?
    assertEquals \
        "[{kind:user,name:$ME,perms:[read],flags:[$entry_flags,no_inherit],allow:true}]" \
        "${msg//\"}"

    # ls output only shows inherited and limit_inherit.
    ls_perms="read,limit_inherit"
    msg=`ls -le $FILE1 | grep -E '^ \d+: '`
    assertEquals \
        " 0: user:$ME inherited allow $ls_perms" \
        "$msg"
}

testWriteAllDirPerms() {
    all="read,write,execute,delete,append,delete_child,readattr,writeattr,readextattr,writeextattr,readsecurity,writesecurity,chown,sync"
    input=`quotifyJson "[{kind:user,name:$ME,perms:[$all],flags:[],allow:true}]"`
    msg=`echo "$input" | exacl --set $DIR1`
    assertEquals 0 $?
    assertEquals "" "$msg"

    msg=`exacl $DIR1`
    assertEquals 0 $?
    assertEquals \
        "[{kind:user,name:$ME,perms:[$all],flags:[],allow:true}]" \
        "${msg//\"}"
}

testWriteAllDirFlags() {
    entry_flags="inherited,file_inherit,directory_inherit,limit_inherit,only_inherit"
    all="defer_inherit,no_inherit,$entry_flags"
    input=`quotifyJson "[{kind:user,name:$ME,perms:[read],flags:[$all],allow:true}]"`
    msg=`echo "$input" | exacl --set $DIR1`
    assertEquals 0 $?
    assertEquals "" "$msg"

    # N.B. "defer_inherit" flag is not returned.
    msg=`exacl $DIR1`
    assertEquals 0 $?
    assertEquals \
        "[{kind:user,name:$ME,perms:[read],flags:[$entry_flags,no_inherit],allow:true}]" \
        "${msg//\"}"
}


testWriteAclNumericUID() {
    # Set ACL for current user to "deny read".
    input=`quotifyJson "[{kind:user,name:$ME_NUM,perms:[read],flags:[],allow:false}]"`
    msg=`echo "$input" | exacl --set $FILE1 2>&1`
    assertEquals 0 $?
    assertEquals "" "$msg"

    ! isReadable "$FILE1" && isWritable "$FILE1"
    assertEquals 0 $?

    # Check ACL again using exacl.
    msg=`exacl $FILE1`
    assertEquals 0 $?
    assertEquals \
        "[{kind:user,name:$ME,perms:[read],flags:[],allow:false}]" \
        "${msg//\"}"
}

testWriteAclNumericGID() {
    # Set ACL for current group to "deny read".
    input=`quotifyJson "[{kind:group,name:$MY_GROUP_NUM,perms:[read],flags:[],allow:false}]"`
    msg=`echo "$input" | exacl --set $FILE1 2>&1`
    assertEquals 0 $?
    assertEquals "" "$msg"

    ! isReadable "$FILE1" && isWritable "$FILE1"
    assertEquals 0 $?

    # Check ACL again using exacl.
    msg=`exacl $FILE1`
    assertEquals 0 $?
    assertEquals \
        "[{kind:group,name:$MY_GROUP,perms:[read],flags:[],allow:false}]" \
        "${msg//\"}"
}
