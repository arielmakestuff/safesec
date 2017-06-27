// src/error/mod.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

//! This module defines types and traits needed for general error handling.
//!
//! Errors are handled by using the safesec::error::Error type with custom enum
//! variants. As the safesec::error::Error type is a slightly simplified
//! version of the std::io::Error type, it can also create an error based on an
//! existing error.
//!
//! # Custom Errors
//!
//! In order to create custom errors, two things must be implemented.
//!
//! First, an enum must be created that implements the following traits:
//!    - std::fmt::Debug
//!    - std::fmt::Display
//!    - std::marker::Copy
//!    - safesec::error::ErrorMessage
//!
//! While these traits are required to be implemented, it is also helpful for
//! testing purposes if other traits are implemented as well:
//!
//! - Clone
//! - Eq
//! - Hash
//! - Ord
//! - PartialEq
//! - PartialOrd
//!
//! Second, an Result alias should also be defined. While not strictly
//! necessary, it does make it much simpler to use.
//!
//! # Example Custom Error
//!
//! ```rust
//! use std::error::Error;
//! use std::fmt;
//! use std::result;
//!
//! // Need to name Error as MyError to prevent clashes with the std lib Error
//! // trait.
//! use safesec::error::{Error as MyError, ErrorMessage};
//!
//! // Result alias
//! pub type Result<T> = result::Result<T, MyError<CustomError>>;
//!
//! // Error enum
//! #[derive(Debug, Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
//! pub enum CustomError {
//!     IsError,
//!     NoError,
//! }
//!
//! impl ErrorMessage for CustomError {
//!     fn message(&self) -> &'static str {
//!         match *self {
//!             CustomError::IsError => "An error definitely happened!",
//!             CustomError::NoError => "Why no error?",
//!         }
//!     }
//! }
//!
//! impl fmt::Display for CustomError {
//!     fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
//!         write!(fmt, "{}", self.message().to_string())
//!     }
//! }
//!
//! fn always_error() -> Result<()> {
//!     Err(MyError::new(CustomError::NoError, "the error is a lie"))
//! }
//!
//! # fn main() {
//! let expected = "the error is a lie";
//! match always_error() {
//!     Err(e) => {
//!         assert_eq!(e.kind(), CustomError::NoError);
//!         assert_eq!(e.description(), expected);
//!     },
//!     _ => unreachable!()
//! }
//! # }
//! ```
//!

// ===========================================================================
// Modules
// ===========================================================================


pub mod network;


// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports
use std::convert::From;
use std::error;
use std::fmt;
use std::result;

// Third-party imports

// Local imports

// ===========================================================================
//
// ===========================================================================


/// Define method that returns a message associated with an object.
///
/// Intended to be implemented for enums, where the message() method will
/// return the appropriate message for each enum variant.
pub trait ErrorMessage {

    /// Return the appropriate message for the current object.
    fn message(&self) -> &'static str;
}


enum Repr<T>
    where T: fmt::Debug+fmt::Display+Copy+ErrorMessage
{
    Simple(T),
    User(Box<UserError<T>>),
}


#[derive(Debug)]
struct UserError<T>
    where T: fmt::Debug+fmt::Display+Copy+ErrorMessage
{
    kind: T,
    error: Box<error::Error+Send+Sync>,
}


/// A new error type used by `safesec`.
///
/// Modeled after `std::io::Error`.
#[derive(Debug)]
pub struct Error<T>
    where T: fmt::Debug+fmt::Display+Copy+ErrorMessage
{
    err: Repr<T>
}


impl<T> From<T> for Error<T>
    where T: fmt::Debug+fmt::Display+Copy+ErrorMessage
{
    /// Convert an error enum variant into a simple error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use safesec::error::{Error, GeneralError};
    ///
    /// # fn main() {
    /// let err = Error::from(GeneralError::InvalidType);
    /// # }
    /// ```
    fn from(kind: T) -> Error<T> {
        Self {
            err: Repr::Simple(kind)
        }
    }
}


impl<T> Error<T>
    where T: fmt::Debug+fmt::Display+Copy+ErrorMessage
{
    /// Create a new error from a known type of error along with an arbitrary
    /// error payload.
    ///
    /// # Example
    ///
    /// ```rust
    /// use safesec::error::{Error, GeneralError};
    ///
    /// // Create an error from a string
    /// let err = Error::new(GeneralError::InvalidType, "no! anything but an invalid type!");
    ///
    /// // Create an error from a different error
    /// let err2 = Error::new(GeneralError::InvalidValue, err);
    /// ```
    pub fn new<E>(kind: T, error: E) -> Error<T>
        where E: Into<Box<error::Error+Send+Sync>>
    {
        let user_error = UserError {
            kind: kind,
            error: error.into()
        };
        Self { err: Repr::User(Box::new(user_error)) }
    }

    /// Returns a reference to the inner error wrapped by this error (if any).
    ///
    /// If this `Error` was created via `new`, then this function will return
    /// `Some`, otherwise it will return `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use safesec::error::{Error, GeneralError};
    ///
    /// fn print_error(err: &Error<GeneralError>) -> String {
    ///     if let Some(inner_err) = err.get_ref() {
    ///         format!("Inner error: {:?}", inner_err)
    ///     } else {
    ///         format!("No inner error")
    ///     }
    /// }
    ///
    /// # fn main() {
    /// // Returns "No inner error".
    /// let msg = print_error(&Error::from(GeneralError::InvalidType));
    /// assert_eq!(msg, "No inner error");
    ///
    /// // Returns "Inner error: StringError(\"yes!\")".
    /// let msg = print_error(&Error::new(GeneralError::InvalidValue, "yes!"));
    /// assert_eq!(msg, "Inner error: StringError(\"yes!\")");
    /// # }
    /// ```
    pub fn get_ref(&self) -> Option<&(error::Error+Send+Sync+'static)> {
        match self.err {
            Repr::Simple(_) => None,
            Repr::User(ref c) => Some(&*c.error),
        }
    }

    /// Returns a mutable reference to the inner error wrapped by this error
    /// (if any).
    ///
    /// If this `Error` was created via `new`, then this function will return
    /// `Some`, otherwise it will return `None`.
    ///
    /// # Example
    ///
    /// ```rust
    ///
    /// use std::error::Error as StdError;
    /// use std::fmt;
    /// use safesec::error::{Error, ErrorMessage};
    ///
    /// #[derive(Debug, Copy, Clone)]
    /// pub enum AnError {
    ///     Hello,
    ///     World,
    /// }
    ///
    /// impl ErrorMessage for AnError {
    ///     fn message(&self) -> &'static str {
    ///         match *self {
    ///             AnError::Hello => "hello",
    ///             AnError::World => "world",
    ///         }
    ///     }
    /// }
    ///
    /// impl fmt::Display for AnError {
    ///     fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
    ///         write!(fmt, "{}", self.message().to_string())
    ///     }
    /// }
    ///
    /// #[derive(Debug)]
    /// struct MyError {
    ///     answer: String
    /// }
    ///
    /// impl MyError {
    ///     fn new() -> Self {
    ///         Self {
    ///             answer: "1".to_string()
    ///         }
    ///     }
    ///
    ///     fn answer(&self) -> &str {
    ///         &self.answer
    ///     }
    ///
    ///     fn change_answer(&mut self, new_val: String) {
    ///         self.answer = new_val;
    ///     }
    /// }
    ///
    /// impl StdError for MyError {
    ///     fn description(&self) -> &str { &self.answer }
    /// }
    ///
    /// impl fmt::Display for MyError {
    ///     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    ///         write!(f, "MyError: {}", self.answer)
    ///     }
    /// }
    ///
    /// fn change_answer(mut err: Error<AnError>) -> Error<AnError> {
    ///     if let Some(inner_err) = err.get_mut() {
    ///         inner_err.downcast_mut::<MyError>().unwrap()
    ///             .change_answer(42.to_string());
    ///     }
    ///     err
    /// }
    ///
    /// # fn main() {
    /// // Has default answer of "1"
    /// let err = Error::new(AnError::Hello, MyError::new());
    /// let inner = err.into_inner().unwrap().downcast::<MyError>().unwrap();
    /// assert_eq!(inner.answer(), "1");
    ///
    /// // Changed answer to "42"
    /// let err = Error::new(AnError::World, MyError::new());
    /// let err = change_answer(err);
    /// let inner = err.into_inner().unwrap().downcast::<MyError>().unwrap();
    /// assert_eq!(inner.answer(), "42");
    /// # }
    /// ```
    pub fn get_mut(&mut self) -> Option<&mut (error::Error+Send+Sync+'static)> {
        match self.err {
            Repr::Simple(_) => None,
            Repr::User(ref mut c) => Some(&mut *c.error),
        }
    }

    /// Consumes the `Error` returning its inner error (if any)
    ///
    /// If this `Error` was created via `new`, then this function will return
    /// `Some`, otherwise it will return `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use safesec::error::{Error, GeneralError};
    ///
    /// fn print_error(err: Error<GeneralError>) -> String {
    ///     if let Some(inner_err) = err.into_inner() {
    ///         format!("Inner error: {:?}", inner_err)
    ///     } else {
    ///         format!("No inner error")
    ///     }
    /// }
    ///
    /// # fn main() {
    /// // Returns "No inner error".
    /// let msg = print_error(Error::from(GeneralError::InvalidType));
    /// assert_eq!(msg, "No inner error");
    ///
    /// // Returns "Inner error: StringError(\"yes!\")".
    /// let msg = print_error(Error::new(GeneralError::InvalidValue, "yes!"));
    /// assert_eq!(msg, "Inner error: StringError(\"yes!\")");
    /// # }
    /// ```
    pub fn into_inner(self) -> Option<Box<error::Error+Send+Sync>> {
        match self.err {
            Repr::Simple(_) => None,
            Repr::User(c) => Some(c.error)
        }
    }

    /// Returns the corresponding enum variant for this error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use safesec::error::{Error, GeneralError};
    ///
    /// # fn main() {
    /// // Simple error
    /// let err = Error::from(GeneralError::InvalidType);
    /// assert_eq!(err.kind(), GeneralError::InvalidType);
    ///
    /// // Error
    /// let err = Error::new(GeneralError::InvalidValue, "wat");
    /// assert_eq!(err.kind(), GeneralError::InvalidValue);
    /// # }
    /// ```
    pub fn kind(&self) -> T {
        match self.err {
            Repr::Simple(kind) => kind,
            Repr::User(ref c) => c.kind,
        }
    }
}


impl<T> fmt::Debug for Repr<T>
    where T: fmt::Debug+fmt::Display+Copy+ErrorMessage
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Repr::Simple(kind) => fmt.debug_tuple("Kind")
                .field(&kind).finish(),
            Repr::User(ref c) => fmt.debug_tuple("UserError")
                .field(c).finish(),
        }
    }
}


impl<T> fmt::Display for Error<T>
    where T: fmt::Debug+fmt::Display+Copy+ErrorMessage
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self.err {
            Repr::Simple(kind) => write!(fmt, "{}", kind.to_string()),
            Repr::User(ref c) => c.error.fmt(fmt),
        }
    }
}


impl<T> error::Error for Error<T>
    where T: fmt::Debug+fmt::Display+Copy+ErrorMessage
{
    fn description(&self) -> &str {
        match self.err {
            Repr::Simple(_) => &*self.kind().message(),
            Repr::User(ref c) => c.error.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match self.err {
            Repr::Simple(_) => None,
            Repr::User(ref c) => c.error.cause(),
        }
    }
}


// ===========================================================================
// General errors
// ===========================================================================


pub type Result<T> = result::Result<T, Error<GeneralError>>;


#[derive(Debug, Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GeneralError {
    InvalidType,
    InvalidValue,
}


impl ErrorMessage for GeneralError {
    fn message(&self) -> &'static str {
        match *self {
            GeneralError::InvalidType => "Invalid type",
            GeneralError::InvalidValue => "Invalid value",
        }
    }
}


impl fmt::Display for GeneralError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.message().to_string())
    }
}


// ===========================================================================
//
// ===========================================================================
