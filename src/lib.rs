// src/lib.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

// ===========================================================================
// Externs
// ===========================================================================


// Stdlib externs

// Third-party externs
extern crate bytes;
extern crate futures;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

extern crate rmp;
extern crate rmp_serde as rmps;
extern crate rmpv;
extern crate serde;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_service;

// Local externs


// ===========================================================================
// Modules
// ===========================================================================


pub mod error;
pub mod network;
pub mod storage;
pub mod util;


// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports

// Third-party imports

// Local imports


// ===========================================================================
//
// ===========================================================================
