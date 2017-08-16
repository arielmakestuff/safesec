// test_rpcserver.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

// ===========================================================================
// Externs
// ===========================================================================
#![recursion_limit = "1024"]

// Stdlib externs

// Third-party externs
extern crate bytes;
extern crate futures;

extern crate rmp;
extern crate rmp_serde as rmps;
extern crate rmpv;
extern crate serde;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_service;

// Local externs
extern crate safesec;


// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports

use std::collections::HashMap;
use std::io;
use std::mem;
use std::thread;
use std::time::Duration;

// Third-party imports

use bytes::BytesMut;
use futures::{Async, Future, Poll, Sink, Stream, future, stream, task};
use futures::sync::mpsc;
use rmpv::Value;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Core;
use tokio_io::AsyncRead;
use tokio_io::codec::{Decoder, Encoder};
use tokio_io::io::{WriteAll, write_all};

// Local imports

use safesec::network::codec::MsgPackCodec;
use safesec::network::rpc::{Message, MessageType, RpcMessage, RpcResponse};
use safesec::network::server::ServerMessage;
use safesec::protocol::message::{AuthError, AuthMessage, AuthNotice,
                                 BootNotice, SessionType};
use safesec::serve;
use safesec::service::state::SessionInfo;
use safesec::service::state::auth::{AuthInfo, AuthRequest, AuthResponse};
use safesec::service::state::boot::{BootInfo, BootResponse};


// ===========================================================================
// ClientSession
// ===========================================================================


type FutureSession = Box<Future<Item = ClientSession, Error = io::Error>>;


struct ClientSession {
    session_type: SessionType,
    in_messages: HashMap<u32, Message>,
    out_messages: Vec<Message>,
    socket: Option<TcpStream>,
    msgid: Option<u32>,
}


impl ClientSession {
    pub fn new(socket: TcpStream, session_type: SessionType) -> Self
    {
        Self {
            session_type: session_type,
            in_messages: HashMap::new(),
            out_messages: Vec::new(),
            socket: Some(socket),
            msgid: None,
        }
    }

    pub fn start(self) -> FutureSession
    {
        let msg = SessionInfo::new(self.session_type(), vec![]).into();
        self.send_msg(msg)
    }

    pub fn done(self) -> FutureSession
    {
        let session_type = self.session_type();
        let msg: Message = match session_type {
            SessionType::Boot => {
                BootInfo::new(BootNotice::Done, vec![]).into()
            }
            SessionType::Auth => {
                AuthInfo::new(AuthNotice::Done, vec![]).into()
            }
        };
        self.send_msg(msg)
    }

    pub fn send_msg(mut self, msg: Message) -> FutureSession
    {
        let msgtype = msg.message_type().unwrap();
        self.out_messages.push(msg);
        let mut future: FutureSession = Box::new(SessionWrite::new(self));
        if let MessageType::Request = msgtype {
            future = Box::new(
                future.and_then(|session| SessionRead::new(session)),
            );
        }
        future
    }

    pub fn session_type(&self) -> SessionType
    {
        self.session_type.clone()
    }

    // pub fn num_outmsg(&self) -> usize
    // {
    //     self.out_messages.len()
    // }

    pub fn has_outmsg(&self) -> bool
    {
        !self.out_messages.is_empty()
    }

    pub fn outmsg_stream(&mut self)
        -> Box<Stream<Item = Message, Error = ()>>
    {
        let messages = mem::replace(&mut self.out_messages, Vec::new());
        Box::new(stream::iter(messages.into_iter().map(Ok)).boxed())
    }

    pub fn socket(&mut self, newval: Option<TcpStream>) -> Option<TcpStream>
    {
        mem::replace(&mut self.socket, newval)
    }

    pub fn store_server_message(&mut self, id: u32, msg: Message)
    {
        self.in_messages.insert(id, msg);
    }

    pub fn pop_server_message(&mut self, id: u32) -> Option<Message>
    {
        self.in_messages.remove(&id)
    }

    // pub fn num_server_messages(&self) -> usize
    // {
    //     self.in_messages.len()
    // }

    pub fn new_msgid(&mut self) -> u32
    {
        let id = self.msgid;
        match id {
            None => {
                self.msgid = Some(0);
                0
            }
            Some(id) => {
                let newid = id + 1;
                self.msgid = Some(newid);
                newid
            }
        }
    }

    pub fn cur_msgid(&self) -> u32
    {
        self.msgid.expect(
            "ClientSession has not generated a msgid yet, please call \
             new_msgid() first",
        )
    }
}


// ===========================================================================
// SessionWrite
// ===========================================================================


struct SessionWrite {
    session: Option<ClientSession>,
    messages: Option<Box<Stream<Item = Message, Error = ()>>>,
    write_future: Option<WriteAll<TcpStream, BytesMut>>,
}


impl SessionWrite {
    pub fn new(session: ClientSession) -> Self
    {
        Self {
            session: Some(session),
            messages: None,
            write_future: None,
        }
    }

    // fn session_ref(&mut self) -> &mut ClientSession
    // {
    //     match self.session {
    //         Some(ref mut s) => s,
    //         None => unreachable!(),
    //     }
    // }

    fn session(&mut self) -> ClientSession
    {
        mem::replace(&mut self.session, None).unwrap()
    }

    fn encode_msg(&self, msg: Message) -> BytesMut
    {
        let val: Value = msg.into();
        let mut buf = BytesMut::new();
        MsgPackCodec.encode(val, &mut buf).unwrap();
        buf
    }

    fn waitfor_socket_write(&mut self, msg: Message)
        -> WriteAll<TcpStream, BytesMut>
    {
        let buf = self.encode_msg(msg);
        let session = match self.session {
            Some(ref mut s) => s,
            None => unreachable!(),
        };
        match session.socket(None) {
            Some(s) => write_all(s, buf),
            None => unreachable!(),
        }
    }
}


impl Future for SessionWrite {
    type Item = ClientSession;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error>
    {
        // Poll message write
        match self.write_future {
            None => {}
            Some(ref mut f) => {
                match f.poll() {
                    Ok(Async::Ready((s, _))) => {
                        let session = match self.session {
                            Some(ref mut s) => s,
                            None => unreachable!(),
                        };
                        session.socket(Some(s));
                    }
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Err(e) => return Err(e),
                }
            }
        }

        // Create message stream if there are any messages
        {
            let session = match self.session {
                Some(ref mut s) => s,
                None => unreachable!(),
            };
            if self.messages.is_none() && session.has_outmsg() {
                self.messages = Some(session.outmsg_stream());
            }
        }

        // Poll message stream
        let result = match self.messages {
            None => None,
            Some(ref mut f) => Some(f.poll()),
        };
        if let Some(poll_result) = result {
            match poll_result {
                Err(()) => unreachable!(),
                Ok(Async::NotReady) => unreachable!(),
                Ok(Async::Ready(None)) => {
                    self.write_future = None;
                    self.messages = None;
                    let session = self.session();
                    Ok(Async::Ready(session))
                }
                Ok(Async::Ready(Some(msg))) => {
                    self.write_future = Some(self.waitfor_socket_write(msg));
                    task::current().notify();
                    Ok(Async::NotReady)
                }
            }
        } else {
            let session = self.session();
            Ok(Async::Ready(session))
        }
    }
}


// ===========================================================================
// SessionRead
// ===========================================================================


#[derive(Debug)]
pub struct ReadToBlock<A> {
    state: ReadToBlockState<A>,
}


#[derive(Debug)]
enum ReadToBlockState<A> {
    Reading { a: A, buf: Vec<u8> },
    Empty,
}


/// Creates a future which will read all the bytes associated with the I/O
/// object `A` into the buffer provided until either the read operation will
/// block or EOF is reached.
///
/// In the case of an error the buffer and the object will be discarded, with
/// the error yielded. In the case of success the object and the buffer will
/// be returned, with all data read from the stream appended to the buffer.
pub fn read_to_block<A>(a: A, buf: Vec<u8>) -> ReadToBlock<A>
where
    A: AsyncRead,
{
    ReadToBlock { state: ReadToBlockState::Reading { a: a, buf: buf } }
}


impl<A> Future for ReadToBlock<A>
where
    A: AsyncRead,
{
    type Item = (A, Vec<u8>);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(A, Vec<u8>), io::Error>
    {
        match self.state {
            ReadToBlockState::Reading {
                ref mut a,
                ref mut buf,
            } => {
                // If we get `Ok`, then we know the stream hit EOF and we're
                // done. If we hit "would block" then all the read data so far
                // is in our buffer and the future completes. Otherwise we
                // propagate errors
                match a.read_to_end(buf) {
                    // match a.read(&mut newbuf) {
                    Ok(_t) => {
                        // println!("READ {} BYTES", _t);
                        // return Ok(Async::NotReady); // <=== FIX THIS!
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        if buf.len() == 0 {
                            return Ok(Async::NotReady);
                        }
                    }
                    Err(e) => return Err(e.into()),
                }
            }
            ReadToBlockState::Empty => {
                panic!("poll ReadToBlock after it's done")
            }
        }

        match mem::replace(&mut self.state, ReadToBlockState::Empty) {
            ReadToBlockState::Reading { a, buf } => Ok((a, buf).into()),
            ReadToBlockState::Empty => unreachable!(),
        }
    }
}


struct SessionRead {
    session: Option<ClientSession>,
    read_future: Option<ReadToBlock<TcpStream>>,
}


impl SessionRead {
    pub fn new(session: ClientSession) -> Self
    {
        Self {
            session: Some(session),
            read_future: None,
        }
    }

    fn session_ref(&mut self) -> &mut ClientSession
    {
        match self.session {
            Some(ref mut s) => s,
            None => unreachable!(),
        }
    }

    fn session(&mut self) -> ClientSession
    {
        mem::replace(&mut self.session, None).unwrap()
    }

    fn waitfor_socket_read(&mut self) -> ReadToBlock<TcpStream>
    {
        let buf = Vec::new();
        let session = self.session_ref();
        match session.socket(None) {
            Some(s) => read_to_block(s, buf),
            None => unreachable!(),
        }
    }

    fn message_id(&self, msgtype: SessionType, msg: Message)
        -> Option<(u32, Message)>
    {
        match msgtype {
            SessionType::Auth => {
                match AuthResponse::from(msg) {
                    Ok(response) => {
                        let id = response.message_id();
                        let msg: Message = response.into();
                        Some((id, msg))
                    }
                    Err(_) => None,
                }
            }
            SessionType::Boot => {
                match BootResponse::from(msg) {
                    Ok(response) => {
                        let id = response.message_id();
                        let msg: Message = response.into();
                        Some((id, msg))
                    }
                    Err(_) => None,
                }
            }
        }
    }

    // self.session is None by the time decode_responses is called
    fn decode_responses(&self, session: &mut ClientSession, data: Vec<u8>)
        -> io::Result<()>
    {
        // Decode data
        let mut buf = BytesMut::from(data);
        while !buf.is_empty() {
            // Decode bytes
            let res = MsgPackCodec.decode(&mut buf)?.unwrap();

            // Process the server messag
            let msg = Message::from(res).unwrap();
            self.process_server_message(session, msg);
        }
        Ok(())
    }

    fn process_server_message(&self, session: &mut ClientSession, msg: Message)
    {
        // Grab mutable reference to session
        let msgtype = msg.message_type().unwrap();
        match msgtype {
            MessageType::Request => unreachable!(),
            MessageType::Notification => {}
            MessageType::Response => {
                let session_type = session.session_type();

                // Only store messages that have a msg id
                match self.message_id(session_type, msg) {
                    // Do nothing if message has no msg id
                    None => {}

                    // Store server message in the session
                    Some((id, msg)) => {
                        session.store_server_message(id, msg);
                    }
                }
            }
        }
    }
}


impl Future for SessionRead {
    type Item = ClientSession;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error>
    {
        // Poll read future
        let poll_future = match self.read_future {
            None => {
                self.read_future = Some(self.waitfor_socket_read());
                task::current().notify();
                return Ok(Async::NotReady);
            }
            Some(ref mut f) => f.poll(),
        };
        match poll_future {
            Ok(Async::Ready((socket, buf))) => {
                let mut session = self.session();
                // let session = match self.session {
                //     Some(ref mut s) => s,
                //     None => unreachable!(),
                // };
                session.socket(Some(socket));
                self.decode_responses(&mut session, buf)?;
                Ok(Async::Ready(session))
            }
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}


// ===========================================================================
// HelloWorld
// ===========================================================================


struct HelloWorld {
    result: Option<String>,
}


type HWFuture = Box<
    Future<
        Item = (HelloWorld, ClientSession),
        Error = io::Error,
    >,
>;


impl HelloWorld {
    fn send_get_keyfile(self, mut session: ClientSession) -> HWFuture
    {
        let value = Value::from("42".to_string().into_bytes());
        let args = vec![value];
        let msgid = session.new_msgid();
        let msg = AuthRequest::new(msgid, AuthMessage::GetKeyFile, args);
        Box::new(session.send_msg(msg.into()).map(move |s| (self, s)))
    }

    fn response_get_keyfile(mut self, mut session: ClientSession)
        -> HWFuture
    {
        let msgid = session.cur_msgid();
        let errmsg = format!("Error: msg with id {} does not exist", msgid);
        let msg = session.pop_server_message(msgid).expect(&errmsg);
        let response = AuthResponse::from(msg).unwrap();

        match response.error_code() {
            AuthError::KeyFileNotFound => {
                let f = self.send_add_keyfile(session).and_then(
                    |(hw, session)| {
                        hw.response_add_keyfile(session)
                    },
                );
                Box::new(f)
            }
            AuthError::Nil => {
                let result = response.result();
                let val = String::from_utf8(
                    result.as_slice().unwrap().to_vec(),
                ).unwrap();
                self.result = Some(val);
                Box::new(future::ok::<(Self, ClientSession), io::Error>(
                    (self, session),
                ))
            }
            _ => Box::new(future::ok::<(Self, ClientSession), io::Error>(
                (self, session),
            )),
        }
    }

    fn send_add_keyfile(self, mut session: ClientSession) -> HWFuture
    {
        let key = Value::from("42".to_string().into_bytes());
        let value = Value::from(
            "The Answer to Life, the Universe, and \
             Everything"
                .to_string()
                .into_bytes(),
        );
        let args = vec![key, value];
        let msgid = session.new_msgid();
        let msg = AuthRequest::new(msgid, AuthMessage::CreateKeyFile, args);
        let f = session.send_msg(msg.into()).map(move |s| (self, s));
        Box::new(f)
    }

    fn response_add_keyfile(self, mut session: ClientSession) -> HWFuture
    {
        let msgid = session.cur_msgid();
        let errmsg = format!("msgid {} doesn't exist", msgid);
        let msg = session.pop_server_message(msgid).expect(errmsg.as_str());
        let response = AuthResponse::from(msg).unwrap();

        match response.error_code() {
            AuthError::Nil => {
                let f = self.send_get_keyfile(session).and_then(
                    |(hw, session)| {
                        hw.response_get_keyfile(session)
                    },
                );
                Box::new(f)
            }
            AuthError::DatabaseError => {
                let err = io::Error::new(
                    io::ErrorKind::Other,
                    "Database error creating keyfile 42",
                );
                let f = future::err::<(Self, ClientSession), io::Error>(err);
                Box::new(f)
            }
            AuthError::KeyFileExists => unreachable!(),
            AuthError::KeyFileNotFound => unreachable!(),
            // _ => unreachable!(),
        }
    }
}


fn hello_world(socket: TcpStream) -> FutureSession
{
    let session = ClientSession::new(socket, SessionType::Auth);
    let client = HelloWorld { result: None };
    let future = session

        // Start session (sends notice that will use Auth session)
        .start()

        // Get keyfile
        .and_then(|session| client.send_get_keyfile(session))
        .and_then(|(client, session)| client.response_get_keyfile(session))

        // Finish session
        .and_then(|(client, session)| session.done().map(|s| (client, s)))

        // Print result
        .and_then(|(mut client, session)| {
            let result = mem::replace(&mut client.result, None);
            match result {
                Some(val) => println!("{}", val),
                None => println!("Did not get an answer!"),
            }
            Ok(session)
        });
    Box::new(future)
}


// ===========================================================================
// Client
// ===========================================================================


fn client_adddata(control: mpsc::Sender<ServerMessage>)
    -> io::Result<String>
{
    // Create event loop
    let mut core = Core::new()?;
    let handle = core.handle();
    // let mut codec = MsgPackCodec;

    // Connect to remote server
    let address = "127.0.0.1:12345".parse().unwrap();
    let socket = TcpStream::connect(&address, &handle);

    // Set up request and response
    let request = socket.and_then(|s| hello_world(s));

    // Shutdown server once everything done
    let request = request.and_then(move |_s| {
        control.send(ServerMessage::Shutdown).map(|_| ()).map_err(
            |_| {
                io::Error::new(
                    io::ErrorKind::Other,
                    "error sending shutdown command",
                )
            },
        )
    });

    // Send request and get response
    // let (_socket, data) = core.run(response).unwrap();
    core.run(request).unwrap();
    Ok("good".to_string())
}

// ===========================================================================
// Tests
// ===========================================================================


extern crate chrono;
extern crate tempdir;

use std::fs;

use chrono::prelude::*;
use tempdir::TempDir;

use safesec::Config;
use safesec::storage::KeyFileBuilder;
use safesec::storage::lmdb::KeyFile;

fn _mktempdir() -> TempDir
{
    // Generate unique temp name
    let dt = UTC::now();
    let suffix = dt.format("%Y%m%d%H%M%S%.9f");
    let name = format!("safesec_test_{}", suffix.to_string());
    let tmpdir = TempDir::new(&name).unwrap();
    let dbpath = tmpdir.path().join("sec.db");
    fs::create_dir(&dbpath).unwrap();
    tmpdir

}


fn _create_db(tmpdir: &TempDir) -> KeyFile
{
    // Create temp directory
    // let tmpdir = _mktempdir();
    let dbpath = tmpdir.path().join("sec.db");

    // Create keyfile store
    KeyFile::new("temp", Some(dbpath.as_path()))
}


#[test]
fn test_rpcserver()
{
    // Create database and bind address
    let tmpdir = _mktempdir();
    let dbdir = tmpdir.path().to_owned();
    // let db = Rc::new(RwLock::new(_create_db(&tmpdir)));
    let address = "127.0.0.1:12345".parse().unwrap();

    // Create a config
    let config = Config {
        name: "safesec".to_string(),
        dbdir: dbdir,
        bindaddr: address,
    };

    // Create command channel
    let (tx, rx) = mpsc::channel::<ServerMessage>(1);

    // Start server
    let child = thread::spawn(move || if let Err(e) = serve(&config, rx) {
        // println!("Server failed with {}", e);
        panic!("Server failed with {}", e);
    });

    thread::sleep(Duration::from_millis(500));

    // Start client
    let res = client_adddata(tx);

    let val = match res {
        Ok(t) => {
            println!("{}", t);
            true
        }
        Err(e) => {
            println!("Client error: {}", e);
            false
        }
    };

    child.join().unwrap();
    assert!(val);
}


// ===========================================================================
//
// ===========================================================================
