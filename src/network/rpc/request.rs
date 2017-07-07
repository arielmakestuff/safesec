// src/network/rpc/request.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

//! This module defines the Request RPC message type.
//!
//! A Request RPC message is used by a client to send an initial request to a
//! server. Based on the generic [`Message`] type, the Request message type is
//! essentially an array containing 4 items. These 4 items are:
//!
//! 1. Message Type - This will always be the Request message type. While
//!    represented as the enum variant `MessageType::Request`, the value stored
//!    in the array is actually a u8 integer.
//!
//! 2. Message ID - This is a u32 integer that is unique for the
//!    session/connection. If the message id is re-used, then the server is
//!    expected to respond with an error.
//!
//! 3. Message code - This is a u8 integer representing the specific request
//!    being made. This is analogous to the method parameter in the
//!    [`msgpack-rpc`] spec.
//!
//! 4. Message arguments - this is an array of values used to provide
//!    information needed by the server to fulfill the request specified by the
//!    message code.
//!
//! # Example
//!
//! To create a new Request object, you can create one from an existing
//! [`Message`] instance. This is used, for example, when the message is
//! deserialized by the server into a general [`Message`] object, identified as
//! a Request message, and it is required to perform Request specific
//! operations on the message.
//!
//! Alternatively, a `RequestMessage` object can be created manually via the
//! `RequestMessage::new()` method
//!
//! ```rust
//!
//! extern crate rmpv;
//! extern crate safesec;
//!
//! #[macro_use]
//! extern crate safesec_derive;
//!
//! use rmpv::Value;
//! use safesec::error::{Error, GeneralError, Result};
//! use safesec::network::rpc::message::{CodeConvert, Message, MessageType,
//!                                      RpcMessage, RpcMessageType};
//! use safesec::network::rpc::request::{RequestMessage, RpcRequest};
//!
//! // Define Request codes
//! #[derive(Debug, Clone, PartialEq, CodeConvert)]
//! enum Func {
//!     Question,
//!     Answer,
//! }
//!
//! # fn main() {
//! // Create an alias for RequestMessage
//! type Request = RequestMessage<Func>;
//!
//! // Build Message
//! let msgtype = Value::from(MessageType::Request.to_number());
//! let msgid = Value::from(42);
//! let msgcode = Value::from(Func::Question.to_number());
//! let msgargs = Value::Array(vec![Value::from(42)]);
//! let msgval = Value::Array(vec![msgtype, msgid, msgcode, msgargs]);
//! let msg = Message::from(msgval).unwrap();
//!
//! // Turn the message into a Request type
//! let req = Request::from(msg).unwrap();
//! assert_eq!(req.message_type().unwrap(), MessageType::Request);
//! assert_eq!(req.message_id(), 42);
//! assert_eq!(req.message_code(), Func::Question);
//! assert_eq!(req.message_args(), &vec![Value::from(42)]);
//!
//! // Create a brand new request from scratch
//! let new_req = Request::new(42, Func::Answer, vec![Value::from(9000)]);
//! assert_eq!(new_req.message_type().unwrap(), MessageType::Request);
//! assert_eq!(new_req.message_id(), 42);
//! assert_eq!(new_req.message_code(), Func::Answer);
//! assert_eq!(new_req.message_args(), &vec![Value::from(9000)]);
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
                              RpcMessageType, value_type};
use ::error::Error;
use ::error::network::rpc::{RpcError, RpcResult};


// ===========================================================================
// RequestMessage
// ===========================================================================


/// Trait providing Request message specific getter methods.
///
/// # Example
///
/// ```rust
/// extern crate rmpv;
/// extern crate safesec;
///
/// use rmpv::Value;
/// use safesec::network::rpc::message::{MessageType, RpcMessage,
///                                      RpcMessageType};
/// use safesec::network::rpc::request::{RequestMessage, RpcRequest};
///
/// # fn main() {
/// // Create Request alias
/// type Request = RequestMessage<MessageType>;
///
/// // Re-use MessageType as message code
/// let req = Request::new(42, MessageType::Notification,
///                        vec![Value::from(42)]);
///
/// // Check all getter methods
/// assert_eq!(req.message_type().unwrap(), MessageType::Request);
/// assert_eq!(req.message_id(), 42);
/// assert_eq!(req.message_code(), MessageType::Notification);
/// assert_eq!(req.message_args(), &vec![Value::from(42)]);
/// # }
/// ```
pub trait RpcRequest<C>: RpcMessage
    where C: CodeConvert<C>
{
    /// Return the message's ID value.
    fn message_id(&self) -> u32 {
        let msgid = &self.as_vec()[1];
        msgid.as_u64().unwrap() as u32
    }

    /// Return the message's code/method value.
    fn message_code(&self) -> C {
        let msgcode = &self.as_vec()[2];
        let msgcode = msgcode.as_u64().unwrap() as u8;
        C::from_number(msgcode).unwrap()
    }

    /// Return the message's arguments.
    fn message_args(&self) -> &Vec<Value> {
        let msgargs = &self.as_vec()[3];
        msgargs.as_array().unwrap()
    }
}


/// A representation of the Request RPC message type.
pub struct RequestMessage<C> {
    msg: Message,
    codetype: PhantomData<C>
}


impl<C> RpcMessage for RequestMessage<C>
    where C: CodeConvert<C>
{
    fn as_vec(&self) -> &Vec<Value> {
        self.msg.as_vec()
    }

    fn as_value(&self) -> &Value {
        self.msg.as_value()
    }
}


impl<C> RpcMessageType for RequestMessage<C>
    where C: CodeConvert<C>
{
    fn as_message(&self) -> &Message {
        &self.msg
    }
}


impl<C> RpcRequest<C> for RequestMessage<C> where C: CodeConvert<C> {}


impl<C> RequestMessage<C> where C: CodeConvert<C> {

    /// Create a brand new RequestMessage object.
    ///
    /// # Example
    ///
    /// ```rust
    /// extern crate rmpv;
    /// extern crate safesec;
    ///
    /// use rmpv::Value;
    /// use safesec::network::rpc::message::{MessageType, RpcMessage};
    /// use safesec::network::rpc::request::{RequestMessage, RpcRequest};
    ///
    /// # fn main() {
    /// // Create Request alias
    /// type Request = RequestMessage<MessageType>;
    ///
    /// // Re-use MessageType as message code
    /// let req = Request::new(42, MessageType::Notification,
    ///                        vec![Value::from(42)]);
    /// # }
    /// ```
    pub fn new(msgid: u32, msgcode: C, args: Vec<Value>) -> Self {
        let msgtype = Value::from(MessageType::Request as u8);
        let msgid = Value::from(msgid);
        let msgcode = Value::from(msgcode.to_number());
        let msgargs = Value::from(args);
        let msgval = Value::from(vec![msgtype, msgid, msgcode, msgargs]);

        match Message::from(msgval) {
            Ok(msg) => Self { msg: msg, codetype: PhantomData },
            Err(_) => unreachable!()
        }
    }

    /// Create a RequestMessage from a Message
    ///
    /// # Example
    ///
    /// ```rust
    /// extern crate rmpv;
    /// extern crate safesec;
    ///
    /// use rmpv::Value;
    /// use safesec::network::rpc::message::{CodeConvert, Message, MessageType, RpcMessage};
    /// use safesec::network::rpc::request::{RequestMessage, RpcRequest};
    ///
    /// # fn main() {
    /// // Create an alias for RequestMessage, re-using `MessageType` as the
    /// // message code.
    /// type Request = RequestMessage<MessageType>;
    ///
    /// // Build Message
    /// let msgtype = Value::from(MessageType::Request.to_number());
    /// let msgid = Value::from(42);
    /// let msgcode = Value::from(MessageType::Notification.to_number());
    /// let msgargs = Value::Array(vec![Value::from(9001)]);
    /// let msgval = Value::Array(vec![msgtype, msgid, msgcode, msgargs]);
    /// let msg = Message::from(msgval).unwrap();
    ///
    /// // Turn the message into a Request type
    /// let req = Request::from(msg).unwrap();
    /// # }
    /// ```
    pub fn from(msg: Message) -> RpcResult<Self> {
        {
            // Requests is always represented as an array of 4 values
            let array = msg.as_vec();
            let arraylen = array.len();
            if arraylen != 4 {
                let errmsg = format!("expected array length of 4, got {}",
                                     arraylen);
                let err = Error::new(RpcError::InvalidArrayLength, errmsg);
                return Err(err);
            }

            // Run all check functions and return the first error generated
            let funcvec: Vec<fn(&Value) -> RpcResult<()>>;
            funcvec = vec![Self::check_message_type, Self::check_message_id,
                           Self::check_message_code, Self::check_message_args];

            for (i, func) in funcvec.iter().enumerate() {
                func(&array[i])?;
            }
        }
        Ok(Self {msg: msg, codetype: PhantomData})
    }

    // Checks that the message type parameter of a Request message is valid
    //
    // This is a private method used by the public from() method
    fn check_message_type(msgtype: &Value) -> RpcResult<()> {
        let msgtype = msgtype.as_u64().unwrap() as u8;
        let expected_msgtype = MessageType::Request.to_number();
        if msgtype != expected_msgtype {
            let errmsg = format!("expected {} for message type (ie MessageType::Request), got {}",
                                 expected_msgtype, msgtype);
            let err = Error::new(RpcError::InvalidMessageType, errmsg);
            return Err(err)
        }
        Ok(())
    }

    // Checks that the message id parameter of a Request message is valid
    //
    // This is a private method used by the public from() method
    fn check_message_id(msgid: &Value) -> RpcResult<()> {
        let msgid = Self::check_int(msgid.as_u64(), u32::max_value() as u64,
                                    "u32".to_string());
        if let Err(e) = msgid {
            let err = Error::new(RpcError::InvalidIDType, e);
            return Err(err)
        }
        Ok(())
    }

    // Checks that the message code parameter of a Request message is valid
    //
    // This is a private method used by the public from() method
    fn check_message_code(msgcode: &Value) -> RpcResult<()> {
        let msgcode = Self::check_int(msgcode.as_u64(),
                                      u8::max_value() as u64, "u8".to_string());
        match msgcode {
            Err(e) => {
                let err = Error::new(RpcError::InvalidRequest, e);
                return Err(err)
            }
            Ok(v) => {
                let u8val = v as u8;
                if let Err(e) = C::from_number(u8val) {
                    let err = Error::new(RpcError::InvalidRequest, e);
                    return Err(err)
                }
            }
        }
        Ok(())
    }

    // Check that the message arguments parameter of a Request message is valid
    //
    // This is a private method used by the public from() method
    fn check_message_args(msgargs: &Value) -> RpcResult<()> {
        let args = msgargs.as_array();
        if args.is_none() {
            let errmsg = format!("expected array for request arguments but got {}",
                                 value_type(&msgargs));
            let err = Error::new(RpcError::InvalidRequestArgs, errmsg);
            return Err(err)
        }
        Ok(())
    }
}


impl<C> Into<Message> for RequestMessage<C>
    where C: CodeConvert<C>
{
    fn into(self) -> Message {
        self.msg
    }
}


impl<C> Into<Value> for RequestMessage<C>
    where C: CodeConvert<C>
{
    fn into(self) -> Value {
        let msg: Message = self.msg.into();
        msg.into()
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
    use ::error::network::rpc::{RpcError, RpcResult};
    use ::network::rpc::message::{CodeConvert, Message, MessageType, RpcMessage,
                                  value_type};
    use ::network::rpc::request::{RpcRequest, RequestMessage};

    // --------------------
    // Helpers
    // --------------------
    #[derive(Debug, PartialEq, Clone, CodeConvert)]
    enum TestEnum {
        One,
        Two,
        Three
    }

    // --------------------
    // RequestMessage::new
    // --------------------

    quickcheck! {
        fn new_messagetype_always_request(msgid: u32, code: u8, args: Vec<u8>) -> TestResult {
            if code > 2 {
                return TestResult::discard()
            }

            let msgtype = Value::from(MessageType::Request.to_number());
            let array: Vec<Value> = args.iter().map(|v| Value::from(v.clone())).collect();
            let array_copy = array.clone();

            // Build expected
            let msgargs = Value::Array(array);
            let a = vec![msgtype, Value::from(msgid), Value::from(code),
                         msgargs];
            let expected = Value::Array(a);

            // Compare RequestMessage object to expected
            let req = RequestMessage::new(msgid,
                                          TestEnum::from_number(code).unwrap(),
                                          array_copy);
            TestResult::from_bool(req.as_value() == &expected)
        }
    }

    // --------------------
    // RequestMessage::from
    // --------------------

    #[test]
    fn from_invalid_arraylen() {
        // --------------------
        // GIVEN
        // --------------------
        // Message with only 3 arguments

        // Create message
        let msgtype = Value::from(MessageType::Request.to_number());
        let msgid = Value::from(42);
        let msgcode = Value::from(TestEnum::One.to_number());
        let array: Vec<Value> = vec![msgtype, msgid, msgcode];

        let val = Value::Array(array);
        let msg = Message::from(val).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // RequestMessage::from is called with the message
        let result: RpcResult<RequestMessage<TestEnum>>;
        result = RequestMessage::from(msg);

        // --------------------
        // THEN
        // --------------------
        // Error is returned
        match result {
            Err(e) => {
                let expected = "expected array length of 4, got 3";
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
        // Message with MessageType::Notification

        // Create message
        let expected = MessageType::Notification.to_number();
        let msgtype = Value::from(expected);
        let msgid = Value::from(42);
        let msgcode = Value::from(TestEnum::One.to_number());
        let msgval = Value::from(42);

        let val = Value::Array(vec![msgtype, msgid, msgcode, msgval]);
        let msg = Message::from(val).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // RequestMessage::from is called with the message
        let result: RpcResult<RequestMessage<TestEnum>>;
        result = RequestMessage::from(msg);

        // --------------------
        // THEN
        // --------------------
        // Error is returned
        match result {
            Err(e) => {
                let expected = format!("expected {} for message type (ie MessageType::Request), got {}",
                                       MessageType::Request.to_number(), expected);
                assert_eq!(e.kind(), RpcError::InvalidMessageType);
                assert_eq!(e.description(), expected);
            },
            _ => assert!(false)
        }
    }

    #[test]
    fn from_message_id_invalid_type() {
        // --------------------
        // GIVEN
        // --------------------
        // Message with a string for message id

        // Create message
        let msgtype = Value::from(MessageType::Request.to_number());
        let msgid = Value::String(Utf8String::from("hello"));
        let msgcode = Value::from(TestEnum::One.to_number());
        let msgval = Value::from(42);

        let val = Value::Array(vec![msgtype, msgid, msgcode, msgval]);
        let msg = Message::from(val).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // RequestMessage::from is called with the message
        let result: RpcResult<RequestMessage<TestEnum>>;
        result = RequestMessage::from(msg);

        // --------------------
        // THEN
        // --------------------
        // Error is returned for the invalid message id
        match result {
            Err(e) => {
                let errmsg = "expected u32 but got None";
                assert_eq!(e.kind(), RpcError::InvalidIDType);
                assert_eq!(e.description(), errmsg);
            },
            _ => assert!(false)
        }
    }

    quickcheck! {
        fn from_message_id_invalid_value(msgid: u64) -> TestResult {
            if msgid <= u32::max_value() as u64 {
                return TestResult::discard()
            }

            // --------------------
            // GIVEN
            // --------------------
            // Message with a val > u32::max_value() for message id

            // Create message
            let msgtype = Value::from(MessageType::Request.to_number());
            let msgid = Value::from(msgid);
            let msgcode = Value::from(TestEnum::One.to_number());
            let msgval = Value::from(42);

            let val = Value::Array(vec![msgtype, msgid.clone(), msgcode, msgval]);
            let msg = Message::from(val).unwrap();

            // --------------------
            // WHEN
            // --------------------
            // RequestMessage::from is called with the message
            let result: RpcResult<RequestMessage<TestEnum>>;
            result = RequestMessage::from(msg);

            // --------------------
            // THEN
            // --------------------
            // Error is returned for the invalid message id value
            match result {
                Err(e) => {
                    let errmsg = format!("expected value <= {} but got value {}",
                                         u32::max_value().to_string(),
                                         msgid.to_string());
                    TestResult::from_bool(e.kind() == RpcError::InvalidIDType &&
                                          e.description() == errmsg)
                },
                _ => TestResult::from_bool(false)
            }
        }
    }

    #[test]
    fn from_message_code_invalid_type() {
        // --------------------
        // GIVEN
        // --------------------
        // Message with a string for message code

        // Create message
        let msgtype = Value::from(MessageType::Request.to_number());
        let msgid = Value::from(42);
        let msgcode = Value::String(Utf8String::from("hello"));
        let msgval = Value::from(42);

        let val = Value::Array(vec![msgtype, msgid, msgcode, msgval]);
        let msg = Message::from(val).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // RequestMessage::from is called with the message
        let result: RpcResult<RequestMessage<TestEnum>>;
        result = RequestMessage::from(msg);

        // --------------------
        // THEN
        // --------------------
        // Error is returned for the invalid message id
        match result {
            Err(e) => {
                let errmsg = "expected u8 but got None";
                assert_eq!(e.kind(), RpcError::InvalidRequest);
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
            let msgtype = Value::from(MessageType::Request.to_number());
            let msgid = Value::from(42);
            let msgcode = Value::from(msgcode);
            let msgval = Value::from(42);

            let val = Value::Array(vec![msgtype, msgid, msgcode.clone(), msgval]);
            let msg = Message::from(val).unwrap();

            // --------------------
            // WHEN
            // --------------------
            // RequestMessage::from is called with the message
            let result: RpcResult<RequestMessage<TestEnum>>;
            result = RequestMessage::from(msg);

            // --------------------
            // THEN
            // --------------------
            // Error is returned for the invalid message id value
            match result {
                Err(e) => {
                    let errmsg = format!("expected value <= {} but got value {}",
                                         u8::max_value().to_string(),
                                         msgcode.to_string());
                    TestResult::from_bool(e.kind() == RpcError::InvalidRequest &&
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
            let msgtype = Value::from(MessageType::Request.to_number());
            let msgid = Value::from(42);
            let msgcode = Value::from(code);
            let msgval = Value::from(42);

            let val = Value::Array(vec![msgtype, msgid, msgcode, msgval]);
            let msg = Message::from(val).unwrap();

            // --------------------
            // WHEN
            // --------------------
            // RequestMessage::from is called with the message
            let result: RpcResult<RequestMessage<TestEnum>>;
            result = RequestMessage::from(msg);

            // --------------------
            // THEN
            // --------------------
            match result {
                Err(e) => {
                    let errmsg = code.to_string();
                    TestResult::from_bool(e.kind() == RpcError::InvalidRequest &&
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
        // let expected = Marker::FixArray(0);
        let msgtype = Value::from(MessageType::Request.to_number());
        let msgid = Value::from(42);
        let msgcode = Value::from(TestEnum::One.to_number());
        let msgval = Value::from(42);

        let val = Value::Array(vec![msgtype, msgid, msgcode, msgval.clone()]);
        let msg = Message::from(val).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // RequestMessage::from is called with the message
        let result: RpcResult<RequestMessage<TestEnum>>;
        result = RequestMessage::from(msg);

        // --------------------
        // THEN
        // --------------------
        // Error is returned for the invalid message id
        match result {
            Err(e) => {
                let errmsg = format!("expected array for request arguments but got {}",
                                     value_type(&msgval));
                assert_eq!(e.kind(), RpcError::InvalidRequestArgs);
                assert_eq!(e.description(), errmsg);
            },
            _ => assert!(false)
        }
    }

    // --------------------
    // RpcMessage methods
    // --------------------

    #[test]
    fn rpcmessage_as_vec() {
        // --------------------
        // GIVEN
        // --------------------
        // A request message

        // Create message
        let msgtype = Value::from(MessageType::Request.to_number());
        let msgid = Value::from(42);
        let msgcode = Value::from(TestEnum::One.to_number());
        let msgval = Value::Array(vec![Value::from(42)]);

        let val = Value::Array(vec![msgtype, msgid, msgcode, msgval]);
        let msg = Message::from(val).unwrap();
        let expected = msg.clone();
        let req: RequestMessage<TestEnum> = RequestMessage::from(msg).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // RequestMessage::as_vec() method is called
        let result = req.as_vec();

        // --------------------
        // THEN
        // --------------------
        // The contained value is as expected
        let expected = expected.as_vec();
        assert_eq!(result, expected)
    }

    #[test]
    fn rpcmessage_as_value() {
        // --------------------
        // GIVEN
        // --------------------
        // A request message

        // Create message
        let msgtype = Value::from(MessageType::Request.to_number());
        let msgid = Value::from(42);
        let msgcode = Value::from(TestEnum::One.to_number());
        let msgval = Value::Array(vec![Value::from(42)]);

        let val = Value::Array(vec![msgtype, msgid, msgcode, msgval]);
        let msg = Message::from(val).unwrap();
        let expected = msg.clone();
        let req: RequestMessage<TestEnum> = RequestMessage::from(msg).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // RequestMessage::as_value() method is called
        let result = req.as_value();

        // --------------------
        // THEN
        // --------------------
        // The contained value is as expected
        let expected = expected.as_value();
        assert_eq!(result, expected)
    }

    // --------------------
    // RpcRequest methods
    // --------------------

    #[test]
    fn rpcrequest_message_id() {
        // --------------------
        // GIVEN
        // --------------------
        // A request message

        // Create message
        let msgtype = Value::from(MessageType::Request.to_number());
        let msgid = Value::from(42);
        let msgcode = Value::from(TestEnum::One.to_number());
        let msgval = Value::Array(vec![Value::from(42)]);

        let val = Value::Array(vec![msgtype, msgid, msgcode, msgval]);
        let msg = Message::from(val).unwrap();
        let expected = msg.clone();
        let req: RequestMessage<TestEnum> = RequestMessage::from(msg).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // RequestMessage::message_id() method is called
        let result = req.message_id();

        // --------------------
        // THEN
        // --------------------
        // The contained value is as expected
        let expected = expected.as_vec()[1].as_u64().unwrap() as u32;
        assert_eq!(result, expected)
    }

    #[test]
    fn rpcrequest_message_code() {
        // --------------------
        // GIVEN
        // --------------------
        // A request message

        // Create message
        let msgtype = Value::from(MessageType::Request.to_number());
        let msgid = Value::from(42);
        let msgcode = Value::from(TestEnum::One.to_number());
        let msgval = Value::Array(vec![Value::from(42)]);

        let val = Value::Array(vec![msgtype, msgid, msgcode, msgval]);
        let msg = Message::from(val).unwrap();
        let expected = msg.clone();
        let req: RequestMessage<TestEnum> = RequestMessage::from(msg).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // RequestMessage::message_id() method is called
        let result = req.message_code();

        // --------------------
        // THEN
        // --------------------
        // The contained value is as expected
        let code = expected.as_vec()[2].as_u64().unwrap() as u8;
        let expected = TestEnum::from_number(code).unwrap();
        assert_eq!(result, expected)
    }

    #[test]
    fn rpcrequest_message_args() {
        // --------------------
        // GIVEN
        // --------------------
        // A request message

        // Create message
        let msgtype = Value::from(MessageType::Request.to_number());
        let msgid = Value::from(42);
        let msgcode = Value::from(TestEnum::One.to_number());
        let msgval = Value::Array(vec![Value::from(42)]);

        let val = Value::Array(vec![msgtype, msgid, msgcode, msgval]);
        let msg = Message::from(val).unwrap();
        let expected = msg.clone();
        let req: RequestMessage<TestEnum> = RequestMessage::from(msg).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // RequestMessage::message_id() method is called
        let result = req.message_args();

        // --------------------
        // THEN
        // --------------------
        // The contained value is as expected
        let expected = expected.as_vec()[3].as_array().unwrap();
        assert_eq!(result, expected)
    }
}


// ===========================================================================
//
// ===========================================================================
