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


// ===========================================================================
// serve
// ===========================================================================


// ------------------------------------------------------------------
// TODO: Remove this function and imports once cli options have been
// implemented
// ------------------------------------------------------------------

extern crate chrono;
extern crate tempdir;
use chrono::prelude::*;
use std::fs;
use storage::KeyFileBuilder;
use storage::lmdb::KeyFile;
use tempdir::TempDir;

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


use std::mem;


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


// ------------------------------------------------------------------
// End TODO
// ------------------------------------------------------------------


pub fn serve() -> io::Result<()>
{
    // Create event loop
    let mut core = Core::new()?;
    let handle = core.handle();

    // Create database
    let tmpdir = _mktempdir();
    let db = Rc::new(RwLock::new(_create_db(&tmpdir)));

    // Create server stream
    // pub fn new(address: &SocketAddr, loop_handle: Handle, channel_size: usize) -> Self {
    // Bind to localhost only
    let address = "127.0.0.1:12345".parse().unwrap();
    let listener = match TcpListener::bind(&address, &handle) {
        Ok(l) => l,
        Err(e) => {
            let errmsg =
                format!("Unable to bind to address {}: {}", address, e);
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
                    service.call(req)
                })

                // Don't send any None values
                .filter(|v| {
                    v.is_some()
                })

                // Process the message and generate a response
                .and_then(move |v| {
                    let msg = Message::from(v.unwrap()).unwrap();
                    rpcstate.process_message(msg)
                })

                // Don't send any None values
                // .filter(|v| v.is_some())
                .filter(|v| {
                    v.is_some()
                })

                // Unwrap Some(Value)
                .map(|some_val| {
                    some_val.unwrap()
                });

            let server = send_message(writer, responses).map_err(|_| ());
            // let server = writer.send_all(responses).then(|_| {
            //     println!("Finished sending response");
            //     Ok(())
            // });

            handle.spawn(server);

            Ok(())
        })
        .map_err(|_| {
            io::Error::new(io::ErrorKind::Other, "connection handler error")
        });

    core.run(server)
}


// ===========================================================================
//
// ===========================================================================
