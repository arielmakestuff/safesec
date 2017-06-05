// src/error.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

// ===========================================================================
// Externs
// ===========================================================================


// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports
use std::error;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;

// Third-party imports

// Local imports


// ===========================================================================
// Errors
// ===========================================================================


pub trait GenericError<E: Debug + Display> {
    fn new(err: E, msg: &str) -> Self;
    fn _defaultfmt(&self, f: &mut Formatter) -> fmt::Result;
}


pub struct Error<E: Debug + Display> {
    err: E,
    msg: String,
    parent: Option<Box<Error<E>>>,
}


impl<E: Debug + Display> Error<E> {
    pub fn parent(mut self, val: Error<E>) -> Self {
        self.parent = Some(Box::new(val));
        self
    }
}


impl<E: Debug + Display> GenericError<E> for Error<E> {

    fn new(err: E, msg: &str) -> Self {
        Self {
            err: err,
            msg: String::from(msg),
            parent: None,
        }
    }

    fn _defaultfmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Error({}) => {}", self.err, self.msg)
    }
}


impl<E: Debug + Display> Debug for Error<E> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self._defaultfmt(f)
    }
}


impl<E: Debug + Display> Display for Error<E> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self._defaultfmt(f)
    }
}


impl<E: Debug + Display> error::Error for Error<E> {
    fn description(&self) -> &str {
        &self.msg
    }

    fn cause(&self) -> Option<&error::Error> {
        match self.parent {
            None => None,
            Some(ref b) => Some(b.deref())
        }
    }
}


// ===========================================================================
// Custom Error
// ===========================================================================


#[derive(Debug)]
pub enum ContextErrorType {
    EnterError,
    Other
}


impl Display for ContextErrorType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "ContextErrorType::{:?}", self)
    }
}


pub type ContextError = Error<ContextErrorType>;


// ===========================================================================
// Unit tests
// ===========================================================================


#[cfg(test)]
mod tests {

    #[test]
    fn what_error() {
        use super::*;

        #[derive(Debug)]
        pub enum MyError {
            What
        }


        impl Display for MyError {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                write!(f, "MyError::{:?}", self)
            }
        }


        pub type CustomError = Error<MyError>;

        fn test() -> Result<(), CustomError> {
            let err = CustomError::new(MyError::What, "What error?!");
            Err(err)
        }

        if let Err(e) = test() {
            assert_eq!(format!("{}", e),
                       "Error(MyError::What) => What error?!")
        };
    }

}

// ===========================================================================
//
// ===========================================================================
