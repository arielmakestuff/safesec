// server.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports

use std::io;
use std::net::SocketAddr;

// Third-party imports

use futures::{Async, Future, Poll, Sink, Stream};
use futures::sync::mpsc;
use tokio_core::net::{Incoming, TcpStream};
use tokio_core::reactor::Handle;

// Local imports


// ===========================================================================
// Server
// ===========================================================================


pub enum ServerMessage {
    Send(TcpStream, SocketAddr),
    Shutdown,
}


// Helper function to send a server message via a channel
pub fn sendmsg<T>(loop_handle: &Handle, control: mpsc::Sender<T>, msg: T)
where
    T: 'static,
{
    let f = control.send(msg).then(|_| Ok(()));
    loop_handle.spawn(f);
}


// Helper function to send a shutdown message via a channel
pub fn shutdown(loop_handle: &Handle, control: mpsc::Sender<ServerMessage>) {
    sendmsg::<ServerMessage>(loop_handle, control, ServerMessage::Shutdown);
}


pub struct Server {
    control: (mpsc::Sender<ServerMessage>, mpsc::Receiver<ServerMessage>),
    handler: (mpsc::UnboundedSender<(TcpStream, SocketAddr)>,
              mpsc::UnboundedReceiver<(TcpStream, SocketAddr)>),
    listener: Incoming, // From TcpListener::incoming(),
    loop_handle: Handle,
}


impl Server {
    pub fn new(
        loop_handle: Handle,
        stream: Incoming,
        channel_size: usize,
    ) -> Self {
        let control = mpsc::channel::<ServerMessage>(channel_size);
        let handler = mpsc::unbounded::<(TcpStream, SocketAddr)>();

        Self {
            control: control,
            handler: handler,
            listener: stream,
            loop_handle: loop_handle,
        }
    }

    pub fn control(&self) -> mpsc::Sender<ServerMessage> {
        let (ref tx, _) = self.control;
        tx.clone()
    }

    fn poll_msg(
        &mut self,
    ) -> Poll<Option<(TcpStream, SocketAddr)>, io::Error> {
        let msg_poll;
        {
            let (_, ref mut rx) = self.control;
            msg_poll = rx.poll();
        }

        match msg_poll {
            Err(()) => {
                let errmsg = "Error receiving server command";
                let err = io::Error::new(io::ErrorKind::Other, errmsg);
                Err(err)
            }

            // Nothing more will be streamed, close the server down
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),

            // Send socket through handler channel
            Ok(Async::Ready(Some(ServerMessage::Send(socket, addr)))) => {
                let (ref mut tx, _) = self.handler;
                let tx = tx.clone();
                let f = tx.send((socket, addr)).then(|_| Ok(()));
                self.loop_handle.spawn(f);
                Ok(Async::NotReady)
            }

            Ok(Async::Ready(Some(ServerMessage::Shutdown))) => {
                {
                    let (_, ref mut rx) = self.handler;
                    rx.close();
                }
                Ok(Async::NotReady)
                // self.poll_msg()
            }

            Ok(Async::NotReady) => Ok(Async::NotReady),
        }
    }

    fn poll_listener(
        &mut self,
    ) -> Poll<Option<(TcpStream, SocketAddr)>, io::Error> {
        let (ref tx, _) = self.control;
        let tx = tx.clone();
        let listener_poll = self.listener.poll();
        match listener_poll {
            Err(e) => Err(e),

            Ok(Async::NotReady) => Ok(Async::NotReady),

            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),

            Ok(Async::Ready(Some((stream, addr)))) => {
                sendmsg(
                    &self.loop_handle,
                    tx,
                    ServerMessage::Send(stream, addr),
                );
                Ok(Async::NotReady)
            }
        }
    }

    fn poll_handler(
        &mut self,
    ) -> Poll<Option<(TcpStream, SocketAddr)>, io::Error> {
        let (_, ref mut rx) = self.handler;
        let handler_poll = rx.poll();
        match handler_poll {
            Err(()) => {
                let errmsg = "Error receiving socket";
                let err = io::Error::new(io::ErrorKind::Other, errmsg);
                Err(err)
            }

            Ok(v) => Ok(v),
        }
    }
}


impl Stream for Server {
    type Item = (TcpStream, SocketAddr);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // Poll for a message first
        let msg = self.poll_msg();

        // If no messages, check listener
        if let Ok(Async::NotReady) = msg {
            let res = self.poll_listener();
            match res {
                // If listener not ready, check handler
                Ok(Async::NotReady) |
                Ok(Async::Ready(None)) => self.poll_handler(),
                _ => res,
            }
        }
        // Return message
        else {
            msg
        }
    }
}


// ===========================================================================
//
// ===========================================================================
