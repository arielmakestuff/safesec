// storage.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports
use std::path::Path;

// Third-party imports

// Local imports


// ===========================================================================
// Types
// ===========================================================================


#[derive(Debug)]
pub enum KeyFileError {
    Key(Vec<u8>),
    Other
}


type KeyFileResult<V> = Result<V, KeyFileError>;


// ===========================================================================
// Modules
// ===========================================================================


pub mod lmdb;


// ===========================================================================
// KeyFile Traits
// ===========================================================================


pub trait KeyFileBuilder {
    fn new(name: &str, envpath: Option<&Path>) -> Self;
}


pub trait KeyFileStore {

    fn get(&self, k: &Vec<u8>) -> KeyFileResult<Vec<u8>>;
    fn set(&self, k: &Vec<u8>, file: &Vec<u8>) -> KeyFileResult<()>;
    // fn delete(&self, k: &[u8]) -> Result<(), String>;
}


// ===========================================================================
//
// ===========================================================================