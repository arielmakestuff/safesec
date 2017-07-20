// src/error/network.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports

// Third-party imports

// Local imports


// ===========================================================================
// Modules
// ===========================================================================


pub mod rpc {

    // Stdlib imports

    use std::fmt;
    use std::result;

    // Third-party imports

    // Local imports

    use error::{Error, ErrorMessage};

    pub type RpcResult<T> = result::Result<T, Error<RpcError>>;

    // RpcError
    #[derive(Debug, Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub enum RpcError {
        InvalidMessage,
        InvalidArrayLength,
        InvalidMessageType,
        InvalidIDType,
        InvalidRequest,
        InvalidRequestType,
        InvalidResponse,
        InvalidResponseType,
        InvalidNotification,
        InvalidNotificationType,
        InvalidRequestArgs,
        InvalidNotificationArgs,
    }

    impl ErrorMessage for RpcError {
        fn message(&self) -> &'static str
        {
            match *self {
                RpcError::InvalidMessage => "Invalid message",
                RpcError::InvalidArrayLength => {
                    "Invalid message array length"
                }
                RpcError::InvalidMessageType => "Invalid message type",
                RpcError::InvalidIDType => "Invalid message id type",
                RpcError::InvalidRequest => "Invalid request message",
                RpcError::InvalidRequestType => "Invalid request type",
                RpcError::InvalidResponse => "Invalid response message",
                RpcError::InvalidResponseType => "Invalid response type",
                RpcError::InvalidNotification => {
                    "Invalid notification message"
                }
                RpcError::InvalidNotificationType => {
                    "Invalid notification type"
                }
                RpcError::InvalidRequestArgs => "Invalid request arguments",
                RpcError::InvalidNotificationArgs => {
                    "Invalid notification arguments"
                }
            }
        }
    }

    impl fmt::Display for RpcError {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result
        {
            write!(fmt, "{}", self.message().to_string())
        }
    }
}


// ===========================================================================
//
// ===========================================================================
