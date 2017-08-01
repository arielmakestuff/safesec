// src/service/state/mod.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

// ===========================================================================
// Externs
// ===========================================================================


// Stdlib externs

// Third-party externs

// Local externs


// ===========================================================================
// Modules
// ===========================================================================


mod auth;
mod boot;


// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports

use std::rc::Rc;
use std::sync::RwLock;


// Third-party imports

// Local imports

use network::rpc::{Message, NotificationMessage, RpcNotice};
use protocol::message::{ProtocolError, SessionType};
use storage::KeyFileStore;


// ===========================================================================
// StateResult
// ===========================================================================


type StateResult<T> = Result<T, ProtocolError>;


// ===========================================================================
// Boot states
// ===========================================================================


pub enum State {
    Start(Box<SessionState>),
    ProcessBootMessage(Box<SessionState>, Option<boot::BootResponse>),
    BootEnd,
    ProcessAuthMessage(Box<SessionState>, Option<auth::AuthResponse>),
    AuthEnd,
}


// ===========================================================================
// SessionState
// ===========================================================================


pub trait SessionState {
    fn change(self: Box<Self>, Message) -> StateResult<State>;
}


// ===========================================================================
// Start state
// ===========================================================================


type KeyFileDB = Rc<RwLock<KeyFileStore>>;


type SessionInfo = NotificationMessage<SessionType>;


pub struct Start {
    db: KeyFileDB,
}


impl Start {
    pub fn new(db: KeyFileDB) -> Self
    {
        Self { db: db }
    }
}


impl SessionState for Start {
    fn change(self: Box<Self>, m: Message) -> StateResult<State>
    {
        // Confirm that the message is a notification
        let notice = SessionInfo::from(m).map_err(|_| {
            ProtocolError::InvalidNotification
        })?;

        // Determine if should use boot or auth processing
        match notice.message_code() {
            SessionType::Boot => Ok(State::ProcessBootMessage(
                Box::new(boot::ProcessBootMessage::new(
                    self.db.clone(),
                )),
                None,
            )),
            SessionType::Auth => Ok(State::ProcessAuthMessage(
                Box::new(auth::ProcessAuthMessage::new(
                    self.db.clone(),
                )),
                None,
            )),
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

    use rmpv::Value;

    // Local imports

    use super::{SessionInfo, Start, State};
    use network::rpc::Message;
    use protocol::message::{BootError, ProtocolError, SessionType};
    use service::state::boot::BootResponse;
    use storage::{KeyFileResult, KeyFileStore};

    // --------------------
    // Start
    // --------------------
    #[test]
    fn start_message_notnotice()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Response message and
        // a Start state initialized with the fake KeyFileDB
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, _k: &Vec<u8>) -> bool
            {
                unimplemented!()
            }
            fn get(&self, _k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                unimplemented!()
            }
            fn set(&mut self, _k: &Vec<u8>, _file: &Vec<u8>)
                -> KeyFileResult<()>
            {
                unimplemented!()
            }
            fn delete(&mut self, _k: &Vec<u8>) -> KeyFileResult<()>
            {
                unimplemented!()
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let info = BootResponse::new(42, BootError::Nil, Value::Nil);
        let msg: Message = info.into();
        let state = State::Start(Box::new(Start::new(db)));

        // ----------------------------------------------------------
        // WHEN
        // Calling Start.change() with the request message
        // ----------------------------------------------------------
        let result = match state {
            State::Start(s) => s.change(msg),
            _ => unreachable!(),
        };

        // ----------------------------------------------------------
        // THEN
        // An ProtocolError::InvalidNotification error is returned
        // ----------------------------------------------------------
        let val = match result {
            Err(ProtocolError::InvalidNotification) => true,
            _ => false,
        };
        assert!(val);
    }

    #[test]
    fn start_boot()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Boot notification message and
        // a Start state initialized with the fake KeyFileDB
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, _k: &Vec<u8>) -> bool
            {
                unimplemented!()
            }
            fn get(&self, _k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                unimplemented!()
            }
            fn set(&mut self, _k: &Vec<u8>, _file: &Vec<u8>)
                -> KeyFileResult<()>
            {
                unimplemented!()
            }
            fn delete(&mut self, _k: &Vec<u8>) -> KeyFileResult<()>
            {
                unimplemented!()
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let args = vec![Value::Nil];
        let info = SessionInfo::new(SessionType::Boot, args);
        let msg: Message = info.into();
        let state = State::Start(Box::new(Start::new(db)));

        // ----------------------------------------------------------
        // WHEN
        // Calling Start.change() with the notification message
        // ----------------------------------------------------------
        let result = match state {
            State::Start(s) => s.change(msg),
            _ => unreachable!(),
        };

        // ----------------------------------------------------------
        // THEN
        // State::ProcessBootMessage is returned
        // ----------------------------------------------------------
        let val = match result {
            Ok(State::ProcessBootMessage(_, r)) => {
                assert!(r.is_none());
                true
            }
            _ => false,
        };
        assert!(val);
    }

    #[test]
    fn start_auth()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // an Auth notification message and
        // a Start state initialized with the fake KeyFileDB
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, _k: &Vec<u8>) -> bool
            {
                unimplemented!()
            }
            fn get(&self, _k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                unimplemented!()
            }
            fn set(&mut self, _k: &Vec<u8>, _file: &Vec<u8>)
                -> KeyFileResult<()>
            {
                unimplemented!()
            }
            fn delete(&mut self, _k: &Vec<u8>) -> KeyFileResult<()>
            {
                unimplemented!()
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let args = vec![Value::Nil];
        let info = SessionInfo::new(SessionType::Auth, args);
        let msg: Message = info.into();
        let state = State::Start(Box::new(Start::new(db)));

        // ----------------------------------------------------------
        // WHEN
        // Calling Start.change() with the notification message
        // ----------------------------------------------------------
        let result = match state {
            State::Start(s) => s.change(msg),
            _ => unreachable!(),
        };

        // ----------------------------------------------------------
        // THEN
        // State::ProcessAuthMessage is returned
        // ----------------------------------------------------------
        let val = match result {
            Ok(State::ProcessAuthMessage(_, r)) => {
                assert!(r.is_none());
                true
            }
            _ => false,
        };
        assert!(val);
    }
}


// ===========================================================================
//
// ===========================================================================
