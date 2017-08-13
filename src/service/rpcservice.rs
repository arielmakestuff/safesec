// src/service/stream.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports

use std::cell::Cell;
use std::io;

// Third-party imports

use futures::{BoxFuture, Future, future};
use futures::sync::mpsc;
use rmpv::Value;
use tokio_core::reactor::Handle;
use tokio_service::Service;

// Local imports

use network::rpc::Message;
use network::server::{ServerMessage, shutdown};
use service::state::{KeyFileDB, Start, State};


// ===========================================================================
// Traits
// ===========================================================================


pub trait ServiceWithShutdown<T> {
    fn set_server_control(&mut self, mpsc::Sender<T>, Handle);
    fn server_control(&self) -> Option<(Handle, mpsc::Sender<T>)>;
    fn shutdown(&self);
}


// ===========================================================================
// RpcService
// ===========================================================================


pub struct RpcService<T> {
    control: Option<(Handle, mpsc::Sender<T>)>,
}


impl RpcService<ServerMessage> {
    pub fn new() -> Self
    {
        Self { control: None }
    }
}


impl Service for RpcService<ServerMessage> {
    type Request = Value;
    type Response = Option<Value>;
    type Error = io::Error;
    type Future = BoxFuture<Option<Value>, io::Error>;

    fn call(&self, val: Self::Request) -> Self::Future
    {
        // Convert Value into a Message
        match Message::from(val) {

            // Immediately shutdown silently if received an invalid message
            Err(_) => {
                self.shutdown();
                future::ok::<Option<Value>, io::Error>(None).boxed()
            }

            // Return the message
            Ok(m) => {
                future::ok::<Option<Value>, io::Error>(Some(m.into())).boxed()
            }
        }
    }
}


impl ServiceWithShutdown<ServerMessage> for RpcService<ServerMessage> {
    fn set_server_control(&mut self, s: mpsc::Sender<ServerMessage>, loop_handle: Handle)
    {
        self.control = Some((loop_handle, s));
    }

    fn server_control(&self)
        -> Option<(Handle, mpsc::Sender<ServerMessage>)>
    {
        if let Some((ref h, ref tx)) = self.control {
            Some((h.clone(), tx.clone()))
        } else {
            None
        }
    }

    fn shutdown(&self)
    {
        // Request shutdown
        let control = self.server_control();
        if let Some((h, tx)) = control {
            shutdown(&h, tx);
        }
    }
}



// ===========================================================================
// RpcState
// ===========================================================================


pub struct RpcState<T> {
    control: Option<(Handle, mpsc::Sender<T>)>,
    state: Cell<State>,
}


impl RpcState<ServerMessage> {
    pub fn new(db: KeyFileDB) -> Self
    {
        Self {
            control: None,
            state: Cell::new(State::Start(Box::new(Start::new(db)))),
        }
    }

    pub fn process_message(&mut self, msg: Message)
        -> BoxFuture<Option<Value>, io::Error>
    {
        // Change state
        let state = self.state.replace(State::Nil);
        let ret = match state {
            State::Nil | State::BootEnd | State::AuthEnd => unreachable!(),
            State::Start(s) => {
                match s.change(msg) {
                    Ok(newstate) => self.state.set(newstate),
                    Err(_) => self.shutdown(),
                }
                None
            }
            State::ProcessBootMessage(s, _) => {
                match s.change(msg) {
                    Ok(State::ProcessBootMessage(s, Some(resp))) => {
                        let newstate = State::ProcessBootMessage(s, None);
                        self.state.set(newstate);
                        let msg: Message = resp.into();
                        let val: Value = msg.into();
                        Some(val)
                    }
                    Ok(State::BootEnd) |
                    Err(_) => {
                        self.shutdown();
                        None
                    }
                    Ok(_) => unreachable!(),
                }
            }
            State::ProcessAuthMessage(s, _) => {
                match s.change(msg) {
                    Ok(State::ProcessAuthMessage(s, Some(resp))) => {
                        let newstate = State::ProcessAuthMessage(s, None);
                        self.state.set(newstate);
                        let msg: Message = resp.into();
                        let val: Value = msg.into();
                        Some(val)
                    }
                    Ok(State::AuthEnd) |
                    Err(_) => {
                        self.shutdown();
                        None
                    }
                    Ok(_) => unreachable!(),
                }
            }
        };
        future::ok::<Option<Value>, io::Error>(ret).boxed()
    }
}


impl ServiceWithShutdown<ServerMessage> for RpcState<ServerMessage> {
    fn set_server_control(&mut self, s: mpsc::Sender<ServerMessage>, loop_handle: Handle)
    {
        self.control = Some((loop_handle, s));
    }

    fn server_control(&self)
        -> Option<(Handle, mpsc::Sender<ServerMessage>)>
    {
        if let Some((ref h, ref tx)) = self.control {
            Some((h.clone(), tx.clone()))
        } else {
            None
        }
    }

    fn shutdown(&self)
    {
        // Request shutdown
        let control = self.server_control();
        if let Some((h, tx)) = control {
            shutdown(&h, tx);
        }
    }
}


// ===========================================================================
// Tests
// ===========================================================================


#[cfg(test)]
mod tests {
    // Stdlib imports

    use std::rc::Rc;
    use std::sync::RwLock;

    // Third-party imports

    use futures::Async;
    use rmpv::Value;

    // Local imports

    use network::rpc::{Message, RpcResponse};
    use network::server::ServerMessage;
    use protocol::message::{AuthError, AuthMessage, AuthNotice, BootError,
                            BootMessage, BootNotice, SessionType};
    use service::rpcservice::RpcState;
    use service::state::{SessionInfo, State};
    use service::state::auth::{AuthInfo, AuthRequest, AuthResponse};
    use service::state::boot::{BootInfo, BootRequest, BootResponse};
    use storage::{KeyFileResult, KeyFileStore};

    type CustomService = RpcState<ServerMessage>;

    #[test]
    fn rpcstate_process_message_startboot()
    {
        // -----------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a SessionInfo message with code SessionType::Boot and
        // a BootRequest message with code BootRequest::KeyExists and
        // a BootInfo message with code BootNotice::Done and
        // an RpcState<ServerMessage> instance
        // ----------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, _k: &Vec<u8>) -> bool
            {
                true
            }
            fn get(&self, _k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                unreachable!()
            }
            fn set(&mut self, _k: &Vec<u8>, _file: &Vec<u8>)
                -> KeyFileResult<()>
            {
                unreachable!()
            }
            fn delete(&mut self, _k: &Vec<u8>) -> KeyFileResult<()>
            {
                unreachable!()
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let key = "42".to_string().into_bytes();
        let mut messages: Vec<Message> =
            vec![
                SessionInfo::new(SessionType::Boot, vec![Value::Nil]).into(),
                BootRequest::new(
                    42,
                    BootMessage::KeyExists,
                    vec![Value::from(key)]
                ).into(),
                BootInfo::new(BootNotice::Done, vec![Value::Nil]).into(),
            ];
        let mut service: CustomService = RpcState::new(db);

        // ----------------------------------------------
        // WHEN
        // RpcState.process_message() is called
        // with each message in sequence
        // ----------------------------------------------
        let mut result: Vec<Option<Value>> = Vec::new();
        for _ in 0..messages.len() {
            let msg = messages.remove(0);
            let mut f = service.process_message(msg);
            match f.poll() {
                Ok(Async::Ready(t)) => result.push(t),
                _ => unreachable!(),
            }
        }

        // ------------------------------------------------------------------
        // THEN
        // the result is None,
        // Some(BootResponse(42, BootError::Nil, Value::Boolean(true))), None
        // and service state is State::Nil
        // ------------------------------------------------------------------
        // Third result
        assert_eq!(result.pop().unwrap(), None);

        // Second result is a BootResponse message
        let val = result.pop().unwrap(); // This is Some(Value)
        let msg = Message::from(val.unwrap()).unwrap();
        let resp = BootResponse::from(msg).unwrap();
        assert_eq!(resp.message_id(), 42);
        assert_eq!(resp.error_code(), BootError::Nil);
        assert_eq!(resp.result(), &Value::Boolean(true));

        // First result
        assert_eq!(result.pop().unwrap(), None);

        // Service state is State::Nil
        match *service.state.get_mut() {
            State::Nil => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn rpcstate_process_message_startauth()
    {
        // -----------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a SessionInfo message with code SessionType::Auth and
        // a AuthRequest message with code AuthRequest::KeyExists and
        // a AuthInfo message with code AuthNotice::Done and
        // an RpcState<ServerMessage> instance
        // ----------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, _k: &Vec<u8>) -> bool
            {
                true
            }
            fn get(&self, _k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                unreachable!()
            }
            fn set(&mut self, _k: &Vec<u8>, _file: &Vec<u8>)
                -> KeyFileResult<()>
            {
                unreachable!()
            }
            fn delete(&mut self, _k: &Vec<u8>) -> KeyFileResult<()>
            {
                unreachable!()
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let key = "42".to_string().into_bytes();
        let mut messages: Vec<Message> =
            vec![
                SessionInfo::new(SessionType::Auth, vec![Value::Nil]).into(),
                AuthRequest::new(
                    42,
                    AuthMessage::KeyExists,
                    vec![Value::from(key)]
                ).into(),
                AuthInfo::new(AuthNotice::Done, vec![Value::Nil]).into(),
            ];
        let mut service: CustomService = RpcState::new(db);

        // ----------------------------------------------
        // WHEN
        // RpcState.process_message() is called
        // with each message in sequence
        // ----------------------------------------------
        let mut result: Vec<Option<Value>> = Vec::new();
        for _ in 0..messages.len() {
            let msg = messages.remove(0);
            let mut f = service.process_message(msg);
            match f.poll() {
                Ok(Async::Ready(t)) => result.push(t),
                _ => unreachable!(),
            }
        }

        // ------------------------------------------------------------------
        // THEN
        // the result is None,
        // Some(AuthResponse(42, AuthError::Nil, Value::Boolean(true))), None
        // and service state is State::Nil
        // ------------------------------------------------------------------
        // Third result
        assert_eq!(result.pop().unwrap(), None);

        // Second result is a AuthResponse message
        let val = result.pop().unwrap(); // This is Some(Value)
        let msg = Message::from(val.unwrap()).unwrap();
        let resp = AuthResponse::from(msg).unwrap();
        assert_eq!(resp.message_id(), 42);
        assert_eq!(resp.error_code(), AuthError::Nil);
        assert_eq!(resp.result(), &Value::Boolean(true));

        // First result
        assert_eq!(result.pop().unwrap(), None);

        // Service state is State::Nil
        match *service.state.get_mut() {
            State::Nil => assert!(true),
            _ => assert!(false),
        }
    }
}


// ===========================================================================
//
// ===========================================================================
