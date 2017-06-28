// src/network/rpc/mod.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

//! Types and traits for working with a type of MessagePack RPC
//!
//! The [`Message`] type is the core underlying type that wraps around the
//! [`rmpv::Value`] type. It ensures that the given [`rmpv::Value`] object
//! conforms with the expected RPC spec.
//!
//! [`Message`]: message/struct.Message.html
//! [`rmpv::Value`]: https://docs.rs/rmpv/0.4.0/rmpv/enum.Value.html
//!
//! # MessagePack RPC
//!
//! The [`msgpack-rpc`] spec is mostly followed with a single exception: the
//! method argument of Request and Notification messages is not a string but
//! instead an integer. Since one goal in safesec is to ensure that all public
//! interfaces have strict type and value validation, an integer that could be
//! mapped to a C-style enum made better sense that using an arbitrary string.
//!
//! [`msgpack-rpc`]: https://github.com/msgpack-rpc/msgpack-rpc/blob/master/spec.md

// ===========================================================================
// Modules
// ===========================================================================


pub mod message;
// pub mod notify;
pub mod request;
pub mod response;


// ===========================================================================
// Exports
// ===========================================================================


pub use ::network::rpc::message::{CodeConvert, Message, MessageType,
                                  RpcMessage};
// pub use self::request::{RequestMessage, ReqMsgOp};
// pub use self::response::{ResponseMessage, RespMsgOp};
// pub use self::notify::{NotifyMessage, NotifyMsgOp};


// ===========================================================================
//
// ===========================================================================
