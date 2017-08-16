// src/service/state/boot.rs
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

// Local imports

use super::{KeyFileDB, SessionState, State, StateResult};
use network::rpc::{Message, MessageType, NotificationMessage,
                   RequestMessage, ResponseMessage, RpcMessage, RpcNotice,
                   RpcRequest};
use protocol::message::{BootError, BootMessage, BootNotice, ProtocolError};
use rmpv::Value;
use storage::KeyFileError;


// ===========================================================================
// Boot states
// ===========================================================================


pub type BootRequest = RequestMessage<BootMessage>;


pub type BootResponse = ResponseMessage<BootError>;


pub type BootInfo = NotificationMessage<BootNotice>;


// ===========================================================================
// Receive boot message state
// ===========================================================================


pub struct ProcessBootMessage {
    db: KeyFileDB,
}


impl ProcessBootMessage {
    pub fn new(db: KeyFileDB) -> Self
    {
        Self { db: db }
    }
}


impl SessionState for ProcessBootMessage {
    fn change(self: Box<Self>, m: Message) -> StateResult<State>
    {
        match m.message_type().unwrap() {

            // If the message is a request, process as a BootMethod and change
            // state back to ProcessBootMessage
            MessageType::Request => {
                let response = ProcessBootRequest.run(self.db.clone(), m)?;
                Ok(State::ProcessBootMessage(
                    Box::new(Self { db: self.db }),
                    Some(response),
                ))
            }

            // If the message is a done notification, change state to BootEnd
            MessageType::Notification => {
                let notice = BootInfo::from(m).map_err(|_| {
                    ProtocolError::InvalidNotification
                })?;
                match notice.message_code() {
                    BootNotice::Done => Ok(State::BootEnd),
                }
            }

            // If the message is a response, return an error
            MessageType::Response => Err(ProtocolError::UnexpectedMessage),
        }
    }
}


struct ProcessBootRequest;


impl ProcessBootRequest {
    fn run(&self, db: KeyFileDB, m: Message) -> StateResult<BootResponse>
    {
        let req = BootRequest::from(m).unwrap();
        match req.message_code() {
            BootMessage::KeyExists => return self.req_key_exists(req, db),
            BootMessage::GetKeyFile => return self.req_get_keyfile(req, db),
        }
    }

    fn _check_message(&self, req: &BootRequest) -> StateResult<Vec<u8>>
    {
        // Get message arguments
        let args = req.message_args();

        // Must only have a single argument
        if args.len() != 1 {
            return Err(ProtocolError::InvalidRequestArgs);
        }

        // The argument must be binary data
        if !args[0].is_bin() {
            return Err(ProtocolError::InvalidRequest);
        }
        let key: Vec<u8> = Vec::from(args[0].as_slice().unwrap());
        Ok(key)
    }

    fn req_key_exists(&self, req: BootRequest, db: KeyFileDB)
        -> StateResult<BootResponse>
    {
        // Get key
        let key = self._check_message(&req)?;

        // Get result, dropping the db lock as soon as possible
        let result = {
            let db = db.read().unwrap();
            Value::Boolean(db.exists(&key))
        };

        // Create response
        let response =
            BootResponse::new(req.message_id(), BootError::Nil, result);
        Ok(response)
    }

    fn req_get_keyfile(&self, req: BootRequest, db: KeyFileDB)
        -> StateResult<BootResponse>
    {
        // Get key
        let key = self._check_message(&req)?;

        // Get keyfile, dropping the db lock as soon as possible
        let keyfile = {
            let db = db.read().unwrap();
            db.get(&key)
        };

        match keyfile {
            // Create response
            Ok(f) => {
                let response = BootResponse::new(
                    req.message_id(),
                    BootError::Nil,
                    Value::from(f),
                );
                Ok(response)
            }

            // Create error response
            Err(KeyFileError::Key(k)) => {
                let response = BootResponse::new(
                    req.message_id(),
                    BootError::KeyFileNotFound,
                    Value::from(k),
                );
                Ok(response)
            }

            // TODO: handle other errors that may be raised (eg from lmdb backend)
            Err(KeyFileError::Other) => unimplemented!(),
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

    use super::{BootInfo, BootRequest, BootResponse, ProcessBootMessage,
                ProcessBootRequest};
    use error::{Error, GeneralError, Result};
    use network::rpc::{CodeConvert, Message, NotificationMessage,
                       RpcResponse};
    use protocol::message::{BootError, BootMessage, BootNotice,
                            ProtocolError};
    use service::state::{SessionState, State};
    use storage::{KeyFileError, KeyFileResult, KeyFileStore};

    // --------------------
    // ProcessBootRequest
    // --------------------
    quickcheck! {
        fn processbootrequest_bad_numargs(args: Vec<u8>) -> TestResult {
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
            let req = BootRequest::new(42, BootMessage::KeyExists, args);
            let msg: Message = req.into();

            // -------------------------------------------------
            // WHEN
            // Calling ProcessBootRequest.run() w/ any KeyfileDB
            // -------------------------------------------------
            let result = ProcessBootRequest.run(db, msg);

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
    fn processbootrequest_bad_argtype()
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
        let req = BootRequest::new(42, BootMessage::KeyExists, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessBootRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let result = match ProcessBootRequest.run(db, msg) {
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
    fn processbootrequest_run_key_exists()
    {
        // ---------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with a single argument and
        // the message argument is a binary type and
        // the request code is BootMessage::KeyExists
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
                // let expected = "ANSWER".to_string().into_bytes();
                // if &expected == k {
                //     Ok("42".to_string().into_bytes())
                // } else {
                //     unreachable!()
                // }
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
        let req = BootRequest::new(42, BootMessage::KeyExists, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessBootRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessBootRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // A BootResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is BootError::Nil and
        // the message's result is the value true
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), BootError::Nil);
        assert_eq!(response.result(), &Value::Boolean(true));
    }

    #[test]
    fn processbootrequest_run_key_notexists()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with a single argument and
        // the request code is BootMessage::KeyExists and
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
        let req = BootRequest::new(42, BootMessage::KeyExists, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessBootRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessBootRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // A BootResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is BootError::Nil and
        // the message's result is the value false
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), BootError::Nil);
        assert_eq!(response.result(), &Value::Boolean(false));
    }

    #[test]
    fn processbootrequest_run_getkey_notexists()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with a single argument and
        // the request code is BootMessage::GetKeyFile and
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
        let req = BootRequest::new(42, BootMessage::GetKeyFile, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessBootRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessBootRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // A BootResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is BootError::KeyFileNotFound and
        // the message's result is the key
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), BootError::KeyFileNotFound);

        let key = "42".to_string().into_bytes();
        assert_eq!(response.result(), &Value::from(key));
    }

    #[test]
    fn processbootrequest_run_getkey_exists()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with a single argument and
        // the request code is BootMessage::GetKeyFile and
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
        let req = BootRequest::new(42, BootMessage::GetKeyFile, args);
        let msg: Message = req.into();

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessBootRequest.run() with a FakeDB object and
        // the request message
        // ----------------------------------------------------------
        let response = ProcessBootRequest.run(db, msg).unwrap();

        // ------------------------------------------------------------------
        // THEN
        // A BootResponse message is returned and
        // the message's message_id is the same as the request message_id and
        // the message's error code is BootError::Nil and
        // the message's result is the expected file
        // ------------------------------------------------------------------
        assert_eq!(response.message_id(), 42);
        assert_eq!(response.error_code(), BootError::Nil);

        let expected = Value::from("42".to_string().into_bytes());
        assert_eq!(response.result(), &expected);
    }

    // --------------------
    // ProcessBootMessage
    // --------------------
    #[test]
    fn processbootmessage_request_error()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with 2 arguments and
        // the request code is BootMessage::KeyExists and
        // the first message argument is a key that does not exist and
        // the second message argument is Nil and
        // a ProcessBootMessage instance initialized with the fake KeyFileDB
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
        let req = BootRequest::new(42, BootMessage::GetKeyFile, args);
        let msg: Message = req.into();
        let process_msg = Box::new(ProcessBootMessage::new(db));

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessBootMessage.change() with the request
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
    fn processbootmessage_request_response()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Request message with a single argument and
        // the request code is BootMessage::KeyExists and
        // the message argument is a key that does not exist and
        // a ProcessBootMessage instance initialized with the fake KeyFileDB
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
        let req = BootRequest::new(42, BootMessage::KeyExists, args);
        let msg: Message = req.into();
        let process_msg = Box::new(ProcessBootMessage::new(db));

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessBootMessage.change() with the request
        // ----------------------------------------------------------
        let result = process_msg.change(msg);

        // ----------------------------------------------------------
        // THEN
        // A new ProcessBootMessage state is returned with a response
        // ----------------------------------------------------------
        let val = match result {
            Ok(State::ProcessBootMessage(_state, Some(response))) => {
                assert_eq!(response.message_id(), 42);
                assert_eq!(response.error_code(), BootError::Nil);
                let expected = Value::Boolean(true);
                assert_eq!(response.result(), &expected);
                true
            }
            _ => false,
        };
        assert!(val);
    }

    #[test]
    fn processbootmessage_notice_valid()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Notification message and
        // the notification code is BootNotice::Done and
        // the notification args is an empty array and
        // a ProcessBootMessage instance initialized with the fake KeyFileDB
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
        let info = BootInfo::new(BootNotice::Done, args);
        let msg: Message = info.into();
        let process_msg = Box::new(ProcessBootMessage::new(db));

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessBootMessage.change() with the notification
        // ----------------------------------------------------------
        let result = process_msg.change(msg);

        // ----------------------------------------------------------
        // THEN
        // A new BootEnd state is returned
        // ----------------------------------------------------------
        let val = match result {
            Ok(State::BootEnd) => true,
            _ => false,
        };
        assert!(val);
    }

    #[test]
    fn processbootmessage_notice_invalid()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Notification message and
        // the notification code is an unknown value and
        // the notification args is an empty array and
        // a ProcessBootMessage instance initialized with the fake KeyFileDB
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
        let process_msg = Box::new(ProcessBootMessage::new(db));

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessBootMessage.change() with the notification
        // ----------------------------------------------------------
        let result = process_msg.change(msg);

        // ----------------------------------------------------------
        // THEN
        // A new BootEnd state is returned
        // ----------------------------------------------------------
        let val = match result {
            Err(ProtocolError::InvalidNotification) => true,
            _ => false,
        };
        assert!(val);
    }

    #[test]
    fn processbootmessage_response_any()
    {
        // --------------------------------------------------------------------
        // GIVEN
        // A fake KeyFileDB and
        // a Response message and
        // a ProcessBootMessage instance initialized with the fake KeyFileDB
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
        let process_msg = Box::new(ProcessBootMessage::new(db));

        // ----------------------------------------------------------
        // WHEN
        // Calling ProcessBootMessage.change() with the notification
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
}


// ===========================================================================
//
// ===========================================================================
