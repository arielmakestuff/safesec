// message.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports

// Third-party imports

// Local imports

use error::{Error, GeneralError, Result};
use network::rpc::CodeConvert;


// ===========================================================================
// Protocol Errors
// ===========================================================================


pub enum ProtocolError {
    InvalidData,
    InvalidMessage,
    InvalidMessageType,

    // --------------------
    // Request
    // --------------------
    InvalidRequestID,

    // This is request code
    InvalidRequestType,
    InvalidRequestArgs,
    InvalidRequest,

    // --------------------
    // Response
    // --------------------
    InvalidResponseID,

    // This is response code
    InvalidResponseType,
    InvalidResponse,

    // --------------------
    // Notification
    // --------------------
    // This is notification code
    InvalidNotificationType,
    InvalidNotificationArgs,
    InvalidNotification,
}


// ===========================================================================
// Messages
// ===========================================================================

// Session type.
//
// Used with the notification rpc message type.
#[derive(Debug, PartialEq, Clone, CodeConvert)]
pub enum SessionType {
    // Session used when an agent is starting.
    //
    // Indicates that authentication services are likely to not be available.
    // Only the GetKeyFile request is available within a bootstrap session.
    Boot,

    // Authentication is used to allow access to keyfiles.
    //
    // All request types are available within an authenticated session.
    Auth,
}


// ===========================================================================
// Bootstrap requests
// ===========================================================================


// Used with the request rpc message type.
#[derive(Debug, PartialEq, Clone, CodeConvert)]
pub enum BootRequest {
    // Retrieve the keyfile
    GetKeyFile,
}


// Used with the response rpc message type.
#[derive(Debug, PartialEq, Clone, CodeConvert)]
pub enum BootError {
    Nil,

    // Key file is not found.
    KeyFileNotFound,
}


// ===========================================================================
// Auth requests
// ===========================================================================


// Used with the request rpc message type.
#[derive(Debug, PartialEq, Clone, CodeConvert)]
pub enum AuthRequest {
    // Retrieve the keyfile.
    //
    // Requires 1 argument: key. Only succeeds if the keyfile exists.
    GetKeyFile,

    // Create the keyfile.
    //
    // Requires 2 arguments: key, keyfile. Only succeeds if the keyfile does
    // not exist.
    CreateKeyFile,

    // Change only the keyfile
    //
    // Requires 2 arguments: key, new keyfile. Only succeeds if the keyfile
    // already exists.
    ChangeKeyFile,

    // Change only the key
    //
    // Requires 2 arguments: old key, new key. Only succeeds if the keyfile
    // already exists.
    ChangeKey,

    // Replace the keyfile
    //
    // Requires 3 arguments: Old key, new key, new keyfile. Only succeeds if
    // the keyfile already exists.
    ReplaceKeyFile,

    // Delete the keyfile.
    //
    // requires 1 argument: key. Only succeeds if the keyfile already exists.
    DeleteKeyFile,

    // Check if a key exists
    //
    // requires 1 argument: key. Always succeeds and returnes true or false.
    KeyExists,
}


// Used with the response rpc message type.
#[derive(Debug, PartialEq, Clone, CodeConvert)]
pub enum AuthError {
    Nil,

    // Key file is not found.
    KeyFileNotFound,

    // Key file exists.
    KeyFileExists,
}


// ===========================================================================
//
// ===========================================================================
