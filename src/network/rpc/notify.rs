// src/network/rpc/notify.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

//! This module defines the Notification RPC message type.
//!
//! A Notification RPC message is used by either server or client to send a
//! notification.  server. Based on the generic [`Message`] type, the
//! Notification message type is essentially an array containing 3 items. These
//! 3 items are:
//!
//! 1. Message Type - This will always be the Notification message type. While
//!    represented as the enum variant `MessageType::Notification`, the value
//!    stored in the array is actually a u8 integer.
//!
//! 2. Message code - This is a u8 integer representing the specific
//!    notification being made.
//!
//! 3. Message arguments - this is an array of values used to provide
//!    information needed to be included with notice specified by the message
//!    code.
//!
//! # Example
//!
//! To create a new Notification object, you can create one from an existing
//! [`Message`] instance. This is used, for example, when the message is
//! deserialized by the server into a general [`Message`] object, identified as
//! a Notification message, and it is required to perform Notification specific
//! operations on the message.
//!
//! Alternatively, a `NotificationMessage` object can be created manually via
//! the `NotificationMessage::new()` method
//!
//! ```text
//!
//! extern crate rmpv;
//! extern crate safesec;
//!
//! #[macro_use]
//! extern crate safesec_derive;
//!
//! use rmpv::Value;
//! use safesec::error::{Error, GeneralError, Result};
//! use safesec::network::rpc::message::{CodeConvert, Message, MessageType, RpcMessage};
//! use safesec::network::rpc::notify::{NotificationMessage, RpcNotification};
//!
//! // Define Error codes
//! #[derive(Debug, Clone, PartialEq, CodeConvert)]
//! enum NotifyCode {
//!     Yep,
//!     UhHuh,
//! }
//!
//! # fn main() {
//! // Create an alias for ResponseMessage
//! type Notice = NotificationMessage<NotifyCode>;
//!
//! // Build Message
//! let msgtype = Value::from(MessageType::Notification.to_number());
//! let msgcode = Value::from(Notify::Yep.to_number());
//! let msgargs = Value::Array(vec![Value::from(9001)]);
//! let msgval = Value::Array(vec![msgtype, msgcode, msgargs]);
//! let msg = Message::from(msgval).unwrap();
//!
//! // Turn the message into a Response type
//! let nmsg = Notice::from(msg).unwrap();
//! assert_eq!(nmsg.message_type().unwrap(), MessageType::Notification);
//! assert_eq!(nmsg.message_code(), NotifyCode::Yep);
//! assert_eq!(nmsg.message_args(), &vec![Value::from(9001)]);
//!
//! // Create a brand new response from scratch
//! let new_nmsg = Notice::new(NotifyCode::UhHuh, vec![Value::from(42)]);
//! assert_eq!(new_nmsg.message_type().unwrap(), MessageType::Response);
//! assert_eq!(new_nmsg.message_code(), NotifyCode::UhHuh);
//! assert_eq!(new_nmsg.message_args(), &vec![Value::from(42)]);
//! # }
//!
//! ```
//!
// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports
use std::marker::PhantomData;

// Third-party imports
use rmpv::Value;

// Local imports
use ::network::rpc::message::{CodeConvert, Message, MessageType, RpcMessage,
                              value_type};
use ::error::Error;
use ::error::network::rpc::{RpcError, RpcResult};


// ===========================================================================
// NotificationMessage
// ===========================================================================


/// Trait providing Notification message specific getter methods.
///
/// # Example
///
/// ```rust
/// extern crate rmpv;
/// extern crate safesec;
///
/// use rmpv::Value;
/// use safesec::network::rpc::message::{MessageType, RpcMessage};
/// use safesec::network::rpc::notify::{NotificationMessage, RpcNotice};
///
/// # fn main() {
/// // Create Request alias
/// type Notice = NotificationMessage<MessageType>;
///
/// // Re-use MessageType as message code
/// let req = Notice::new(MessageType::Request,
///                       vec![Value::from(42)]);
///
/// // Check all getter methods
/// assert_eq!(req.message_type().unwrap(), MessageType::Notification);
/// assert_eq!(req.message_code(), MessageType::Request);
/// assert_eq!(req.message_args(), &vec![Value::from(42)]);
/// # }
/// ```
pub trait RpcNotice<C>: RpcMessage
    where C: CodeConvert<C>
{
    fn message_code(&self) -> C {
        let msgcode = &self.message()[1];
        let msgcode = msgcode.as_u64().unwrap() as u8;
        C::from_number(msgcode).unwrap()
    }

    fn message_args(&self) -> &Vec<Value> {
        let msgargs = &self.message()[2];
        msgargs.as_array().unwrap()
    }
}


/// A representation of the Notification RPC message type.
pub struct NotificationMessage<C>
    where C: CodeConvert<C>
{
    msg: Message,
    msgtype: PhantomData<C>
}


impl<C> RpcMessage for NotificationMessage<C>
    where C: CodeConvert<C>
{
    fn message(&self) -> &Vec<Value> {
        self.msg.message()
    }

    fn raw_message(&self) -> &Value {
        self.msg.raw_message()
    }
}


impl<C> RpcNotice<C> for NotificationMessage<C> where C: CodeConvert<C> {}


impl<C> NotificationMessage<C> where C: CodeConvert<C> {

    /// Create a brand new NotificationMessage object.
    ///
    /// # Example
    ///
    /// ```rust
    /// extern crate rmpv;
    /// extern crate safesec;
    ///
    /// use rmpv::Value;
    /// use safesec::network::rpc::message::{MessageType, RpcMessage};
    /// use safesec::network::rpc::notify::{NotificationMessage, RpcNotice};
    ///
    /// # fn main() {
    /// // Create Notice alias
    /// type Notice = NotificationMessage<MessageType>;
    ///
    /// // Re-use MessageType as message code
    /// let req = Notice::new(MessageType::Notification,
    ///                       vec![Value::from(42)]);
    /// # }
    /// ```
    pub fn new(notifycode: C, args: Vec<Value>) -> Self {
        let msgtype = Value::from(MessageType::Notification as u8);
        let notifycode = Value::from(notifycode.to_number());
        let msgargs = Value::from(args);
        let msgval = Value::from(vec![msgtype, notifycode, msgargs]);

        match Message::from(msgval) {
            Ok(msg) => Self { msg: msg, msgtype: PhantomData },
            Err(_) => unreachable!()
        }
    }

    /// Create a NotificationMessage from a Message
    ///
    /// # Example
    ///
    /// ```rust
    /// extern crate rmpv;
    /// extern crate safesec;
    ///
    /// use rmpv::Value;
    /// use safesec::network::rpc::message::{CodeConvert, Message,
    ///                                      MessageType, RpcMessage};
    /// use safesec::network::rpc::notify::{NotificationMessage, RpcNotice};
    ///
    /// # fn main() {
    /// // Create an alias for NotificationMessage, re-using `MessageType` as the
    /// // message code.
    /// type Notice = NotificationMessage<MessageType>;
    ///
    /// // Build Message
    /// let msgtype = Value::from(MessageType::Notification.to_number());
    /// let msgcode = Value::from(MessageType::Request.to_number());
    /// let msgargs = Value::Array(vec![Value::from(9001)]);
    /// let msgval = Value::Array(vec![msgtype, msgcode, msgargs]);
    /// let msg = Message::from(msgval).unwrap();
    ///
    /// // Turn the message into a Request type
    /// let req = Notice::from(msg).unwrap();
    /// # }
    /// ```
    pub fn from(msg: Message) -> RpcResult<Self> {
        // Notifications is always represented as an array of 4 values
        {
            // Requests is always represented as an array of 3 values
            let array = msg.message();
            let arraylen = array.len();
            if arraylen != 3 {
                let errmsg = format!("expected array length of 3, got {}",
                                     arraylen);
                let err = Error::new(RpcError::InvalidArrayLength, errmsg);
                return Err(err);
            }

            // Run all check functions and return the first error generated
            let funcvec: Vec<fn(&Value) -> RpcResult<()>>;
            funcvec = vec![Self::check_message_type, Self::check_message_code,
                           Self::check_message_args];

            for (i, func) in funcvec.iter().enumerate() {
                func(&array[i])?;
            }
        }

        Ok(Self {msg: msg, msgtype: PhantomData})
    }

    // Checks that the message type parameter of a Notification message is
    // valid.
    //
    // This is a private method used by the public from() method
    fn check_message_type(msgtype: &Value) -> RpcResult<()> {
        let msgtype = msgtype.as_u64().unwrap() as u8;
        let expected_msgtype = MessageType::Notification.to_number();
        if msgtype != expected_msgtype {
            let errmsg = format!("expected {} for message type (ie \
                                  MessageType::Notification), got {}",
                                 expected_msgtype, msgtype);
            let err = Error::new(RpcError::InvalidMessageType, errmsg);
            return Err(err)
        }
        Ok(())
    }

    // Checks that the message code parameter of a Notification message is
    // valid.
    //
    // This is a private method used by the public from() method
    fn check_message_code(msgcode: &Value) -> RpcResult<()> {
        let msgcode = Self::check_int(msgcode.as_u64(),
                                      u8::max_value() as u64,
                                      "u8".to_string());
        match msgcode {
            Err(e) => {
                let err = Error::new(RpcError::InvalidNotification, e);
                return Err(err)
            }
            Ok(v) => {
                let u8val = v as u8;
                if let Err(e) = C::from_number(u8val) {
                    let err = Error::new(RpcError::InvalidNotification, e);
                    return Err(err)
                }
            }
        }
        Ok(())
    }

    // Check that the message arguments parameter of a Notification message is
    // valid.
    //
    // This is a private method used by the public from() method
    fn check_message_args(msgargs: &Value) -> RpcResult<()> {
        let args = msgargs.as_array();
        if args.is_none() {
            let errmsg = format!("expected array for request arguments but \
                                  got {}",
                                 value_type(&msgargs));
            let err = Error::new(RpcError::InvalidNotificationArgs, errmsg);
            return Err(err)
        }
        Ok(())
    }
}


// ===========================================================================
// Tests
// ===========================================================================


#[cfg(test)]
mod tests {
    // --------------------
    // Imports
    // --------------------
    // Stdlib imports
    use std::error::Error as StdError;

    // Third-party imports
    use quickcheck::TestResult;
    use rmpv::{Utf8String, Value};

    // Local imports
    use ::error::{Error, GeneralError, Result};
    use ::error::network::rpc::RpcError;
    use ::network::rpc::message::{CodeConvert, Message, MessageType, RpcMessage,
                                  value_type};
    use ::network::rpc::notify::{RpcNotice, NotificationMessage};

    // --------------------
    // Helpers
    // --------------------
    #[derive(Debug, PartialEq, Clone, CodeConvert)]
    enum TestCode {
        One,
        Two,
        Three
    }

    type Notice = NotificationMessage<TestCode>;

    // --------------------
    // NotificationMessage::new
    // --------------------

    quickcheck! {
        fn new_messagetype_always_notify(code: u8, args: Vec<u8>) -> TestResult {
            if code > 2 {
                return TestResult::discard()
            }

            let msgtype = Value::from(MessageType::Notification.to_number());
            let array: Vec<Value> = args.iter().map(|v| Value::from(v.clone())).collect();
            let array_copy = array.clone();

            // Build expected
            let msgargs = Value::Array(array);
            let a = vec![msgtype, Value::from(code), msgargs];
            let expected = Value::Array(a);

            // Compare NotificationMessage object to expected
            let notice = Notice::new(TestCode::from_number(code).unwrap(),
                                     array_copy);
            TestResult::from_bool(notice.raw_message() == &expected)
        }
    }

    // --------------------------
    // NotificationMessage::from
    // --------------------------

    #[test]
    fn from_invalid_arraylen() {
        // --------------------
        // GIVEN
        // --------------------
        // Message with only 4 arguments

        // Create message
        let msgtype = Value::from(MessageType::Notification.to_number());
        let msgcode = Value::from(TestCode::One.to_number());
        let arg2 = Value::from(42);
        let arg3 = Value::from(42);
        let array: Vec<Value> = vec![msgtype, msgcode, arg2, arg3];

        let val = Value::Array(array);
        let msg = Message::from(val).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // NotificationMessage::from is called with the message
        let result = Notice::from(msg);

        // --------------------
        // THEN
        // --------------------
        // Error is returned
        match result {
            Err(e) => {
                let expected = "expected array length of 3, got 4";
                assert_eq!(e.kind(), RpcError::InvalidArrayLength);
                assert_eq!(e.description(), expected);
            },
            _ => assert!(false)
        }
    }

    #[test]
    fn from_invalid_messagetype() {
        // --------------------
        // GIVEN
        // --------------------
        // Message with MessageType::Request

        // Create message
        let expected = MessageType::Request.to_number();
        let msgtype = Value::from(expected);
        let msgcode = Value::from(TestCode::One.to_number());
        let msgval = Value::from(42);

        let val = Value::Array(vec![msgtype, msgcode, msgval]);
        let msg = Message::from(val).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // NotificationMessage::from is called with the message
        let result = Notice::from(msg);

        // --------------------
        // THEN
        // --------------------
        // Error is returned
        match result {
            Err(e) => {
                let expected = format!("expected {} for message type (ie \
                                        MessageType::Notification), got {}",
                                       MessageType::Notification.to_number(),
                                       expected);
                assert_eq!(e.kind(), RpcError::InvalidMessageType);
                assert_eq!(e.description(), expected);
            },
            _ => assert!(false)
        }
    }

    #[test]
    fn from_message_code_invalid_type() {
        // --------------------
        // GIVEN
        // --------------------
        // Message with a string for message code

        // Create message
        let msgtype = Value::from(MessageType::Notification.to_number());
        let msgcode = Value::String(Utf8String::from("hello"));
        let msgval = Value::from(42);

        let val = Value::Array(vec![msgtype, msgcode, msgval]);
        let msg = Message::from(val).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // NotificationMessage::from is called with the message
        let result = Notice::from(msg);

        // --------------------
        // THEN
        // --------------------
        // Error is returned for the invalid message id
        match result {
            Err(e) => {
                let errmsg = "expected u8 but got None";
                assert_eq!(e.kind(), RpcError::InvalidNotification);
                assert_eq!(e.description(), errmsg);
            },
            _ => assert!(false)
        }
    }

    quickcheck! {
        fn from_message_code_invalid_value(msgcode: u64) -> TestResult {
            if msgcode <= u8::max_value() as u64 {
                return TestResult::discard()
            }

            // --------------------
            // GIVEN
            // --------------------
            // Message with a msgcode > u8::max_value() for message code

            // Create message
            let msgtype = Value::from(MessageType::Notification.to_number());
            let msgcode = Value::from(msgcode);
            let msgval = Value::from(42);

            let val = Value::Array(vec![msgtype, msgcode.clone(), msgval]);
            let msg = Message::from(val).unwrap();

            // --------------------
            // WHEN
            // --------------------
            // NotificationMessage::from is called with the message
            let result = Notice::from(msg);

            // --------------------
            // THEN
            // --------------------
            // Error is returned for the invalid message id value
            match result {
                Err(e) => {
                    let errmsg = format!("expected value <= {} but got value {}",
                                         u8::max_value().to_string(),
                                         msgcode.to_string());
                    TestResult::from_bool(e.kind() ==
                                            RpcError::InvalidNotification &&
                                          e.description() == errmsg)
                },
                _ => TestResult::from_bool(false)
            }
        }

        fn from_message_code_invalid_code(code: u8) -> TestResult {

            // --------------------
            // GIVEN
            // --------------------
            // Message with a msgcode > 2 for message code
            if code <= 2 {
                return TestResult::discard()
            }

            // Create message
            let msgtype = Value::from(MessageType::Notification.to_number());
            let msgcode = Value::from(code);
            let msgval = Value::from(42);

            let val = Value::Array(vec![msgtype, msgcode, msgval]);
            let msg = Message::from(val).unwrap();

            // --------------------
            // WHEN
            // --------------------
            // NotificationMessage::from is called with the message
            let result = Notice::from(msg);

            // --------------------
            // THEN
            // --------------------
            // Error is returned for the invalid message code value
            match result {
                Err(e) => {
                    let errmsg = code.to_string();
                    TestResult::from_bool(e.kind() ==
                                            RpcError::InvalidNotification &&
                                          e.description() == errmsg)
                },
                _ => TestResult::from_bool(false)
            }
        }
    }

    #[test]
    fn from_message_args_invalid_type() {
        // --------------------
        // GIVEN
        // --------------------
        // Message with an integer for message args

        // Create message
        let msgtype = Value::from(MessageType::Notification.to_number());
        let msgcode = Value::from(TestCode::One.to_number());
        let msgval = Value::from(42);

        let val = Value::Array(vec![msgtype, msgcode, msgval.clone()]);
        let msg = Message::from(val).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // NotificationMessage::from is called with the message
        let result = Notice::from(msg);

        // --------------------
        // THEN
        // --------------------
        // Error is returned for the invalid message id
        match result {
            Err(e) => {
                let errmsg = format!("expected array for request arguments but got {}",
                                     value_type(&msgval));
                assert_eq!(e.kind(), RpcError::InvalidNotificationArgs);
                assert_eq!(e.description(), errmsg);
            },
            _ => assert!(false)
        }
    }

    // --------------------
    // RpcMessage methods
    // --------------------

    #[test]
    fn rpcmessage_message() {
        // --------------------
        // GIVEN
        // --------------------
        // A request message

        // Create message
        let msgtype = Value::from(MessageType::Notification.to_number());
        let msgcode = Value::from(TestCode::One.to_number());
        let msgval = Value::Array(vec![Value::from(42)]);

        let val = Value::Array(vec![msgtype, msgcode, msgval]);
        let msg = Message::from(val).unwrap();
        let expected = msg.clone();
        let notice = Notice::from(msg).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // NotificationMessage::message() method is called
        let result = notice.message();

        // --------------------
        // THEN
        // --------------------
        // The contained value is as expected
        let expected = expected.message();
        assert_eq!(result, expected)
    }

    #[test]
    fn rpcmessage_raw_message() {
        // --------------------
        // GIVEN
        // --------------------
        // A request message

        // Create message
        let msgtype = Value::from(MessageType::Notification.to_number());
        let msgcode = Value::from(TestCode::One.to_number());
        let msgval = Value::Array(vec![Value::from(42)]);

        let val = Value::Array(vec![msgtype, msgcode, msgval]);
        let msg = Message::from(val).unwrap();
        let expected = msg.clone();
        let notice = Notice::from(msg).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // NotificationMessage::raw_message() method is called
        let result = notice.raw_message();

        // --------------------
        // THEN
        // --------------------
        // The contained value is as expected
        let expected = expected.raw_message();
        assert_eq!(result, expected)
    }

    // --------------------
    // RpcNotice methods
    // --------------------

    #[test]
    fn rpcnotice_message_code() {
        // --------------------
        // GIVEN
        // --------------------
        // A request message

        // Create message
        let msgtype = Value::from(MessageType::Notification.to_number());
        let msgcode = Value::from(TestCode::One.to_number());
        let msgval = Value::Array(vec![Value::from(42)]);

        let val = Value::Array(vec![msgtype, msgcode, msgval]);
        let msg = Message::from(val).unwrap();
        let expected = msg.clone();
        let notice = Notice::from(msg).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // NotificationMessage::message_id() method is called
        let result = notice.message_code();

        // --------------------
        // THEN
        // --------------------
        // The contained value is as expected
        let code = expected.message()[1].as_u64().unwrap() as u8;
        let expected = TestCode::from_number(code).unwrap();
        assert_eq!(result, expected)
    }

    #[test]
    fn rpcnotice_message_args() {
        // --------------------
        // GIVEN
        // --------------------
        // A request message

        // Create message
        let msgtype = Value::from(MessageType::Notification.to_number());
        let msgcode = Value::from(TestCode::One.to_number());
        let msgargs = Value::Array(vec![Value::from(42)]);

        let val = Value::Array(vec![msgtype, msgcode, msgargs.clone()]);
        let msg = Message::from(val).unwrap();
        let notice = Notice::from(msg).unwrap();

        let expected = msgargs.as_array().unwrap();

        // --------------------
        // WHEN
        // --------------------
        // NotificationMessage::message_id() method is called
        let result = notice.message_args();

        // --------------------
        // THEN
        // --------------------
        // The contained value is as expected
        assert_eq!(result, expected)
    }
}


// ===========================================================================
//
// ===========================================================================
