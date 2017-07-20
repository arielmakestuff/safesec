// server.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports

use std::io;

// Third-party imports

use futures::{Async, Future, Poll, Sink, Stream};
use futures::sync::mpsc;
use tokio_core::net::Incoming;
use tokio_core::reactor::Handle;

// Local imports


// ===========================================================================
// WIP Server
// ===========================================================================


// Helper function to send a server message via a channel
pub fn sendmsg<T>(loop_handle: &Handle, control: mpsc::Sender<T>, msg: T)
where
    T: 'static,
{
    let f = control.send(msg).then(|_| Ok(()));
    loop_handle.spawn(f);
}


pub enum ServerTypeMessage<M> {
    Send(M),
    Shutdown,
}


pub enum ServerTypeChannel {
    Control,
    Handler,
}


pub trait ServerType<S>
where
    S: Stream,
    S::Item: 'static,
{
    fn loop_handle(&self) -> Handle;
    fn channel_error(&self, ServerTypeChannel) -> S::Error;
    fn control_channel(&mut self)
        -> (mpsc::Sender<ServerTypeMessage<S::Item>>,
            &mut mpsc::Receiver<ServerTypeMessage<S::Item>>);
    fn handler_channel(&mut self)
        -> (mpsc::UnboundedSender<S::Item>,
            &mut mpsc::UnboundedReceiver<S::Item>);
    fn listener_channel(&mut self) -> &mut S;

    fn poll_msg(&mut self) -> Poll<Option<S::Item>, S::Error>
    {
        println!("Poll message");
        let msg_poll;
        {
            let (_, mut rx) = self.control_channel();
            msg_poll = rx.poll();
        }

        match msg_poll {
            Err(()) => {
                Err(self.channel_error(ServerTypeChannel::Control))
            }

            // Nothing more will be streamed, close the server down
            Ok(Async::Ready(None)) => {
                println!("Shutdown now!!");
                Ok(Async::Ready(None))
            }

            // Send socket through handler channel
            Ok(Async::Ready(Some(ServerTypeMessage::Send(m)))) => {
                let (tx, _) = self.handler_channel();
                let f = tx.send(m).then(|_| Ok(()));
                self.loop_handle().spawn(f);
                Ok(Async::NotReady)
            }

            Ok(Async::Ready(Some(ServerTypeMessage::Shutdown))) => {
                println!("Shutdown received, closing command channel");
                let (_, mut rx) = self.handler_channel();
                rx.close();
                Ok(Async::NotReady)
                // self.poll_msg()
            }

            Ok(Async::NotReady) => Ok(Async::NotReady),
        }
    }

    fn poll_listener(&mut self) -> Poll<Option<S::Item>, S::Error>
    {
        println!("Poll listener");
        let (tx, _) = self.control_channel();
        let listener_poll = self.listener_channel().poll();
        match listener_poll {
            Err(e) => Err(e),

            Ok(Async::NotReady) => Ok(Async::NotReady),

            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),

            Ok(Async::Ready(Some(m))) => {
                sendmsg(&self.loop_handle(), tx, ServerTypeMessage::Send(m));
                Ok(Async::NotReady)
            }
        }
    }

    fn poll_handler(&mut self) -> Poll<Option<S::Item>, S::Error>
    {
        println!("Poll handler");
        let handler_poll;
        {
            let (_, mut rx) = self.handler_channel();
            handler_poll = rx.poll();
        }
        match handler_poll {
            Err(()) => {
                Err(self.channel_error(ServerTypeChannel::Handler))
            }

            Ok(v) => Ok(v),
        }
    }
}


pub struct MyServer<S>
where
    S: Stream,
    S::Item: 'static,
{
    control: (mpsc::Sender<ServerTypeMessage<S::Item>>,
              mpsc::Receiver<ServerTypeMessage<S::Item>>),
    handler:
        (mpsc::UnboundedSender<S::Item>, mpsc::UnboundedReceiver<S::Item>),
    listener: S,
    loop_handle: Handle,
}


impl<S> MyServer<S>
where
    S: Stream,
    S::Item: 'static,
{
    pub fn new(loop_handle: Handle, stream: S, channel_size: usize) -> Self
    {
        let control =
            mpsc::channel::<ServerTypeMessage<S::Item>>(channel_size);
        let handler = mpsc::unbounded::<S::Item>();

        Self {
            control: control,
            handler: handler,
            listener: stream,
            loop_handle: loop_handle,
        }
    }

    fn loop_handle(&self) -> Handle
    {
        self.loop_handle.clone()
    }

    fn control_channel(&mut self)
        -> (mpsc::Sender<ServerTypeMessage<S::Item>>,
            &mut mpsc::Receiver<ServerTypeMessage<S::Item>>)
    {
        let (ref tx, ref mut rx) = self.control;
        (tx.clone(), rx)
    }

    fn handler_channel(&mut self)
        -> (mpsc::UnboundedSender<S::Item>,
            &mut mpsc::UnboundedReceiver<S::Item>)
    {
        let (ref tx, ref mut rx) = self.handler;
        (tx.clone(), rx)
    }

    fn listener_channel(&mut self) -> &mut S
    {
        &mut self.listener
    }
}


impl ServerType<Incoming> for MyServer<Incoming> {
    fn channel_error(&self, channel: ServerTypeChannel) -> io::Error
    {
        match channel {
            ServerTypeChannel::Control => {
                let errmsg = "Error receiving server command";
                let err = io::Error::new(io::ErrorKind::Other, errmsg);
                err
            }
            ServerTypeChannel::Handler => {
                let errmsg = "Error receiving socket";
                let err = io::Error::new(io::ErrorKind::Other, errmsg);
                err
            }
        }
    }

    fn loop_handle(&self) -> Handle
    {
        unreachable!()
    }

    fn control_channel(&mut self) ->
        (mpsc::Sender<ServerTypeMessage<<Incoming as Stream>::Item>>,
         &mut mpsc::Receiver<ServerTypeMessage<<Incoming as Stream>::Item>>)
    {
        unreachable!()
    }

    fn handler_channel(&mut self)
        -> (mpsc::UnboundedSender<<Incoming as Stream>::Item>,
            &mut mpsc::UnboundedReceiver<<Incoming as Stream>::Item>)
    {
        unreachable!()
    }

    fn listener_channel(&mut self) -> &mut Incoming
    {
        unreachable!()
    }
}


impl Stream for MyServer<Incoming> {
    type Item = <Incoming as Stream>::Item;
    type Error = <Incoming as Stream>::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error>
    {
        println!("Server Poll!");
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
