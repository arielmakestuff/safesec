// src/network/rpc/message.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

//! This module defines the base type of all RPC messages
//!
//! The [`Message`] type is the core underlying type that wraps around the
//! [`rmpv::Value`] type. It ensures that the given [`rmpv::Value`] object
//! conforms with an expected minimum of the RPC spec.
//!
//! The intended use is for a buffer of bytes to be deserialized into a
//! [`rmpv::Value`] value (eg using [`rmp-serde`]). This value would then be
//! used to create a [`Message`] value.
//!
//! # Types and Traits
//!
//! This module provides 2 types and 2 traits as the building blocks of all RPC
//! messages. The types provided are:
//!
//! * MessageType
//! * Message
//!
//! And the traits provided are:
//!
//! * CodeConvert
//! * RpcMessage
//!
//! While each type and trait is discussed in more detail in their definition,
//! the following summarizes the purpose of each type and trait.
//!
//! ## MessageType
//!
//! This is an enum that defines all possible message types. Due to sticking
//! somewhat closely to the official [`msgpack-rpc`] spec, there are only 3
//! types of messages that can be defined:
//!
//! * Request
//! * Response
//! * Notification
//!
//! ## Message
//!
//! The core base type of all RPC messages.
//!
//! ## CodeConvert
//!
//! This trait provides a common interface for converting between a number and
//! a type.
//!
//! ## RpcMessage
//!
//! This trait provides a interface common to all messages.
//!
//! # Validation
//!
//! When the [`Message`] type is being instantiated, it checks for the following:
//!
//! * The [`rmpv::Value`] type being wrapped is an array
//! * The array is not empty
//! * The array's first item is an integer that can be mapped to the
//!   [`MessageType`] enum
//!
//! [`Message`]: struct.Message.html
//! [`rmpv::Value`]: https://docs.rs/rmpv/0.4.0/rmpv/enum.Value.html
//! [`MessageType`]: enum.MessageType.html
//! [`rmp-serde`]: https://docs.rs/rmp-serde/0.13.3/rmp_serde
//! [`msgpack-rpc`]: https://github.com/msgpack-rpc/msgpack-rpc/blob/master/spec.md
//!
//! # Example
//!
//! ```rust
//! extern crate rmp_serde as rmps;
//! extern crate rmpv;
//! extern crate safesec;
//!
//! use rmpv::Value;
//! use safesec::network::rpc::{CodeConvert, Message, MessageType, RpcMessage};
//!
//! # fn main() {
//! // Build expected value
//! let msgtype = Value::from(MessageType::Request.to_number());
//! let msgid = Value::from(42);
//! let msgcode = Value::from(0);
//! let msgargs = Value::Array(vec![Value::from(42)]);
//! let expected = Value::Array(vec![msgtype, msgid, msgcode, msgargs]);
//!
//! // Given a buffer of bytes
//! let buf: Vec<u8> = vec![148, 0, 42, 0, 145, 42];
//!
//! // Deserializing it will give the expected value
//! let val = rmps::from_slice(&buf[..]).unwrap();
//! assert_eq!(val, expected);
//!
//! // Turn the value into a Message type
//! let msg = Message::from(val).unwrap();
//!
//! // Grab a reference to the internal value and check against expected
//! assert_eq!(msg.raw_message(), &expected);
//!
//! // Check internal array items against expected
//! let expected_array = expected.as_array().unwrap();
//! let val_array = msg.message();
//! for i in 0..expected_array.len() {
//!     assert_eq!(val_array[i], expected_array[i]);
//! }
//! # }
//! ```
//!


// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports
use std::clone::Clone;

// Third-party imports
// use rmp::Marker;
use rmpv::Value;

// Local imports
use ::error::{Error, GeneralError, Result};
use ::error::network::rpc::{RpcError, RpcResult};


// ===========================================================================
// Helpers
// ===========================================================================


// Return the name of a Value variant
pub fn value_type(arg: &Value) -> String {
    let ret = match *arg {
        Value::Nil => "nil",
        Value::Boolean(_) => "bool",
        Value::Integer(_) => "int",
        Value::F32(_) => "float32",
        Value::F64(_) => "float64",
        Value::String(_) => "str",
        Value::Binary(_) => "bytearray",
        Value::Array(_) => "array",
        Value::Map(_) => "map",
        Value::Ext(_, _) => "ext"
    };
    String::from(ret)
}


// ===========================================================================
// CodeConvert
// ===========================================================================


/// Allows converting between a number and a type.
///
/// The type implementing [`CodeConvert`] will usually be an enum that defines
/// different codes.
///
/// [`CodeConvert`]: trait.CodeConvert.html
pub trait CodeConvert<T>: Clone + PartialEq {
    /// Convert a number to type T.
    fn from_number(num: u8) -> Result<T>;

    /// Convert type To to a number.
    fn to_number(&self) -> u8;
}


// ===========================================================================
// MessageType
// ===========================================================================


/// Enum defining different types of messages
#[derive(Debug, PartialEq, Clone, CodeConvert)]
pub enum MessageType {
    /// A message initiating a request.
    Request,

    /// A message sent in response to a request.
    Response,

    /// A message notifying of some additional information.
    Notification
}


// ===========================================================================
// Message
// ===========================================================================


/// Define methods common to all RPC messages
pub trait RpcMessage {

    /// Return the message as a vec containing [`rmpv::Value`] objects.
    fn message(&self) -> &Vec<Value>;

    /// Return a reference to the internally owned [`rmpv::Value`] object.
    fn raw_message(&self) -> &Value;

    /// Return the message's type.
    ///
    /// # Errors
    ///
    /// If the internally owned [`rmpv::Value`] object contains an invalid
    /// value for the message type, then an RpcError::InvalidMessageType
    /// error is returned.
    fn message_type(&self) -> RpcResult<MessageType> {
        let msgtype: u8 = match self.message()[0].as_u64() {
            Some(v) => v as u8,
            None => unreachable!()
        };
        match MessageType::from_number(msgtype) {
            Ok(c) => Ok(c),
            Err(_) => {
                let errmsg = msgtype.to_string();
                let err = Error::new(RpcError::InvalidMessageType, errmsg);
                Err(err)
            }
        }
    }

    /// Check if an unsigned integer value can be cast as a given integer type.
    ///
    /// # Errors
    ///
    /// If the value is either None or a value that cannot fit into the type
    /// specified by `expected`, then the GeneralError::InvalidType error
    /// is returned.
    fn check_int(val: Option<u64>, max_value: u64, expected: String) -> Result<u64> {
        match val {
            None => {
                let errmsg = format!("expected {} but got {}",
                                     expected,
                                     String::from("None"));
                let err = Error::new(GeneralError::InvalidType, errmsg);
                Err(err)
            },
            Some(v) => {
                if v > max_value {
                    let errmsg = format!("expected value <= {} but got value {}",
                                         max_value.to_string(),
                                         v.to_string());
                    let err = Error::new(GeneralError::InvalidType, errmsg);
                    return Err(err);
                }
                Ok(v)
            }
        }
    }

    /// Return the string name of an [`rmpv::Value`] object.
    fn value_type_name(arg: &Value) -> String {
        value_type(arg)
    }

}


/// The [`Message`] type is the core underlying type of all RPC messages
///
/// [`Message`] wraps around the [`rmpv::Value`] type. It ensures that the
/// given [`rmpv::Value`] object conforms with the expected RPC spec.
///
/// [`Message`]: message/struct.Message.html
/// [`rmpv::Value`]: https://docs.rs/rmpv/0.4.0/rmpv/enum.Value.html
pub struct Message {
    msg: Value
}


impl RpcMessage for Message {
    fn message(&self) -> &Vec<Value> {
        if let Some(array) = self.msg.as_array() {
            array
        } else {
            unreachable!()
        }
    }

    fn raw_message(&self) -> &Value {
        &self.msg
    }
}


impl Message {

    /// Converts an [`rmpv::Value`].
    ///
    /// # Errors
    ///
    /// An error is returned if any of the following are true:
    ///
    /// 1. The value is not an array
    /// 2. The length of the array is less than 3 or greater than 4
    /// 3. The array's first item is not a u8
    pub fn from(val: Value) -> RpcResult<Self> {
        if let Some(array) = val.as_array() {
            let arraylen = array.len();
            if arraylen < 3 || arraylen > 4 {
                let errmsg = format!("expected array length of either 3 or 4, got {}",
                                     arraylen);
                let err = Error::new(RpcError::InvalidArrayLength, errmsg);
                return Err(err);
            }

            // Check msg type
            let msgtype = Self::check_int(array[0].as_u64(),
                                          u8::max_value() as u64,
                                          "u8".to_string());
            if let Err(e) = msgtype {
                let err = Error::new(RpcError::InvalidMessageType, e);
                return Err(err);
            }
        } else {
            let errmsg = format!("expected array but got {}",
                                 value_type(&val));
            let err = Error::new(RpcError::InvalidMessage, errmsg);
            return Err(err);
        }
        Ok(Self {msg: val})
    }
}


// Clone impl
impl Clone for Message {
    fn clone(&self) -> Self {
        Self { msg: self.msg.clone() }
    }

    fn clone_from(&mut self, source: &Self) {
        self.msg = source.raw_message().clone();
    }
}


// ===========================================================================
// Tests
// ===========================================================================


#[cfg(test)]
mod tests {
    // std lib imports
    use std::error::Error;

    // Third-party imports
    use quickcheck::TestResult;
    use rmpv::Value;

    // Local imports
    use ::error;
    use ::error::network::rpc::RpcError;
    use ::network::rpc::message::{CodeConvert, Message, MessageType,
                                  RpcMessage};
    use super::value_type;

    // --------------------
    // Decode tests
    // --------------------

    // #[test]
    // fn test_temp() {
    //     let buf = [0x93, 0xa4, 0x4a, 0x6f, 0x68, 0x6e, 0xa5, 0x53, 0x6d, 0x69, 0x74, 0x68, 0x2a];
    //     let expected = Value::Array(vec![Value::from("John"),
    //                                      Value::from("Smith"),
    //                                      Value::from(42)]);
    //     assert_eq!(expected, rmps::from_slice(&buf[..]).unwrap());
    // }

    // --------------------
    // MessageType
    // --------------------
    // MessageType::from_number
    quickcheck! {
        // MessageType::from_number's Ok value when casted as u8 is equal to u8
        // input value
        fn messagetype_from_number_variant_u8_matches_number(xs: u8) -> TestResult {
            match MessageType::from_number(xs) {
                Err(_) => TestResult::discard(),
                Ok(code) => {
                    TestResult::from_bool(code as u8 == xs)
                }
            }
        }

        // MessageType::from_number returns error if input value is >= the number
        // of variants
        fn messagetype_from_number_invalid_number(xs: u8) -> TestResult {
            if xs < 3 {
                return TestResult::discard()
            }
            match MessageType::from_number(xs) {
                Err(c) => {
                    let errmsg = xs.to_string();
                    let errkind = error::GeneralError::InvalidValue;
                    // let err = error::Error(errkind, _);
                    TestResult::from_bool(c.kind() == errkind &&
                                          c.description() == errmsg)
                },
                Ok(_) => TestResult::from_bool(false)
            }
        }

    }

    // MessageType::to_number
    quickcheck! {
        // MessageType::to_number always returns an integer < 3
        fn messagetype_to_number_lt_3(xs: u8) -> TestResult {
            if xs > 2 {
                return TestResult::discard()
            }
            let val = MessageType::from_number(xs).unwrap();
            TestResult::from_bool(val.to_number() < 3)
        }

        // MessageType::to_number return value converted back to MessageType ==
        // original MessageType value
        fn messagetype_to_number_from_number(xs: u8) -> TestResult {
            if xs > 2 {
                return TestResult::discard()
            }
            let val = MessageType::from_number(xs).unwrap();
            let after = MessageType::from_number(val.to_number()).unwrap();
            TestResult::from_bool(val == after)
        }
    }


    // --------------------
    // Message
    // --------------------

    // Helper
    fn mkmessage(msgtype: u8) -> Message {
        let msgtype = Value::from(msgtype);
        let msgid = Value::from(0);
        let msgcode = Value::from(0);
        let msgargs = Value::Nil;
        let val = Value::from(vec![msgtype, msgid, msgcode, msgargs]);
        Message::from(val).unwrap()
    }


    // Message::check_int
    quickcheck! {
        // val == None always returns an err with given marker
        fn message_check_int_none_val(xs: u64) -> bool {
            let expected = "expected u8 but got None";
            if let Err(e) = Message::check_int(None, xs, String::from("u8")) {
                e.kind() == error::GeneralError::InvalidType && e.description() == expected
            } else {
                false
            }
        }

        // val > max value returns an err with given marker
        fn message_check_int_val_gt_max_value(val: u64, max_value: u64) -> TestResult {
            if val <= max_value {
                return TestResult::discard()
            }

            let expected = format!("expected value <= {} but got value {}",
                                   max_value, val);
            let result = Message::check_int(Some(val), max_value, val.to_string());
            if let Err(e) = result {
                TestResult::from_bool(e.kind() == error::GeneralError::InvalidType &&
                                      e.description() == expected)
            } else {
                TestResult::from_bool(false)
            }
        }

        // val <= max returns value
        fn message_check_int_val_le_max_value(val: u64, max_value: u64) -> TestResult {
            if val > max_value {
                return TestResult::discard()
            }

            let result = Message::check_int(Some(val), max_value, val.to_string());
            if let Ok(v) = result {
                TestResult::from_bool(v == val)
            } else {
                TestResult::from_bool(false)
            }
        }
    }

    // Message::message_type
    quickcheck! {
        // Unknown code number returns error
        fn message_message_type_bad_code_number(varnum: u8) -> TestResult {
            if varnum < 3 {
                return TestResult::discard()
            }
            let expected = varnum.to_string();
            let msg = mkmessage(varnum);
            if let Err(e) = msg.message_type() {
                TestResult::from_bool(e.kind() == RpcError::InvalidMessageType &&
                                      e.description() == expected)
            } else {
                TestResult::from_bool(false)
            }
        }

        // Known code number returns MessageType variant
        fn message_message_type_good_code_number(varnum: u8) -> TestResult {
            if varnum >= 3 {
                return TestResult::discard()
            }
            let expected = MessageType::from_number(varnum).unwrap();
            let msg = mkmessage(varnum);
            if let Ok(c) = msg.message_type() {
                TestResult::from_bool(c == expected)
            } else {
                TestResult::from_bool(false)
            }
        }
    }

    use rmpv::{Integer, Utf8String};

    // Message::value_type_name
    quickcheck! {

        // Return value is never the empty string
        fn message_value_type_name_return_nonempty_string(i: usize) -> TestResult {
            let values = vec![
                Value::Nil,
                Value::Boolean(true),
                Value::Integer(Integer::from(42)),
                Value::F32(42.0),
                Value::F64(42.0),
                Value::String(Utf8String::from("hello")),
                Value::Binary(vec![0, 0]),
                Value::Array(vec![Value::from(42)]),
                Value::Map(vec![(Value::from(42), Value::from("ANSWER"))]),
                Value::Ext(-42, vec![0, 1, 2]),
            ];

            if i > values.len() - 1 {
                return TestResult::discard()
            }

            let choice = &values[i];
            let ret = Message::value_type_name(choice);
            TestResult::from_bool(ret.len() > 0)
        }

        // Return value is expected name of the Value variant
        fn message_value_type_name_return_expected_string(i: usize) -> TestResult {
            let values = vec![
                (Value::Nil, "nil"),
                (Value::Boolean(true), "bool"),
                (Value::Integer(Integer::from(42)), "int"),
                (Value::F32(42.0), "float32"),
                (Value::F64(42.0), "float64"),
                (Value::String(Utf8String::from("hello")), "str"),
                (Value::Binary(vec![0, 0]), "bytearray"),
                (Value::Array(vec![Value::from(42)]), "array"),
                (Value::Map(vec![(Value::from(42), Value::from("ANSWER"))]), "map"),
                (Value::Ext(-42, vec![0, 1, 2]), "ext"),
            ];

            if i > values.len() - 1 {
                return TestResult::discard()
            }

            let choice = &values[i];
            let ret = Message::value_type_name(&choice.0);
            TestResult::from_bool(ret == choice.1)
        }
    }

    // Message::message
    #[test]
    fn message_message_value() {
        let v = Value::from(vec![Value::from(42)]);
        let expected = v.clone();
        let m = Message { msg: v };

        let msg_val = m.message();
        assert_eq!(msg_val, expected.as_array().unwrap());
    }

    // Should only panic if manually creating a Message object using a non
    // Vec<Value> instead of using the from function
    #[test]
    #[should_panic]
    fn message_message_panic() {
        let v = Value::from(Value::from(42));
        let m = Message { msg: v };
        m.message();
    }

    //Message::raw_message
    #[test]
    fn message_raw_message() {
        let v = Value::from(42);
        let expected = v.clone();
        let msg = Message { msg: v };
        assert_eq!(msg.raw_message(), &expected);
    }

    // If a non-Value::Array is stored then will always return an error
    #[test]
    fn message_from_non_array_always_err() {
        let v = Value::from(42);
        let expected = format!("expected array but got {}",
                               value_type(&v));
        let ret = match Message::from(v) {
            Err(e) => {
                (e.kind() == RpcError::InvalidMessage &&
                 e.description() == expected)
            },
            _ => false
        };
        assert!(ret)
    }

    quickcheck! {
        fn message_from_invalid_array_length(val: Vec<u8>) -> TestResult {
            let arraylen = val.len();
            if arraylen == 3 || arraylen == 4 {
                return TestResult::discard()
            }

            // GIVEN
            // an array with length either < 3 or > 4
            let valvec: Vec<Value> = val.iter()
                .map(|v| Value::from(v.clone())).collect();
            let array = Value::from(valvec);

            // WHEN
            // creating a message using from method
            let expected = format!("expected array length of either 3 or 4, got {}",
                                   arraylen);
            let result = match Message::from(array) {
                Err(e) => {
                    (e.kind() == RpcError::InvalidArrayLength &&
                     e.description() == expected)
                },
                _ => false
            };

            // THEN
            // an appropriate error is returned
            TestResult::from_bool(result)
        }

        fn message_from_invalid_messagetype_number(code: u64) -> TestResult {
            let maxval = u8::max_value() as u64;
            if code <= maxval {
                return TestResult::discard()
            }

            // GIVEN
            // array with invalid code number (ie code number is >
            // u8::max_value()
            let array: Vec<Value> = vec![code, 42, 42].iter()
                .map(|v| Value::from(v.clone())).collect();

            // WHEN
            // creating a message via Message::from()
            let expected = format!("expected value <= {} but got value {}",
                                   maxval, code);
            let result = match Message::from(Value::from(array)) {
                Err(e) => {
                    (e.kind() == RpcError::InvalidMessageType &&
                     e.description() == expected)
                }
                _ => false
            };

            // THEN
            // MessageError::InvalidType error is returned
            TestResult::from_bool(result)
        }
    }

    // A valid value is an array with a length of 3 or 4 and the first item in
    // the array is u8
    #[test]
    fn message_from_valid_value() {
        let valvec: Vec<Value> = vec![42, 42, 42].iter()
            .map(|v| Value::from(v.clone())).collect();
        let array = Value::from(valvec);
        let expected = array.clone();

        let ret = match Message::from(array) {
            Ok(m) => {
                m.raw_message() == &expected
            },
            _ => false
        };
        assert!(ret)
    }

}


// ===========================================================================
//
// ===========================================================================
