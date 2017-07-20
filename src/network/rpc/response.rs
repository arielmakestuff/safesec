// src/network/rpc/response.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

//! This module defines the Response RPC message type.
//!
//! A Response RPC message is used by a server to send a response to a client's request.
//! server. Based on the generic [`Message`] type, the Response message type is
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
//! 3. Error code - This is a u8 integer representing any errors due to the
//!    client's request. If there is no error, then this value is `0`.
//!
//! 4. Message result - this is an arbitrary value.
//!
//! # Example
//!
//! To create a new Response object, you can create one from an existing
//! [`Message`] instance. This is used, for example, when the message is
//! deserialized by the server into a general [`Message`] object, identified as
//! a Response message, and it is required to perform Response specific
//! operations on the message.
//!
//! Alternatively, a `ResponseMessage` object can be created manually via the
//! `ResponseMessage::new()` method
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
//! use safesec::network::rpc::response::{ResponseMessage, RpcResponse};
//!
//! // Define Error codes
//! #[derive(Debug, Clone, PartialEq, CodeConvert)]
//! enum RequestError {
//!     Nope,
//!     NuhUh,
//! }
//!
//! # fn main() {
//! // Create an alias for ResponseMessage
//! type Response = ResponseMessage<RequestError>;
//!
//! // Build Message
//! let msgtype = Value::from(MessageType::Response.to_number());
//! let msgid = Value::from(42);
//! let msgcode = Value::from(RequestError::Nope.to_number());
//! let msgresult = Value::from(9001);
//! let msgval = Value::Array(vec![msgtype, msgid, msgcode, msgresult]);
//! let msg = Message::from(msgval).unwrap();
//!
//! // Turn the message into a Response type
//! let res = Response::from(msg).unwrap();
//! assert_eq!(res.message_type().unwrap(), MessageType::Response);
//! assert_eq!(res.message_id(), 42);
//! assert_eq!(res.error_code(), RequestError::Nope);
//! assert_eq!(res.result(), &Value::from(9001));
//!
//! // Create a brand new response from scratch
//! let new_res = Response::new(42, RequestError::NuhUh, Value::from(9001));
//! assert_eq!(new_res.message_type().unwrap(), MessageType::Response);
//! assert_eq!(new_res.message_id(), 42);
//! assert_eq!(new_res.error_code(), RequestError::NuhUh);
//! assert_eq!(new_res.result(), &Value::from(9001));
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

use error::Error;
use error::network::rpc::{RpcError, RpcResult};
use network::rpc::message::{CodeConvert, Message, MessageType, RpcMessage,
                            RpcMessageType};


// ===========================================================================
// ResponseMessage
// ===========================================================================


/// Trait providing Response message specific getter methods.
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
/// use safesec::network::rpc::response::{ResponseMessage, RpcResponse};
///
/// # fn main() {
/// // Create Response alias
/// type Response = ResponseMessage<MessageType>;
///
/// // Re-use MessageType as error code
/// let req = Response::new(42, MessageType::Notification,
///                         Value::from(9001));
///
/// // Check all getter methods
/// assert_eq!(req.message_type().unwrap(), MessageType::Response);
/// assert_eq!(req.message_id(), 42);
/// assert_eq!(req.error_code(), MessageType::Notification);
/// assert_eq!(req.result(), &Value::from(9001));
/// # }
/// ```
pub trait RpcResponse<C>: RpcMessage
where
    C: CodeConvert<C>,
{
    fn message_id(&self) -> u32
    {
        let msgid = &self.as_vec()[1];
        msgid.as_u64().unwrap() as u32
    }

    fn error_code(&self) -> C
    {
        let errcode = &self.as_vec()[2];
        let errcode = errcode.as_u64().unwrap() as u8;
        C::from_number(errcode).unwrap()
    }

    fn result(&self) -> &Value
    {
        let msgresult = &self.as_vec()[3];
        msgresult
    }
}


/// A representation of the Response RPC message type.
pub struct ResponseMessage<C> {
    msg: Message,
    msgtype: PhantomData<C>,
}


impl<C> RpcMessage for ResponseMessage<C>
where
    C: CodeConvert<C>,
{
    fn as_vec(&self) -> &Vec<Value>
    {
        self.msg.as_vec()
    }

    fn as_value(&self) -> &Value
    {
        self.msg.as_value()
    }
}


impl<C> RpcMessageType for ResponseMessage<C>
where
    C: CodeConvert<C>,
{
    fn as_message(&self) -> &Message
    {
        &self.msg
    }
}


impl<C> RpcResponse<C> for ResponseMessage<C>
where
    C: CodeConvert<C>,
{
}


impl<C> ResponseMessage<C>
where
    C: CodeConvert<C>,
{
    /// Create a brand new ResponseMessage object.
    ///
    /// # Example
    ///
    /// ```rust
    /// extern crate rmpv;
    /// extern crate safesec;
    ///
    /// use rmpv::Value;
    /// use safesec::network::rpc::message::{MessageType, RpcMessage};
    /// use safesec::network::rpc::response::{ResponseMessage, RpcResponse};
    ///
    /// # fn main() {
    /// // Create Response alias
    /// type Response = ResponseMessage<MessageType>;
    ///
    /// // Re-use MessageType as error code
    /// let res = Response::new(42, MessageType::Notification,
    ///                         Value::from(42));
    /// # }
    /// ```
    pub fn new(msgid: u32, errcode: C, result: Value) -> Self
    {
        let msgtype = Value::from(MessageType::Response as u8);
        let msgid = Value::from(msgid);
        let errcode = Value::from(errcode.to_number());
        let msgval = Value::from(vec![msgtype, msgid, errcode, result]);

        match Message::from(msgval) {
            Ok(msg) => Self {
                msg: msg,
                msgtype: PhantomData,
            },
            Err(_) => unreachable!(),
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
    /// use safesec::network::rpc::response::{ResponseMessage, RpcResponse};
    ///
    /// # fn main() {
    /// // Create an alias for ResponseMessage, re-using `MessageType` as the
    /// // message code.
    /// type Response = ResponseMessage<MessageType>;
    ///
    /// // Build Message
    /// let msgtype = Value::from(MessageType::Response.to_number());
    /// let msgid = Value::from(42);
    /// let msgcode = Value::from(MessageType::Notification.to_number());
    /// let msgresult = Value::from(9001);
    /// let msgval = Value::Array(vec![msgtype, msgid, msgcode, msgresult]);
    /// let msg = Message::from(msgval).unwrap();
    ///
    /// // Turn the message into a Response type
    /// let res = Response::from(msg).unwrap();
    /// # }
    /// ```
    pub fn from(msg: Message) -> RpcResult<Self>
    {
        // Response is always represented as an array of 4 values
        {
            // Response is always represented as an array of 4 values
            let array = msg.as_vec();
            let arraylen = array.len();
            if arraylen != 4 {
                let errmsg =
                    format!("expected array length of 4, got {}", arraylen);
                let err = Error::new(RpcError::InvalidArrayLength, errmsg);
                return Err(err);
            }

            // Run all check functions and return the first error generated
            let funcvec: Vec<fn(&Value) -> RpcResult<()>>;
            funcvec = vec![
                Self::check_message_type,
                Self::check_message_id,
                Self::check_error_code,
            ];

            for (i, func) in funcvec.iter().enumerate() {
                func(&array[i])?;
            }
        }
        Ok(Self {
            msg: msg,
            msgtype: PhantomData,
        })
    }

    // Checks that the message type parameter of a Response message is valid
    //
    // This is a private method used by the public from() method
    fn check_message_type(msgtype: &Value) -> RpcResult<()>
    {
        let msgtype = msgtype.as_u64().unwrap() as u8;
        let expected_msgtype = MessageType::Response.to_number();
        if msgtype != expected_msgtype {
            let errmsg = format!(
                "expected {} for message type (ie \
                 MessageType::Response), got {}",
                expected_msgtype,
                msgtype
            );
            let err = Error::new(RpcError::InvalidMessageType, errmsg);
            return Err(err);
        }
        Ok(())
    }

    // Checks that the message id parameter of a Response message is valid
    //
    // This is a private method used by the public from() method
    fn check_message_id(msgid: &Value) -> RpcResult<()>
    {
        let msgid = Self::check_int(
            msgid.as_u64(),
            u32::max_value() as u64,
            "u32".to_string(),
        );
        if let Err(e) = msgid {
            let err = Error::new(RpcError::InvalidIDType, e);
            return Err(err);
        }
        Ok(())
    }

    // Checks that the error code parameter of a Response message is valid
    //
    // This is a private method used by the public from() method
    fn check_error_code(msgcode: &Value) -> RpcResult<()>
    {
        let msgcode = Self::check_int(
            msgcode.as_u64(),
            u8::max_value() as u64,
            "u8".to_string(),
        );
        match msgcode {
            Err(e) => {
                let err = Error::new(RpcError::InvalidResponse, e);
                return Err(err);
            }
            Ok(v) => {
                let u8val = v as u8;
                if let Err(e) = C::from_number(u8val) {
                    let err = Error::new(RpcError::InvalidResponse, e);
                    return Err(err);
                }
            }
        }
        Ok(())
    }
}


impl<C> Into<Message> for ResponseMessage<C>
where
    C: CodeConvert<C>,
{
    fn into(self) -> Message
    {
        self.msg
    }
}


impl<C> Into<Value> for ResponseMessage<C>
where
    C: CodeConvert<C>,
{
    fn into(self) -> Value
    {
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

    use error::{Error, GeneralError, Result};
    use error::network::rpc::RpcError;
    use network::rpc::message::{CodeConvert, Message, MessageType,
                                RpcMessage};
    use network::rpc::response::{ResponseMessage, RpcResponse};

    // --------------------
    // Helpers
    // --------------------
    #[derive(Debug, PartialEq, Clone, CodeConvert)]
    enum TestError {
        One,
        Two,
        Three,
    }

    type Response = ResponseMessage<TestError>;

    // --------------------
    // ResponseMessage::new
    // --------------------

    quickcheck! {
        fn new_messagetype_always_response(msgid: u32, err: u8) -> TestResult {
            if err > 2 {
                return TestResult::discard()
            }

            let msgtype = Value::from(MessageType::Response.to_number());

            // Build expected
            let a = vec![msgtype, Value::from(msgid), Value::from(err),
                                Value::from(42)];
            let expected = Value::Array(a);

            // Compare ResponseMessage object to expected
            let req = Response::new(msgid,
                                    TestError::from_number(err).unwrap(),
                                    Value::from(42));
            TestResult::from_bool(req.as_value() == &expected)
        }
    }

    // --------------------
    // ResponseMessage::from
    // --------------------

    #[test]
    fn from_invalid_arraylen()
    {
        // --------------------
        // GIVEN
        // --------------------
        // Message with only 3 arguments

        // Create message
        let msgtype = Value::from(MessageType::Response.to_number());
        let msgid = Value::from(42);
        let msgcode = Value::from(TestError::One.to_number());
        let array: Vec<Value> = vec![msgtype, msgid, msgcode];

        let val = Value::Array(array);
        let msg = Message::from(val).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // ResponseMessage::from is called with the message
        let result = Response::from(msg);

        // --------------------
        // THEN
        // --------------------
        // Error is returned
        match result {
            Err(e) => {
                let expected = "expected array length of 4, got 3";
                assert_eq!(e.kind(), RpcError::InvalidArrayLength);
                assert_eq!(e.description(), expected);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn from_invalid_messagetype()
    {
        // --------------------
        // GIVEN
        // --------------------
        // Message with MessageType::Notification

        // Create message
        let expected = MessageType::Notification.to_number();
        let msgtype = Value::from(expected);
        let msgid = Value::from(42);
        let msgcode = Value::from(TestError::One.to_number());
        let msgval = Value::from(42);

        let val = Value::Array(vec![msgtype, msgid, msgcode, msgval]);
        let msg = Message::from(val).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // ResponseMessage::from is called with the message
        let result = Response::from(msg);

        // --------------------
        // THEN
        // --------------------
        // Error is returned
        match result {
            Err(e) => {
                let expected = format!(
                    "expected {} for message type (ie \
                     MessageType::Response), got {}",
                    MessageType::Response.to_number(),
                    expected
                );
                assert_eq!(e.kind(), RpcError::InvalidMessageType);
                assert_eq!(e.description(), expected);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn from_message_id_invalid_type()
    {
        // --------------------
        // GIVEN
        // --------------------
        // Message with a string for message id

        // Create message
        let msgtype = Value::from(MessageType::Response.to_number());
        let msgid = Value::String(Utf8String::from("hello"));
        let msgcode = Value::from(TestError::One.to_number());
        let msgval = Value::from(42);

        let val = Value::Array(vec![msgtype, msgid, msgcode, msgval]);
        let msg = Message::from(val).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // ResponseMessage::from is called with the message
        let result = Response::from(msg);

        // --------------------
        // THEN
        // --------------------
        // Error is returned for the invalid message id
        match result {
            Err(e) => {
                let errmsg = "expected u32 but got None";
                assert_eq!(e.kind(), RpcError::InvalidIDType);
                assert_eq!(e.description(), errmsg);
            }
            _ => assert!(false),
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
            let msgtype = Value::from(MessageType::Response.to_number());
            let msgid = Value::from(msgid);
            let msgcode = Value::from(TestError::One.to_number());
            let msgval = Value::from(42);

            let val = Value::Array(vec![msgtype, msgid.clone(), msgcode, msgval]);
            let msg = Message::from(val).unwrap();

            // --------------------
            // WHEN
            // --------------------
            // ResponseMessage::from is called with the message
            let result = Response::from(msg);

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
    fn from_error_code_invalid_type()
    {
        // --------------------
        // GIVEN
        // --------------------
        // Message with a string for message code

        // Create message
        let msgtype = Value::from(MessageType::Response.to_number());
        let msgid = Value::from(42);
        let msgcode = Value::String(Utf8String::from("hello"));
        let msgval = Value::from(42);

        let val = Value::Array(vec![msgtype, msgid, msgcode, msgval]);
        let msg = Message::from(val).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // ResponseMessage::from is called with the message
        let result = Response::from(msg);

        // --------------------
        // THEN
        // --------------------
        // Error is returned for the invalid message id
        match result {
            Err(e) => {
                let errmsg = "expected u8 but got None";
                assert_eq!(e.kind(), RpcError::InvalidResponse);
                assert_eq!(e.description(), errmsg);
            }
            _ => assert!(false),
        }
    }

    quickcheck! {
        fn from_error_code_invalid_value(msgcode: u64) -> TestResult {
            if msgcode <= u8::max_value() as u64 {
                return TestResult::discard()
            }

            // --------------------
            // GIVEN
            // --------------------
            // Message with a msgcode > u8::max_value() for message code

            // Create message
            let msgtype = Value::from(MessageType::Response.to_number());
            let msgid = Value::from(42);
            let msgcode = Value::from(msgcode);
            let msgval = Value::from(42);

            let val = Value::Array(vec![msgtype, msgid, msgcode.clone(), msgval]);
            let msg = Message::from(val).unwrap();

            // --------------------
            // WHEN
            // --------------------
            // ResponseMessage::from is called with the message
            let result = Response::from(msg);

            // --------------------
            // THEN
            // --------------------
            // Error is returned for the invalid message id value
            match result {
                Err(e) => {
                    let errmsg = format!("expected value <= {} but got value {}",
                                         u8::max_value().to_string(),
                                         msgcode.to_string());
                    TestResult::from_bool(e.kind() == RpcError::InvalidResponse &&
                                          e.description() == errmsg)
                },
                _ => TestResult::from_bool(false)
            }
        }

        fn from_error_code_invalid_code(code: u8) -> TestResult {

            // --------------------
            // GIVEN
            // --------------------
            // Message with a msgcode > 2 for message code
            if code <= 2 {
                return TestResult::discard()
            }

            // Create message
            let msgtype = Value::from(MessageType::Response.to_number());
            let msgid = Value::from(42);
            let msgcode = Value::from(code);
            let msgval = Value::from(42);

            let val = Value::Array(vec![msgtype, msgid, msgcode, msgval]);
            let msg = Message::from(val).unwrap();

            // --------------------
            // WHEN
            // --------------------
            // ResponseMessage::from is called with the message
            let result = Response::from(msg);

            // --------------------
            // THEN
            // --------------------
            match result {
                Err(e) => {
                    let errmsg = code.to_string();
                    TestResult::from_bool(e.kind() == RpcError::InvalidResponse &&
                                          e.description() == errmsg)
                },
                _ => TestResult::from_bool(false)
            }
        }
    }

    // --------------------
    // RpcMessage methods
    // --------------------

    #[test]
    fn rpcmessage_as_vec()
    {
        // --------------------
        // GIVEN
        // --------------------
        // A response message

        // Create message
        let msgtype = Value::from(MessageType::Response.to_number());
        let msgid = Value::from(42);
        let msgcode = Value::from(TestError::One.to_number());
        let msgval = Value::Array(vec![Value::from(42)]);

        let val = Value::Array(vec![msgtype, msgid, msgcode, msgval]);
        let msg = Message::from(val).unwrap();
        let expected = msg.clone();
        let res = Response::from(msg).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // ResponseMessage::as_vec() method is called
        let result = res.as_vec();

        // --------------------
        // THEN
        // --------------------
        // The contained value is as expected
        let expected = expected.as_vec();
        assert_eq!(result, expected)
    }

    #[test]
    fn rpcmessage_as_value()
    {
        // --------------------
        // GIVEN
        // --------------------
        // A response message

        // Create message
        let msgtype = Value::from(MessageType::Response.to_number());
        let msgid = Value::from(42);
        let msgcode = Value::from(TestError::One.to_number());
        let msgval = Value::Array(vec![Value::from(42)]);

        let val = Value::Array(vec![msgtype, msgid, msgcode, msgval]);
        let msg = Message::from(val).unwrap();
        let expected = msg.clone();
        let res = Response::from(msg).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // ResponseMessage::as_value() method is called
        let result = res.as_value();

        // --------------------
        // THEN
        // --------------------
        // The contained value is as expected
        let expected = expected.as_value();
        assert_eq!(result, expected)
    }

    // --------------------
    // RpcResponse methods
    // --------------------

    #[test]
    fn rpcresponse_message_id()
    {
        // --------------------
        // GIVEN
        // --------------------
        // A response message

        // Create message
        let msgtype = Value::from(MessageType::Response.to_number());
        let msgid = Value::from(42);
        let msgcode = Value::from(TestError::One.to_number());
        let msgval = Value::Array(vec![Value::from(42)]);

        let val = Value::Array(vec![msgtype, msgid, msgcode, msgval]);
        let msg = Message::from(val).unwrap();
        let expected = msg.clone();
        let res = Response::from(msg).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // ResponseMessage::message_id() method is called
        let result = res.message_id();

        // --------------------
        // THEN
        // --------------------
        // The contained value is as expected
        let expected = expected.as_vec()[1].as_u64().unwrap() as u32;
        assert_eq!(result, expected)
    }

    #[test]
    fn rpcresponse_error_code()
    {
        // --------------------
        // GIVEN
        // --------------------
        // A response message

        // Create message
        let msgtype = Value::from(MessageType::Response.to_number());
        let msgid = Value::from(42);
        let msgcode = Value::from(TestError::One.to_number());
        let msgval = Value::Array(vec![Value::from(42)]);

        let val = Value::Array(vec![msgtype, msgid, msgcode, msgval]);
        let msg = Message::from(val).unwrap();
        let expected = msg.clone();
        let res = Response::from(msg).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // ResponseMessage::error_code() method is called
        let result = res.error_code();

        // --------------------
        // THEN
        // --------------------
        // The contained value is as expected
        let code = expected.as_vec()[2].as_u64().unwrap() as u8;
        let expected = TestError::from_number(code).unwrap();
        assert_eq!(result, expected)
    }

    #[test]
    fn rpcresponse_result()
    {
        // --------------------
        // GIVEN
        // --------------------
        // A response message

        // Create message
        let msgtype = Value::from(MessageType::Response.to_number());
        let msgid = Value::from(42);
        let errcode = Value::from(TestError::One.to_number());
        let msgresult = Value::from(42);

        let val = Value::Array(vec![msgtype, msgid, errcode, msgresult]);
        let msg = Message::from(val).unwrap();
        let expected = msg.clone();
        let res = Response::from(msg).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // ResponseMessage::result() method is called
        let result = res.result();

        // --------------------
        // THEN
        // --------------------
        // The contained value is as expected
        let expected = &expected.as_vec()[3];
        assert_eq!(result, expected)
    }
}


// ===========================================================================
//
// ===========================================================================
