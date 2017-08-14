// src/lib.rs
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
extern crate lmdb;
extern crate lmdb_sys;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

extern crate rmp;
extern crate rmp_serde as rmps;
extern crate rmpv;
extern crate serde;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_service;

// Local externs

#[macro_use]
extern crate safesec_derive;

// ===========================================================================
// Modules
// ===========================================================================


pub mod error;
pub mod network;
pub mod prelude;
pub mod protocol;
pub mod service;
pub mod storage;
pub mod util;


// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports

use std::io;
use std::mem;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::RwLock;

// Third-party imports

use futures::{Async, AsyncSink, Future, Poll, Sink, Stream, task};
use futures::stream::SplitSink;
use rmpv::Value;
use tokio_core::net::TcpListener;
use tokio_core::reactor::Core;
use tokio_io::AsyncRead;
use tokio_service::Service;

// Local imports

use network::codec::MsgPackCodec;
use network::rpc::Message;
use network::server::Server;
use service::rpcservice::{RpcService, RpcState, ServiceWithShutdown};
use storage::KeyFileBuilder;
use storage::lmdb::KeyFile;


// ===========================================================================
// Config
// ===========================================================================


pub struct Config {
    pub name: String,
    pub dbdir: PathBuf,
    pub bindaddr: SocketAddr,
}


// ===========================================================================
// SendMessage future
// ===========================================================================


#[derive(Debug)]
enum SendMessageState {
    Start,
    Full(Value),
    Waiting,
    Empty,
}


// Custom send future
pub struct SendMessage<S, M> {
    state: SendMessageState,
    writer: SplitSink<S>,
    msg_stream: M,
}


pub fn send_message<S, M>(writer: SplitSink<S>, msg_stream: M)
    -> SendMessage<S, M>
{
    SendMessage {
        state: SendMessageState::Start,
        writer: writer,
        msg_stream: msg_stream,
    }
}


impl<S, M> SendMessage<S, M>
where
    S: Sink<SinkItem = Value, SinkError = io::Error>,
    M: Stream<Item = Value, Error = io::Error>,
{
    fn start_send(&mut self, msg: Value) -> Poll<(), io::Error>
    {
        match self.writer.start_send(msg)? {
            AsyncSink::NotReady(msg) => {
                self.state = SendMessageState::Full(msg);
                task::current().notify();
                Ok(Async::NotReady)
            }
            AsyncSink::Ready => self.poll_complete(),
        }
    }

    fn poll_complete(&mut self) -> Poll<(), io::Error>
    {
        let poll_result = self.writer.poll_complete()?;
        if let Async::Ready(()) = poll_result {
            self.state = SendMessageState::Start;
            task::current().notify();
        } else {
            self.state = SendMessageState::Waiting;
        }
        Ok(Async::NotReady)
    }
}


impl<S, M> Future for SendMessage<S, M>
where
    S: Sink<SinkItem = Value, SinkError = io::Error>,
    M: Stream<Item = Value, Error=io::Error>,
{
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error>
    {
        let state = mem::replace(&mut self.state, SendMessageState::Empty);
        match state {
            SendMessageState::Start => {
                let poll_future = self.msg_stream.poll()?;
                match poll_future {
                    Async::Ready(None) => Ok(Async::Ready(())),
                    Async::Ready(Some(msg)) => self.start_send(msg),
                    Async::NotReady => {
                        self.state = SendMessageState::Start;
                        Ok(Async::NotReady)
                    }
                }
            }
            SendMessageState::Full(msg) => {
                self.start_send(msg)
            },
            SendMessageState::Waiting => {
                self.poll_complete()
            },
            SendMessageState::Empty => panic!("Poll on completed SendMessage"),
        }
    }
}


// ===========================================================================
// serve
// ===========================================================================


pub fn serve(config: &Config) -> io::Result<()>
{
    // Create event loop
    let mut core = Core::new()?;
    let handle = core.handle();

    // Open database, creating it if it doesn't exist
    let keyfile = KeyFile::new("temp", Some(config.dbdir.as_path()));
    let db = Rc::new(RwLock::new(keyfile));

    // Create server stream, binding to configured bind address
    let listener = match TcpListener::bind(&config.bindaddr, &handle) {
        Ok(l) => l,
        Err(e) => {
            let errmsg = format!(
                "Unable to bind to address {}: {}",
                config.bindaddr,
                e
            );
            let err =
                io::Error::new(io::ErrorKind::ConnectionRefused, errmsg);
            return Err(err);
        }
    };

    // Create server
    let server = Server::new(handle.clone(), listener.incoming(), 1);
    let tx = server.control();

    // Set up server future
    let server = server
        .for_each(|(socket, _peer_addr)| {
            let (writer, reader) = socket.framed(MsgPackCodec).split();
            let mut service = RpcService::new();
            let mut rpcstate = RpcState::new(db.clone());
            service.set_server_control(tx.clone(), handle.clone());
            rpcstate.set_server_control(tx.clone(), handle.clone());

            let responses = reader
                .and_then(move |req| {
                    // println!("calling service");
                    service.call(req)
                })

                // Don't send any None values
                .filter(|v| v.is_some())

                // Process the message and generate a response
                .and_then(move |v| {
                    // println!("processing message");
                    let msg = Message::from(v.unwrap()).unwrap();
                    rpcstate.process_message(msg)
                })

                // Don't send any None values
                .filter(|v| v.is_some())

                // Unwrap Some(Value)
                .map(|some_val| some_val.unwrap());

            let server = send_message(writer, responses).map_err(|_| ());

            handle.spawn(server);

            Ok(())
        })
        .map_err(|e| {
            eprintln!("ERROR HAPPENED: {}", e);
            io::Error::new(io::ErrorKind::Other, "connection handler error")
        });

    core.run(server)
}


// ===========================================================================
//
// ===========================================================================
