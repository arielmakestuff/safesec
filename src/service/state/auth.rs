// src/service/state/auth.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

// ===========================================================================
// Externs
// ===========================================================================


// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports

// Third-party imports

use rmpv::Value;

// Local imports

use super::{KeyFileDB, SessionState, State, StateResult};
use network::rpc::{Message, MessageType, NotificationMessage,
                   RequestMessage, ResponseMessage, RpcMessage, RpcNotice,
                   RpcRequest};
use protocol::message::{AuthError, AuthMessage, AuthNotice, ProtocolError};
use storage::KeyFileError;


// ===========================================================================
// Auth messages
// ===========================================================================


pub type AuthRequest = RequestMessage<AuthMessage>;


pub type AuthResponse = ResponseMessage<AuthError>;


pub type AuthInfo = NotificationMessage<AuthNotice>;


// ===========================================================================
// ProcessAuthMessage
// ===========================================================================


pub struct ProcessAuthMessage {
    db: KeyFileDB,
}


impl ProcessAuthMessage {
    pub fn new(db: KeyFileDB) -> Self
    {
        Self { db: db }
    }
}


impl SessionState for ProcessAuthMessage {
    fn change(self: Box<Self>, m: Message) -> StateResult<State>
    {
        match m.message_type().unwrap() {

            // If the message is a request, process as an AuthMethod and change
            // state back to ProcessAuthMessage
            MessageType::Request => {
                let response = ProcessAuthRequest.run(self.db.clone(), m)?;
                Ok(State::ProcessAuthMessage(
                    Box::new(Self { db: self.db }),
                    Some(response),
                ))
            }

            // If the message is a done notification, change state to BootEnd
            MessageType::Notification => {
                let notice = AuthInfo::from(m).map_err(|_| {
                    ProtocolError::InvalidNotification
                })?;
                match notice.message_code() {
                    AuthNotice::Done => Ok(State::AuthEnd),
                }
            }

            // If the message is a response, return an error
            MessageType::Response => Err(ProtocolError::UnexpectedMessage),
        }
    }
}


struct ProcessAuthRequest;


impl ProcessAuthRequest {
    fn run(&self, db: KeyFileDB, m: Message) -> StateResult<AuthResponse>
    {
        let req = AuthRequest::from(m).unwrap();
        match req.message_code() {
            AuthMessage::KeyExists => self.req_key_exists(req, db),
            AuthMessage::GetKeyFile => self.req_get_keyfile(req, db),
            AuthMessage::CreateKeyFile => self.req_create_keyfile(req, db),
            AuthMessage::ChangeKeyFile => self.req_change_keyfile(req, db),
            AuthMessage::DeleteKeyFile => self.req_del_keyfile(req, db),
            AuthMessage::ChangeKey => self.req_change_key(req, db),
            AuthMessage::ReplaceKeyFile => self.req_replace_keyfile(req, db),
        }
    }

    fn _check_message(&self, req: &AuthRequest, numargs: usize)
        -> StateResult<Vec<Vec<u8>>>
    {
        // Get message arguments
        let args = req.message_args();

        // Must only have a single argument
        if args.len() != numargs {
            return Err(ProtocolError::InvalidRequestArgs);
        }

        // All arguments must be binary data
        let mut ret: Vec<Vec<u8>> = Vec::new();
        for i in 0..numargs {
            let val = &args[i];
            if !val.is_bin() {
                return Err(ProtocolError::InvalidRequest);
            }
            let a: Vec<u8> = Vec::from(val.as_slice().unwrap());
            ret.push(a);
        }
        Ok(ret)
    }

    fn req_key_exists(&self, req: AuthRequest, db: KeyFileDB)
        -> StateResult<AuthResponse>
    {
        // Get key
        let key = &self._check_message(&req, 1)?[0];

        // Get result, dropping the db lock as soon as possible
        let result = {
            let db = db.read().unwrap();
            Value::Boolean(db.exists(key))
        };

        // Create response
        let response =
            AuthResponse::new(req.message_id(), AuthError::Nil, result);
        Ok(response)
    }

    fn req_get_keyfile(&self, req: AuthRequest, db: KeyFileDB)
        -> StateResult<AuthResponse>
    {
        // Get key
        let key = &self._check_message(&req, 1)?[0];

        // Get keyfile, dropping the db lock as soon as possible
        let keyfile = {
            let db = db.read().unwrap();
            db.get(key)
        };

        match keyfile {
            // Create response
            Ok(f) => {
                let response = AuthResponse::new(
                    req.message_id(),
                    AuthError::Nil,
                    Value::from(f),
                );
                Ok(response)
            }

            // Create error response
            Err(KeyFileError::Key(k)) => {
                let response = AuthResponse::new(
                    req.message_id(),
                    AuthError::KeyFileNotFound,
                    Value::from(k),
                );
                Ok(response)
            }

            // TODO: handle other errors that may be raised (eg from lmdb backend)
            Err(KeyFileError::Other) => unimplemented!(),
        }
    }

    fn req_create_keyfile(&self, req: AuthRequest, db: KeyFileDB)
        -> StateResult<AuthResponse>
    {
        // Get args
        let args = self._check_message(&req, 2)?;
        let key = &args[0];
        let keyfile = &args[1];

        {
            let mut db = db.write().unwrap();

            // Return an error if keyfile exists
            if db.exists(key) {
                let response = AuthResponse::new(
                    req.message_id(),
                    AuthError::KeyFileExists,
                    Value::from(&key[..]),
                );
                return Ok(response);
            }

            // Create keyfile
            match db.set(key, keyfile) {
                Ok(_) => {
                    let response = AuthResponse::new(
                        req.message_id(),
                        AuthError::Nil,
                        Value::Boolean(true),
                    );
                    Ok(response)
                }
                // Create error response
                Err(KeyFileError::Other) => {
                    let response = AuthResponse::new(
                        req.message_id(),
                        AuthError::DatabaseError,
                        Value::Boolean(false),
                    );
                    Ok(response)
                }
                Err(_) => unreachable!(),
            }
        }
    }

    fn req_change_keyfile(&self, req: AuthRequest, db: KeyFileDB)
        -> StateResult<AuthResponse>
    {
        // Get args
        let args = self._check_message(&req, 2)?;
        let key = &args[0];
        let new_keyfile = &args[1];

        {
            let mut db = db.write().unwrap();

            // Return an error if key does not exist
            if !db.exists(key) {
                let response = AuthResponse::new(
                    req.message_id(),
                    AuthError::KeyFileNotFound,
                    Value::from(&key[..]),
                );
                return Ok(response);
            }

            // Change keyfile
            match db.set(key, new_keyfile) {
                Ok(_) => {
                    let response = AuthResponse::new(
                        req.message_id(),
                        AuthError::Nil,
                        Value::Boolean(true),
                    );
                    Ok(response)
                }
                // Create error response
                Err(KeyFileError::Other) => {
                    let response = AuthResponse::new(
                        req.message_id(),
                        AuthError::DatabaseError,
                        Value::Boolean(false),
                    );
                    Ok(response)
                }
                Err(_) => unreachable!(),
            }
        }
    }

    fn req_del_keyfile(&self, req: AuthRequest, db: KeyFileDB)
        -> StateResult<AuthResponse>
    {
        // Get args
        let args = self._check_message(&req, 1)?;
        let key = &args[0];

        {
            let mut db = db.write().unwrap();

            // Return an error if key does not exist
            if !db.exists(key) {
                let response = AuthResponse::new(
                    req.message_id(),
                    AuthError::KeyFileNotFound,
                    Value::from(&key[..]),
                );
                return Ok(response);
            }

            // Delete keyfile
            match db.delete(key) {
                Ok(()) => {
                    let response = AuthResponse::new(
                        req.message_id(),
                        AuthError::Nil,
                        Value::Boolean(true),
                    );
                    Ok(response)
                }
                // Create error response
                Err(KeyFileError::Other) => {
                    let response = AuthResponse::new(
                        req.message_id(),
                        AuthError::DatabaseError,
                        Value::Boolean(false),
                    );
                    Ok(response)
                }
                Err(_) => unreachable!(),
            }
        }
    }

    fn req_change_key(&self, req: AuthRequest, db: KeyFileDB)
        -> StateResult<AuthResponse>
    {
        // Get args
        let args = self._check_message(&req, 2)?;
        let oldkey = &args[0];
        let newkey = &args[1];
        let mkresponse = |code: AuthError, val: Value| {
            let response = AuthResponse::new(req.message_id(), code, val);
            Ok(response)
        };

        // Get exclusive lock to database
        let mut db = db.write().unwrap();

        // Return error response if newkey already exists
        if db.exists(newkey) {
            return mkresponse(
                AuthError::KeyFileExists,
                Value::from(&newkey[..]),
            );
        }

        let keyfile: Vec<u8> = match db.get(oldkey) {
            // Get keyfile for oldkey
            Ok(kf) => kf,

            // Return an error response if oldkey does not exist
            Err(KeyFileError::Key(k)) => {
                return mkresponse(AuthError::KeyFileNotFound, Value::from(k))
            }

            // Any other error is a db error response
            Err(KeyFileError::Other) => {
                return mkresponse(
                    AuthError::DatabaseError,
                    Value::Boolean(false),
                )
            }
        };

        // Delete the oldkey, send error response on db error
        let result = db.delete(oldkey);
        if let Err(KeyFileError::Other) = result {
            return mkresponse(
                AuthError::DatabaseError,
                Value::Boolean(false),
            );
        } else if result.is_err() {
            unreachable!()
        }

        // Re-add the keyfile with the new key
        match db.set(newkey, &keyfile) {
            Ok(()) => {
                mkresponse(AuthError::Nil, Value::Boolean(true))
            }
            // Create error response
            Err(KeyFileError::Other) => {
                mkresponse(AuthError::DatabaseError, Value::Boolean(false))
            }
            Err(_) => unreachable!(),
        }
    }

    fn req_replace_keyfile(&self, req: AuthRequest, db: KeyFileDB)
        -> StateResult<AuthResponse>
    {
        // Get args
        let args = self._check_message(&req, 3)?;
        let oldkey = &args[0];
        let newkey = &args[1];
        let newkeyfile = &args[2];
        let mkresponse = |code: AuthError, val: Value| {
            let response = AuthResponse::new(req.message_id(), code, val);
            Ok(response)
        };

        // Get exclusive lock to database
        let mut db = db.write().unwrap();

        // Return error response if newkey already exists
        if db.exists(newkey) {
            return mkresponse(
                AuthError::KeyFileExists,
                Value::from(&newkey[..]),
            );
        }

        // Delete oldkey, return error response if oldkey doesn't exist
        match db.delete(oldkey) {
            Err(KeyFileError::Other) => {
                return mkresponse(
                    AuthError::DatabaseError,
                    Value::Boolean(false),
                )
            }
            Err(KeyFileError::Key(k)) => {
                return mkresponse(AuthError::KeyFileNotFound, Value::from(k))
            }
            Ok(()) => {}
        }

        // Add the new keyfile with the new key
        match db.set(newkey, &newkeyfile) {
            Ok(()) => {
                mkresponse(AuthError::Nil, Value::Boolean(true))
            }
            // Create error response
            Err(KeyFileError::Other) => {
                mkresponse(AuthError::DatabaseError, Value::Boolean(false))
            }
            Err(_) => unreachable!(),
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

    use quickcheck::TestResult;
    use rmpv::Value;

    // Local imports

    use super::{AuthInfo, AuthRequest, AuthResponse, ProcessAuthMessage,
                ProcessAuthRequest};
    use error::{Error, GeneralError, Result};
    use network::rpc::{CodeConvert, Message, NotificationMessage,
                       RpcResponse};
    use protocol::message::{AuthError, AuthMessage, AuthNotice,
                            ProtocolError};
    use service::state::{SessionState, State};
    use storage::{KeyFileError, KeyFileResult, KeyFileStore};

    // --------------------
    // ProcessAuthMessage
    // --------------------

    #[test]
    fn processauthmessage_request_error()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with 2 arguments and
        // the request code is AuthMessage::KeyExists and
        // the first message argument is a key that does not exist and
        // the second message argument is Nil and
        // a ProcessAuthMessage instance initialized with the fake KeyFileDB
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let expected = "ANSWER".to_string().into_bytes();
                &expected == k
            }
            fn get(&self, k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                let expected = "ANSWER".to_string().into_bytes();
                if &expected == k {
                    Ok("42".to_string().into_bytes())
                } else {
                    unreachable!()
                }
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

        let key = "noanswer".to_string().into_bytes();
        let args = vec![Value::from(key), Value::Nil];
        let req = AuthRequest::new(42, AuthMessage::GetKeyFile, args);
        let msg: Message = req.into();
        let process_msg = Box::new(ProcessAuthMessage::new(db));

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthMessage.change() with the request
        // ----------------------------------------------------------
        let result = process_msg.change(msg);

        // ----------------------------------------------------------
        // THEN
        // An error is returned
        // ----------------------------------------------------------
        let val = match result {
            Err(ProtocolError::InvalidRequestArgs) => true,
            _ => false,
        };
        assert!(val);
    }

    #[test]
    fn processauthmessage_response_any()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Response message and
        // a ProcessAuthMessage instance initialized with the fake KeyFileDB
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

        let info = AuthResponse::new(42, AuthError::Nil, Value::Nil);
        let msg: Message = info.into();
        let process_msg = Box::new(ProcessAuthMessage::new(db));

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthMessage.change() with the message
        // ----------------------------------------------------------
        let result = process_msg.change(msg);

        // ----------------------------------------------------------
        // THEN
        // An ProtocolError::UnexpectedMessage error is returned
        // ----------------------------------------------------------
        let val = match result {
            Err(ProtocolError::UnexpectedMessage) => true,
            _ => false,
        };
        assert!(val);
    }

    #[test]
    fn processauthmessage_request_response()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with a single argument and
        // the request code is AuthMessage::KeyExists and
        // the message argument is a key that does not exist and
        // a ProcessAuthMessage instance initialized with the fake KeyFileDB
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let expected = "ANSWER".to_string().into_bytes();
                &expected == k
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

        let key = "ANSWER".to_string().into_bytes();
        let args = vec![Value::from(key)];
        let req = AuthRequest::new(42, AuthMessage::KeyExists, args);
        let msg: Message = req.into();
        let process_msg = Box::new(ProcessAuthMessage::new(db));

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthMessage.change() with the request
        // ----------------------------------------------------------
        let result = process_msg.change(msg);

        // ----------------------------------------------------------
        // THEN
        // A new ProcessAuthMessage state is returned with a response
        // ----------------------------------------------------------
        let val = match result {
            Ok(State::ProcessAuthMessage(_state, Some(response))) => {
                assert_eq!(response.message_id(), 42);
                assert_eq!(response.error_code(), AuthError::Nil);
                let expected = Value::Boolean(true);
                assert_eq!(response.result(), &expected);
                true
            }
            _ => false,
        };
        assert!(val);
    }

    #[test]
    fn processauthmessage_notice_valid()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Notification message and
        // the notification code is AuthNotice::Done and
        // the notification args is an empty array and
        // a ProcessAuthMessage instance initialized with the fake KeyFileDB
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

        let args: Vec<Value> = Vec::new();
        let info = AuthInfo::new(AuthNotice::Done, args);
        let msg: Message = info.into();
        let process_msg = Box::new(ProcessAuthMessage::new(db));

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthMessage.change() with the notification
        // ----------------------------------------------------------
        let result = process_msg.change(msg);

        // ----------------------------------------------------------
        // THEN
        // A new AuthEnd state is returned
        // ----------------------------------------------------------
        let val = match result {
            Ok(State::AuthEnd) => true,
            _ => false,
        };
        assert!(val);
    }

    #[test]
    fn processauthmessage_notice_invalid()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Notification message and
        // the notification code is an unknown value and
        // the notification args is an empty array and
        // a ProcessAuthMessage instance initialized with the fake KeyFileDB
        // --------------------------------------------------------------------
        #[derive(Debug, PartialEq, Clone, CodeConvert)]
        enum FakeCode {
            Bad = 42,
        }
        type FakeInfo = NotificationMessage<FakeCode>;

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

        let args: Vec<Value> = Vec::new();
        let info = FakeInfo::new(FakeCode::Bad, args);
        let msg: Message = info.into();
        let process_msg = Box::new(ProcessAuthMessage::new(db));

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthMessage.change() with the notification
        // ----------------------------------------------------------
        let result = process_msg.change(msg);

        // ----------------------------------------------------------
        // THEN
        // A new AuthEnd state is returned
        // ----------------------------------------------------------
        let val = match result {
            Err(ProtocolError::InvalidNotification) => true,
            _ => false,
        };
        assert!(val);
    }

    // --------------------
    // ProcessAuthRequest
    // --------------------
    quickcheck! {
        fn processauthrequest_bad_numargs(args: Vec<u8>) -> TestResult {
            // Discard
            let numargs = args.len();
            if numargs == 1 {
                return TestResult::discard()
            }

            // -------------------------------------------
            // GIVEN
            // A fake KeyFileDB and
            // a Request message with number of args != 1
            // -------------------------------------------
            struct FakeDB;
            impl KeyFileStore for FakeDB {
                fn exists(&self, _k: &Vec<u8>) -> bool {
                    true
                }

                fn get(&self, _k: &Vec<u8>) -> KeyFileResult<Vec<u8>> {
                    unimplemented!()
                }
                fn set(&mut self, _k: &Vec<u8>, _file: &Vec<u8>) -> KeyFileResult<()> {
                    unimplemented!()
                }
                fn delete(&mut self, _k: &Vec<u8>) -> KeyFileResult<()>
                {
                    unimplemented!()
                }
            }
            let db = Rc::new(RwLock::new(FakeDB));

            let args: Vec<Value> =
                args.iter().map(|v| Value::from(v.clone())).collect();
            let req = AuthRequest::new(42, AuthMessage::KeyExists, args);
            let msg: Message = req.into();

            // -------------------------------------------------
            // WHEN
            // Calling ProcessAuthRequest.run() w/ any KeyfileDB
            // -------------------------------------------------
            let result = ProcessAuthRequest.run(db, msg);

            // -------------------------------------------------------
            // THEN
            // The ProtocolError::InvalidRequestArgs error is returned
            // -------------------------------------------------------
            let val = match result {
                Err(ProtocolError::InvalidRequestArgs) => true,
                _ => false
            };
            TestResult::from_bool(val)
        }
    }

    #[test]
    fn processauthrequest_bad_argtype()
    {
        // ---------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with a single argument and
        // the message argument is a non binary type
        // ---------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, _k: &Vec<u8>) -> bool
            {
                true
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
        let req = AuthRequest::new(42, AuthMessage::KeyExists, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let result = match ProcessAuthRequest.run(db, msg) {
            Err(ProtocolError::InvalidRequest) => true,
            _ => false,
        };

        // ---------------------------------------------------
        // THEN
        // The ProtocolError::InvalidRequest error is returned
        // ---------------------------------------------------
        assert!(result);
    }

    #[test]
    fn processauthrequest_run_key_exists()
    {
        // ---------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with a single argument and
        // the message argument is a binary type and
        // the request code is AuthMessage::KeyExists
        // ---------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let expected = "ANSWER".to_string().into_bytes();
                &expected == k
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

        let key = "ANSWER".to_string().into_bytes();
        let args = vec![Value::from(key)];
        let req = AuthRequest::new(42, AuthMessage::KeyExists, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::Nil and
        // the message's result is the value true
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::Nil);
        assert_eq!(response.result(), &Value::Boolean(true));
    }

    #[test]
    fn processauthrequest_run_key_notexists()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with a single argument and
        // the request code is AuthMessage::KeyExists and
        // the message argument is a key that doesn't exist in the keyfilestore
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let expected = "ANSWER".to_string().into_bytes();
                &expected == k
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

        let key = "42".to_string().into_bytes();
        let args = vec![Value::from(key)];
        let req = AuthRequest::new(42, AuthMessage::KeyExists, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // A AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::Nil and
        // the message's result is the value false
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::Nil);
        assert_eq!(response.result(), &Value::Boolean(false));
    }

    #[test]
    fn processauthrequest_run_getkey_notexists()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with a single argument and
        // the request code is AuthMessage::GetKeyFile and
        // the message argument is a key that doesn't exist in the keyfilestore
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let expected = "ANSWER".to_string().into_bytes();
                &expected == k
            }
            fn get(&self, k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                let expected = "ANSWER".to_string().into_bytes();
                if &expected != k {
                    Err(KeyFileError::Key(k.clone()))
                } else {
                    unreachable!()
                }
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

        let key = "42".to_string().into_bytes();
        let args = vec![Value::from(key)];
        let req = AuthRequest::new(42, AuthMessage::GetKeyFile, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // A AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::KeyFileNotFound and
        // the message's result is the key
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::KeyFileNotFound);

        let key = "42".to_string().into_bytes();
        assert_eq!(response.result(), &Value::from(key));
    }

    #[test]
    fn processauthrequest_run_getkey_exists()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with a single argument and
        // the request code is AuthMessage::GetKeyFile and
        // the message argument is a key that exists in the keyfilestore
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let expected = "ANSWER".to_string().into_bytes();
                &expected == k
            }
            fn get(&self, k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                let expected = "ANSWER".to_string().into_bytes();
                if &expected == k {
                    Ok("42".to_string().into_bytes())
                } else {
                    unreachable!()
                }
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

        let key = "ANSWER".to_string().into_bytes();
        let args = vec![Value::from(key)];
        let req = AuthRequest::new(42, AuthMessage::GetKeyFile, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::Nil and
        // the message's result is the expected file
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::Nil);

        let expected = Value::from("42".to_string().into_bytes());
        assert_eq!(response.result(), &expected);
    }

    #[test]
    fn processauthrequest_run_createkeyfile_keyexists()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with two arguments and
        // the request code is AuthMessage::CreateKeyFile and
        // the first message arg is a key that exists in the keyfilestore and
        // the second message arg is the keyfile key references
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let expected = "ANSWER".to_string().into_bytes();
                &expected == k
            }
            fn get(&self, _k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                let keyfile = "42".to_string().into_bytes();
                Ok(keyfile)
            }
            fn set(&mut self, _k: &Vec<u8>, _file: &Vec<u8>)
                -> KeyFileResult<()>
            {
                Ok(())
            }
            fn delete(&mut self, _k: &Vec<u8>) -> KeyFileResult<()>
            {
                unimplemented!()
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let key = "ANSWER".to_string().into_bytes();
        let keyfile = "42".to_string().into_bytes();
        let args = vec![Value::from(&key[..]), Value::from(&keyfile[..])];
        let req = AuthRequest::new(42, AuthMessage::CreateKeyFile, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::KeyFileExists and
        // the message's result is the false boolean value
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::KeyFileExists);

        assert_eq!(response.result(), &Value::from(key));
    }

    #[test]
    fn processauthrequest_run_createkeyfile_notexists()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with two arguments and
        // the request code is AuthMessage::CreateKeyFile and
        // the first message arg is a key that doesn't exist in the db and
        // the second message arg is the keyfile key references
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let expected = "ANSWER".to_string().into_bytes();
                &expected != k
            }
            fn get(&self, _k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                let keyfile = "42".to_string().into_bytes();
                Ok(keyfile)
            }
            fn set(&mut self, _k: &Vec<u8>, _file: &Vec<u8>)
                -> KeyFileResult<()>
            {
                Ok(())
            }
            fn delete(&mut self, _k: &Vec<u8>) -> KeyFileResult<()>
            {
                unimplemented!()
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let key = "ANSWER".to_string().into_bytes();
        let keyfile = "42".to_string().into_bytes();
        let args = vec![Value::from(&key[..]), Value::from(&keyfile[..])];
        let req = AuthRequest::new(42, AuthMessage::CreateKeyFile, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::Nil and
        // the message's result is the true boolean value
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::Nil);

        let expected = Value::Boolean(true);
        assert_eq!(response.result(), &expected);
    }

    #[test]
    fn processauthrequest_run_createkeyfile_dberror()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with two arguments and
        // the request code is AuthMessage::CreateKeyFile and
        // the first message arg is a key that doesn't exist in the db and
        // the second message arg is the keyfile key references
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let expected = "ANSWER".to_string().into_bytes();
                &expected != k
            }
            fn get(&self, _k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                let keyfile = "42".to_string().into_bytes();
                Ok(keyfile)
            }
            fn set(&mut self, _k: &Vec<u8>, _file: &Vec<u8>)
                -> KeyFileResult<()>
            {
                Err(KeyFileError::Other)
            }
            fn delete(&mut self, _k: &Vec<u8>) -> KeyFileResult<()>
            {
                unimplemented!()
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let key = "ANSWER".to_string().into_bytes();
        let keyfile = "42".to_string().into_bytes();
        let args = vec![Value::from(&key[..]), Value::from(&keyfile[..])];
        let req = AuthRequest::new(42, AuthMessage::CreateKeyFile, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message and
        // the KeyFileStore.set() method returns an error
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::DatabaseError and
        // the message's result is the false boolean value
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::DatabaseError);

        let expected = Value::Boolean(false);
        assert_eq!(response.result(), &expected);
    }

    #[test]
    fn processauthrequest_run_changekeyfile_keyexists()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with two arguments and
        // the request code is AuthMessage::ChangeKeyFile and
        // the first message arg is a key that exists in the keyfilestore and
        // the second message arg is the new replacement keyfile
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let expected = "ANSWER".to_string().into_bytes();
                &expected == k
            }
            fn get(&self, _k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                let keyfile = "LIFE".to_string().into_bytes();
                Ok(keyfile)
            }
            fn set(&mut self, _k: &Vec<u8>, _file: &Vec<u8>)
                -> KeyFileResult<()>
            {
                Ok(())
            }
            fn delete(&mut self, _k: &Vec<u8>) -> KeyFileResult<()>
            {
                unimplemented!()
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let key = "ANSWER".to_string().into_bytes();
        let keyfile = "42".to_string().into_bytes();
        let args = vec![Value::from(&key[..]), Value::from(&keyfile[..])];
        let req = AuthRequest::new(42, AuthMessage::ChangeKeyFile, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::Nil and
        // the message's result is the true boolean value
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::Nil);

        assert_eq!(response.result(), &Value::Boolean(true));
    }

    #[test]
    fn processauthrequest_run_changekeyfile_notexists()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with two arguments and
        // the request code is AuthMessage::ChangeKeyFile and
        // the first message arg is a key that doesn't exist in the db and
        // the second message arg is the new keyfile
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let expected = "ANSWER".to_string().into_bytes();
                &expected != k
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
                unimplemented!()
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let key = "ANSWER".to_string().into_bytes();
        let keyfile = "42".to_string().into_bytes();
        let args = vec![Value::from(&key[..]), Value::from(&keyfile[..])];
        let req = AuthRequest::new(42, AuthMessage::ChangeKeyFile, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::KeyFileNotFound and
        // the message's result is the key
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::KeyFileNotFound);

        let expected = Value::from(&key[..]);
        assert_eq!(response.result(), &expected);
    }

    #[test]
    fn processauthrequest_run_changekeyfile_dberror()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with two arguments and
        // the request code is AuthMessage::ChangeKeyFile and
        // the first message arg is a key that exists in the db and
        // the second message arg is the keyfile key references
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let expected = "ANSWER".to_string().into_bytes();
                &expected == k
            }
            fn get(&self, _k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                unreachable!()
            }
            fn set(&mut self, _k: &Vec<u8>, _file: &Vec<u8>)
                -> KeyFileResult<()>
            {
                Err(KeyFileError::Other)
            }
            fn delete(&mut self, _k: &Vec<u8>) -> KeyFileResult<()>
            {
                unimplemented!()
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let key = "ANSWER".to_string().into_bytes();
        let keyfile = "42".to_string().into_bytes();
        let args = vec![Value::from(&key[..]), Value::from(&keyfile[..])];
        let req = AuthRequest::new(42, AuthMessage::ChangeKeyFile, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message and
        // the KeyFileStore.set() method returns an error
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::DatabaseError and
        // the message's result is the false boolean value
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::DatabaseError);

        let expected = Value::Boolean(false);
        assert_eq!(response.result(), &expected);
    }

    #[test]
    fn processauthrequest_run_deletekeyfile_keyexists()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with a single argument and
        // the request code is AuthMessage::DeleteKeyFile and
        // the message arg is a key that exists in the keyfilestore
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let expected = "ANSWER".to_string().into_bytes();
                &expected == k
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
                Ok(())
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let key = "ANSWER".to_string().into_bytes();
        let args = vec![Value::from(&key[..])];
        let req = AuthRequest::new(42, AuthMessage::DeleteKeyFile, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::Nil and
        // the message's result is the true boolean value
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::Nil);

        assert_eq!(response.result(), &Value::Boolean(true));
    }

    #[test]
    fn processauthrequest_run_deletekeyfile_notexists()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with a single argument and
        // the request code is AuthMessage::DeleteKeyFile and
        // the message arg is a key that doesn't exist in the db
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let expected = "ANSWER".to_string().into_bytes();
                &expected != k
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
                unimplemented!()
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let key = "ANSWER".to_string().into_bytes();
        let args = vec![Value::from(&key[..])];
        let req = AuthRequest::new(42, AuthMessage::DeleteKeyFile, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::KeyFileNotFound and
        // the message's result is the key
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::KeyFileNotFound);

        let expected = Value::from(&key[..]);
        assert_eq!(response.result(), &expected);
    }

    #[test]
    fn processauthrequest_run_deletekeyfile_dberror()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with a single argument and
        // the request code is AuthMessage::DeleteKeyFile and
        // the message arg is a key that exists in the db
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let expected = "ANSWER".to_string().into_bytes();
                &expected == k
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
                Err(KeyFileError::Other)
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let key = "ANSWER".to_string().into_bytes();
        let args = vec![Value::from(&key[..])];
        let req = AuthRequest::new(42, AuthMessage::DeleteKeyFile, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message and
        // the KeyFileStore.set() method returns an error
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::DatabaseError and
        // the message's result is the false boolean value
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::DatabaseError);

        let expected = Value::Boolean(false);
        assert_eq!(response.result(), &expected);
    }

    #[test]
    fn processauthrequest_run_changekey_oldkeyexists()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with 2 arguments and
        // the request code is AuthMessage::ChangeKey and
        // the first message arg is a key that exists in the keyfilestore and
        // the second message arg is the new key that does not exist in the
        // keyfilestore
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let expected = "ANSWER".to_string().into_bytes();
                &expected == k
            }
            fn get(&self, _k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                let keyfile = "LIFE".to_string().into_bytes();
                Ok(keyfile)
            }
            fn set(&mut self, _k: &Vec<u8>, _file: &Vec<u8>)
                -> KeyFileResult<()>
            {
                Ok(())
            }
            fn delete(&mut self, _k: &Vec<u8>) -> KeyFileResult<()>
            {
                Ok(())
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let oldkey = "ANSWER".to_string().into_bytes();
        let newkey = "UNIVERSE".to_string().into_bytes();
        let args = vec![Value::from(&oldkey[..]), Value::from(&newkey[..])];
        let req = AuthRequest::new(42, AuthMessage::ChangeKey, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::Nil and
        // the message's result is the true boolean value
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::Nil);

        assert_eq!(response.result(), &Value::Boolean(true));
    }

    #[test]
    fn processauthrequest_run_changekey_newkeyexists()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with 2 arguments and
        // the request code is AuthMessage::ChangeKey and
        // the first message arg is a key that exists in the keyfilestore and
        // the second message arg is another key that exists in the
        // keyfilestore
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let old = "ANSWER".to_string().into_bytes();
                let new = "UNIVERSE".to_string().into_bytes();
                &old == k || &new == k
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

        let oldkey = "ANSWER".to_string().into_bytes();
        let newkey = "UNIVERSE".to_string().into_bytes();
        let args = vec![Value::from(&oldkey[..]), Value::from(&newkey[..])];
        let req = AuthRequest::new(42, AuthMessage::ChangeKey, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::Nil and
        // the message's result is the true boolean value
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::KeyFileExists);

        assert_eq!(response.result(), &Value::from(&newkey[..]));
    }

    #[test]
    fn processauthrequest_run_changekey_oldkey_notexists()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with two arguments and
        // the request code is AuthMessage::ChangeKey and
        // the first message arg is a key that doesn't exist in the db and
        // the second message arg is a key that doesn't exists in the db
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, _k: &Vec<u8>) -> bool
            {
                false
            }
            fn get(&self, k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                Err(KeyFileError::Key(Vec::from(&k[..])))
            }
            fn set(&mut self, _k: &Vec<u8>, _file: &Vec<u8>)
                -> KeyFileResult<()>
            {
                unreachable!()
            }
            fn delete(&mut self, _k: &Vec<u8>) -> KeyFileResult<()>
            {
                unimplemented!()
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let oldkey = "ANSWER".to_string().into_bytes();
        let newkey = "UNIVERSE".to_string().into_bytes();
        let args = vec![Value::from(&oldkey[..]), Value::from(&newkey[..])];
        let req = AuthRequest::new(42, AuthMessage::ChangeKey, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::KeyFileNotFound and
        // the message's result is the key
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::KeyFileNotFound);

        let expected = Value::from(&oldkey[..]);
        assert_eq!(response.result(), &expected);
    }

    #[test]
    fn processauthrequest_run_changekey_get_dberror()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with two arguments and
        // the request code is AuthMessage::ChangeKey and
        // the first message arg is a key that doesn't exist in the db and
        // the second message arg is a key that doesn't exists in the db and
        // any db get generates KeyFileError::Other error
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, _k: &Vec<u8>) -> bool
            {
                false
            }
            fn get(&self, _k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                Err(KeyFileError::Other)
            }
            fn set(&mut self, _k: &Vec<u8>, _file: &Vec<u8>)
                -> KeyFileResult<()>
            {
                unreachable!()
            }
            fn delete(&mut self, _k: &Vec<u8>) -> KeyFileResult<()>
            {
                unimplemented!()
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let oldkey = "ANSWER".to_string().into_bytes();
        let newkey = "UNIVERSE".to_string().into_bytes();
        let args = vec![Value::from(&oldkey[..]), Value::from(&newkey[..])];
        let req = AuthRequest::new(42, AuthMessage::ChangeKey, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::KeyFileNotFound and
        // the message's result is the key
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::DatabaseError);

        let expected = Value::Boolean(false);
        assert_eq!(response.result(), &expected);
    }

    #[test]
    fn processauthrequest_run_changekey_delete_dberror()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with 2 arguments and
        // the request code is AuthMessage::ChangeKey and
        // the first message arg is a key that exists in the db and
        // the second message arg is a key that doesn't exist in the db and
        // any db delete operation returns KeyFileError::Other error
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let oldkey = "ANSWER".to_string().into_bytes();
                &oldkey == k
            }
            fn get(&self, _k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                Ok("42".to_string().into_bytes())
            }
            fn set(&mut self, _k: &Vec<u8>, _file: &Vec<u8>)
                -> KeyFileResult<()>
            {
                unreachable!()
            }
            fn delete(&mut self, _k: &Vec<u8>) -> KeyFileResult<()>
            {
                Err(KeyFileError::Other)
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let oldkey = "ANSWER".to_string().into_bytes();
        let newkey = "UNIVERSE".to_string().into_bytes();
        let args = vec![Value::from(&oldkey[..]), Value::from(&newkey[..])];
        let req = AuthRequest::new(42, AuthMessage::ChangeKey, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::DatabaseError and
        // the message's result is the false boolean value
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::DatabaseError);

        let expected = Value::Boolean(false);
        assert_eq!(response.result(), &expected);
    }

    #[test]
    fn processauthrequest_run_changekey_set_dberror()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with 2 arguments and
        // the request code is AuthMessage::ChangeKey and
        // the first message arg is a key that exists in the db and
        // the second message arg is a key that doesn't exist in the db and
        // any db set operation returns KeyFileError::Other error
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let oldkey = "ANSWER".to_string().into_bytes();
                &oldkey == k
            }
            fn get(&self, _k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                Ok("42".to_string().into_bytes())
            }
            fn set(&mut self, _k: &Vec<u8>, _file: &Vec<u8>)
                -> KeyFileResult<()>
            {
                Err(KeyFileError::Other)
            }
            fn delete(&mut self, _k: &Vec<u8>) -> KeyFileResult<()>
            {
                Ok(())
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let oldkey = "ANSWER".to_string().into_bytes();
        let newkey = "UNIVERSE".to_string().into_bytes();
        let args = vec![Value::from(&oldkey[..]), Value::from(&newkey[..])];
        let req = AuthRequest::new(42, AuthMessage::ChangeKey, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::DatabaseError and
        // the message's result is the false boolean value
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::DatabaseError);

        let expected = Value::Boolean(false);
        assert_eq!(response.result(), &expected);
    }

    #[test]
    fn processauthrequest_run_changekey_success()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with 2 arguments and
        // the request code is AuthMessage::ChangeKey and
        // the first message arg is a key that exists in the db and
        // the second message arg is a key that doesn't exist in the db
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let oldkey = "ANSWER".to_string().into_bytes();
                &oldkey == k
            }
            fn get(&self, _k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                Ok("42".to_string().into_bytes())
            }
            fn set(&mut self, _k: &Vec<u8>, _file: &Vec<u8>)
                -> KeyFileResult<()>
            {
                Ok(())
            }
            fn delete(&mut self, _k: &Vec<u8>) -> KeyFileResult<()>
            {
                Ok(())
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let oldkey = "ANSWER".to_string().into_bytes();
        let newkey = "UNIVERSE".to_string().into_bytes();
        let args = vec![Value::from(&oldkey[..]), Value::from(&newkey[..])];
        let req = AuthRequest::new(42, AuthMessage::ChangeKey, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::DatabaseError and
        // the message's result is the false boolean value
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::Nil);

        let expected = Value::Boolean(true);
        assert_eq!(response.result(), &expected);
    }

    #[test]
    fn processauthrequest_run_replacekeyfile_newkey_exists()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with 2 arguments and
        // the request code is AuthMessage::ReplaceKeyFile and
        // the first message arg is a key that exists in the db and
        // the second message arg is a key that exists in the db
        // --------------------------------------------------------------------
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

        let oldkey = "ANSWER".to_string().into_bytes();
        let newkey = "UNIVERSE".to_string().into_bytes();
        let keyfile = "42".to_string().into_bytes();
        let args = vec![
            Value::from(&oldkey[..]),
            Value::from(&newkey[..]),
            Value::from(&keyfile[..]),
        ];
        let req = AuthRequest::new(42, AuthMessage::ReplaceKeyFile, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::KeyFileExists and
        // the message's result is the key that doesn't exist in the db
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::KeyFileExists);

        let expected = Value::from(&newkey[..]);
        assert_eq!(response.result(), &expected);
    }

    #[test]
    fn processauthrequest_run_replacekeyfile_deloldkey_dberror()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with 2 arguments and
        // the request code is AuthMessage::ReplaceKeyFile and
        // the first message arg is a key that exists in the db and
        // the second message arg is a key that doesn't exist in the db and
        // the db delete operation returns KeyFileError::Other error
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let indb = "UNIVERSE".to_string().into_bytes();
                &indb != k
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
                Err(KeyFileError::Other)
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let oldkey = "ANSWER".to_string().into_bytes();
        let newkey = "UNIVERSE".to_string().into_bytes();
        let keyfile = "42".to_string().into_bytes();
        let args = vec![
            Value::from(&oldkey[..]),
            Value::from(&newkey[..]),
            Value::from(&keyfile[..]),
        ];
        let req = AuthRequest::new(42, AuthMessage::ReplaceKeyFile, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::DatabaseError and
        // the message's result is the false boolean value
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::DatabaseError);

        let expected = Value::Boolean(false);
        assert_eq!(response.result(), &expected);
    }

    #[test]
    fn processauthrequest_run_replacekeyfile_oldkey_notexists()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with 2 arguments and
        // the request code is AuthMessage::ReplaceKeyFile and
        // the first message arg is a key that exists in the db and
        // the second message arg is a key that doesn't exist in the db and
        // the db delete operation returns KeyFileError::Key error
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let indb = "UNIVERSE".to_string().into_bytes();
                &indb != k
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
            fn delete(&mut self, k: &Vec<u8>) -> KeyFileResult<()>
            {
                Err(KeyFileError::Key(k.clone()))
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let oldkey = "ANSWER".to_string().into_bytes();
        let newkey = "UNIVERSE".to_string().into_bytes();
        let keyfile = "42".to_string().into_bytes();
        let args = vec![
            Value::from(&oldkey[..]),
            Value::from(&newkey[..]),
            Value::from(&keyfile[..]),
        ];
        let req = AuthRequest::new(42, AuthMessage::ReplaceKeyFile, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::KeyFileNotFound and
        // the message's result is the false boolean value
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::KeyFileNotFound);

        let expected = Value::from(&oldkey[..]);
        assert_eq!(response.result(), &expected);
    }

    #[test]
    fn processauthrequest_run_replacekeyfile_setnewkey_dberror()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with 2 arguments and
        // the request code is AuthMessage::ReplaceKeyFile and
        // the first message arg is a key that exists in the db and
        // the second message arg is a key that doesn't exist in the db and
        // the db set operation returns KeyFileError::Other error
        // --------------------------------------------------------------------
        struct FakeDB;
        impl KeyFileStore for FakeDB {
            fn exists(&self, k: &Vec<u8>) -> bool
            {
                let indb = "UNIVERSE".to_string().into_bytes();
                &indb != k
            }
            fn get(&self, _k: &Vec<u8>) -> KeyFileResult<Vec<u8>>
            {
                unreachable!()
            }
            fn set(&mut self, _k: &Vec<u8>, _file: &Vec<u8>)
                -> KeyFileResult<()>
            {
                Err(KeyFileError::Other)
            }
            fn delete(&mut self, _k: &Vec<u8>) -> KeyFileResult<()>
            {
                Ok(())
            }
        }
        let db = Rc::new(RwLock::new(FakeDB));

        let oldkey = "ANSWER".to_string().into_bytes();
        let newkey = "UNIVERSE".to_string().into_bytes();
        let keyfile = "42".to_string().into_bytes();
        let args = vec![
            Value::from(&oldkey[..]),
            Value::from(&newkey[..]),
            Value::from(&keyfile[..]),
        ];
        let req = AuthRequest::new(42, AuthMessage::ReplaceKeyFile, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessAuthRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessAuthRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // An AuthResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is AuthError::DatabaseError and
        // the message's result is the false boolean value
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), AuthError::DatabaseError);

        let expected = Value::Boolean(false);
        assert_eq!(response.result(), &expected);
    }

}


// ===========================================================================
//
// ===========================================================================
