// test_lmdb.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

// ===========================================================================
// Externs
// ===========================================================================


extern crate chrono;
extern crate safesec;
extern crate tempdir;


// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports
use std::fs;

// Third-party imports
use chrono::prelude::*;
use tempdir::TempDir;

// Local imports
use safesec::storage::*;
use safesec::storage::lmdb::*;


// ===========================================================================
// Helpers
// ===========================================================================


fn mktempdir() -> TempDir {
    //Generate unique temp name
    let dt = UTC::now();
    let suffix = dt.format("%Y%m%d%H%M%S%.9f");
    let name = format!("safesec_test_{}", suffix.to_string());
    let tmpdir = TempDir::new(&name).unwrap();
    let dbpath = tmpdir.path().join("sec.db");
    fs::create_dir(&dbpath).unwrap();
    tmpdir
}


// ===========================================================================
// Tests
// ===========================================================================


#[test]
fn create_db() {
    // Create temp directory
    let tmpdir = mktempdir();
    let dbpath = tmpdir.path().join("sec.db");

    // Create keyfile store
    let kf = KeyFile::new("temp", Some(dbpath.as_path()));

    // Test
    assert_eq!(kf.dbinit.path, dbpath);
    assert!(dbpath.exists());
}


#[test]
fn get_set_value() {
    // Create temp directory
    let tmpdir = mktempdir();
    let dbpath = tmpdir.path().join("sec.db");

    // Create keyfile store
    let kf = KeyFile::new("temp", Some(dbpath.as_path()));

    // Set value
    let key = 42.to_string().into_bytes();
    let value = "The Answer to Life, the Universe, and Everything";
    let expected = String::from(value).into_bytes();
    kf.set(&key, &expected).unwrap();

    // Get value
    let v = kf.get(&key).unwrap();

    // Test
    assert_eq!(v, expected);
}


#[test]
#[should_panic(expected = "Key doesn't exist: 42")]
fn no_key() {
    // Create temp directory
    let tmpdir = mktempdir();
    let dbpath = tmpdir.path().join("sec.db");

    // Create keyfile store
    let kf = KeyFile::new("temp", Some(dbpath.as_path()));

    // Get non-existent value
    let key = 42.to_string().into_bytes();
    match kf.get(&key) {
        Err(KeyFileError::Key(e)) => {
            let k = String::from_utf8(e).unwrap();
            let errmsg = format!("Key doesn't exist: {}", k);
            panic!(errmsg)
        },
        _ => panic!("Expected error did not occur")
    }
}


#[test]
fn set_overwrites_value() {
    // Create temp directory
    let tmpdir = mktempdir();
    let dbpath = tmpdir.path().join("sec.db");

    // Create keyfile store
    let kf = KeyFile::new("temp", Some(dbpath.as_path()));

    // Set value
    let key = 42.to_string().into_bytes();
    for value in 0..5 {
        let v = value.to_string().into_bytes();
        kf.set(&key, &v).unwrap();
    }

    // Get value
    let v = kf.get(&key).unwrap();

    // Test
    assert_eq!(v, 4.to_string().into_bytes());

    let value = "The Answer to Life, the Universe, and Everything";
    let expected = String::from(value).into_bytes();
    kf.set(&key, &expected).unwrap();

    // Get value
    let v = kf.get(&key).unwrap();

    // Test overwrite
    assert_eq!(v, expected);
}


#[test]
fn multiple_keys() {
    // Create temp directory
    let tmpdir = mktempdir();
    let dbpath = tmpdir.path().join("sec.db");

    // Create keyfile store
    let kf = KeyFile::new("temp", Some(dbpath.as_path()));

    // Set values
    for (i, num) in (0..5).rev().enumerate() {
        let k = i.to_string().into_bytes();
        let v = num.to_string().into_bytes();
        kf.set(&k, &v).unwrap();
    }

    // Check values
    for (i, num) in (0..5).rev().enumerate() {
        let k = i.to_string().into_bytes();
        let expected = num.to_string().into_bytes();
        let v = kf.get(&k).unwrap();
        assert_eq!(v, expected);
    }
}


// ===========================================================================
//
// ===========================================================================
