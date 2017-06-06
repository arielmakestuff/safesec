// msgpack.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

// ===========================================================================
// Externs
// ===========================================================================

// Tokio crates
extern crate bytes;
extern crate tokio_io;

extern crate serde;
extern crate rmp_serde as rmps;


// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports
use std::io;
use std::str;

// Third-party imports
use self::bytes::BytesMut;
use self::rmps::{Deserializer, Serializer};
use self::rmps::decode;
use self::serde::{Serialize, Deserialize};
use self::tokio_io::codec::{Decoder, Encoder};

// Local imports


// ===========================================================================
// Message
// ===========================================================================


#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Message {
    text: String
}


// ===========================================================================
// Codec
// ===========================================================================


pub struct MsgPackCodec;


impl MsgPackCodec {

    fn handle_decode_error(err: decode::Error) -> Option<io::Error> {
        match err {
            decode::Error::InvalidDataRead(e) => {
                match e.kind() {
                    io::ErrorKind::UnexpectedEof => None,
                    _ => Some(e)
                }
            },
            decode::Error::InvalidMarkerRead(e) => Some(e),
            decode::Error::TypeMismatch(_) => {
                let errmsg = "msgpack type mismatch".to_string();
                Some(io::Error::new(io::ErrorKind::InvalidData, errmsg))
            },
            decode::Error::OutOfRange => {
                let errmsg = "msgpack value out of range".to_string();
                Some(io::Error::new(io::ErrorKind::InvalidData, errmsg))
            },
            decode::Error::LengthMismatch(l) => {
                let errmsg = format!("msgpack length mismatch: {}", l);
                Some(io::Error::new(io::ErrorKind::InvalidData, errmsg))
            },
            decode::Error::Uncategorized(errmsg) => {
                Some(io::Error::new(io::ErrorKind::Other, errmsg))
            },
            decode::Error::Syntax(e) => {
                let errmsg = format!("msgpack syntax error: {}", e);
                Some(io::Error::new(io::ErrorKind::InvalidData, errmsg))
            },
            decode::Error::Utf8Error(utferr) => {
                let invalid_byte = utferr.valid_up_to();
                let errmsg = format!("msgpack utf-8 error: invalid byte starts at {}",
                                     invalid_byte);
                Some(io::Error::new(io::ErrorKind::InvalidData, errmsg))
            },
            decode::Error::DepthLimitExceeded => {
                let errmsg = "DepthLimitExceeded".to_string();
                Some(io::Error::new(io::ErrorKind::Other, errmsg))
            },
        }
    }
}


impl Decoder for MsgPackCodec {
    type Item = Message;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<Message>> {
        let result;
        let curpos: usize;

        // If no data has been given yet, ask for data to be sent
        if buf.len() == 0 {
            return Ok(None);
        }

        // Attempt to deserialize the current buffer
        {
            let cursor = io::Cursor::new(&buf[..]);
            let mut de = Deserializer::new(cursor);
            result = Message::deserialize(&mut de);
            curpos = de.position() as usize;
        }

        // Discard read bytes
        buf.split_to(curpos);

        match result {
            Ok(m) => Ok(Some(m)),
            Err(e) => {
                match Self::handle_decode_error(e) {
                    Some(err) => Err(err),
                    None => Ok(None)
                }
            }
        }
    }
}


impl Encoder for MsgPackCodec {
    type Item = Message;
    type Error = io::Error;

    fn encode(&mut self, msg: Message, buf: &mut BytesMut) -> io::Result<()> {
        let mut tmpbuf = Vec::new();
        msg.serialize(&mut Serializer::new(&mut tmpbuf)).unwrap();
        buf.extend_from_slice(&tmpbuf[..]);
        Ok(())
    }
}


// ===========================================================================
// Tests
// ===========================================================================


#[cfg(test)]
mod tests {

    use super::serde::Serialize;
    use super::rmps::Serializer;
    use super::{BytesMut, Message, MsgPackCodec};
    use super::bytes::buf::FromBuf;
    use super::{Encoder, Decoder};

    // --------------------
    // Decode tests
    // --------------------

    #[test]
    fn decode_one_message() {
        let mut buf = Vec::new();
        let msg = Message { text: String::from("ANSWER") };
        msg.serialize(&mut Serializer::new(&mut buf)).unwrap();

        let mut codec = MsgPackCodec;
        let mut buf = BytesMut::from_buf(buf);
        let val = codec.decode(&mut buf).unwrap();
        let msg = match val {
            Some(m) => m,
            _ => Message { text: "".to_string() }
        };

        assert_eq!(msg.text, "ANSWER");
    }


    #[test]
    fn decode_incomplete_message() {
        // --------------------
        // GIVEN
        // --------------------
        // A message pack serialized message
        let mut buf = Vec::new();
        let msg = Message { text: String::from("ANSWER") };
        msg.serialize(&mut Serializer::new(&mut buf)).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // the serialized message is cut in half and and the half-message is
        // decoded

        // Cut serialized message in half
        let length = buf.len();
        let newlength = length / 2;
        assert!(newlength > 0);
        let newbuf = Vec::from(&buf[..newlength]);

        // Decode the incomplete message
        let mut codec = MsgPackCodec;
        let mut buf = BytesMut::from_buf(newbuf);

        // --------------------
        // THEN
        // --------------------
        // Ok(None) is returned (signifying more data is needed to decode)
        if let Ok(None) = codec.decode(&mut buf) {
            assert!(true);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn decode_complete_and_incomplete() {
        // --------------------
        // GIVEN
        // --------------------
        // Two message pack serialized messages
        let mut buf = Vec::new();
        let mut buf2 = Vec::new();
        let msg1 = Message { text: String::from("ANSWER ONE") };
        let msg2 = Message { text: String::from("ANSWER TWO") };
        msg1.serialize(&mut Serializer::new(&mut buf)).unwrap();
        msg2.serialize(&mut Serializer::new(&mut buf2)).unwrap();

        // --------------------
        // WHEN
        // --------------------
        // One complete and another incomplete messagepack messages are sent to
        // decode in a single buffer

        // Cut one of the serialized messages in half
        let buffer_length = buf.len();
        let length = buf2.len();
        let newlength = length / 2;
        assert!(newlength > 0);

        // Join the two messages together
        buf.extend_from_slice(&buf2[..newlength]);

        // Create the buffer
        let mut codec = MsgPackCodec;
        let mut buf = BytesMut::from_buf(buf);

        // --------------------
        // THEN
        // --------------------
        // The first complete message is returned, and the buffer contains the
        // incomplete second message
        let val = codec.decode(&mut buf).unwrap();
        let msg = match val {
            Some(m) => m,
            _ => Message { text: "".to_string() }
        };

        assert_eq!(msg.text, "ANSWER ONE");
        assert!(buf.len() < buffer_length);
        assert_eq!(buf.len(), newlength);
        assert_eq!(&buf[..], &buf2[..newlength]);
    }

    #[test]
    fn decode_empty_buffer() {
        // --------------------
        // GIVEN
        // --------------------
        // An empty buffer
        let mut buf = BytesMut::from_buf(Vec::new());
        assert_eq!(buf.len(), 0);

        // --------------------
        // WHEN
        // --------------------
        // Decoding the buffer
        let mut codec = MsgPackCodec;
        let result = codec.decode(&mut buf);

        // --------------------
        // THEN
        // --------------------
        // Ok(None) is returned (ie ask for data to be sent)
        match result {
            Ok(None) => assert!(true),
            _ => assert!(false)
        };
    }

    // --------------------
    // Encode tests
    // --------------------

    #[test]
    fn encode_message() {
        // --------------------
        // GIVEN
        // --------------------
        // A message and an empty buffer
        let msg = Message { text: "Hello".to_string() };
        let buf = Vec::new();
        let mut codec = MsgPackCodec;

        // --------------------
        // WHEN
        // --------------------
        // The message is serialized into messagepack
        let mut buf = BytesMut::from(&buf[..]);
        match codec.encode(msg.clone(), &mut buf) {
            Ok(()) => assert!(true),
            Err(_) => assert!(false)
        };

        // --------------------
        // THEN
        // --------------------
        // The serialized message can be deserialized back into a message
        let val = codec.decode(&mut buf).unwrap();
        let result = match val {
            Some(m) => m,
            _ => Message { text: "".to_string() }
        };

        assert_eq!(msg.text, result.text);
    }
}


// ===========================================================================
//
// ===========================================================================
