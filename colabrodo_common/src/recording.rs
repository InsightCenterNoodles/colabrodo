//!
//! Reads and writes NOODLES messages to disk
//!
//! # Disk Format
//!
//! All in CBOR. Note that the file is an array
//! of CBOR lists. The array could be definite or indefinite in length.
//!
//! // File
//! [
//!     packet,
//!     ...
//! ]
//!
//! // packet
//! [
//!     packet_type : integer,
//!     content1 : any,
//!     content2 : any,
//!     ...
//! ]
//!
//! # Packet Types
//!
//! Type 1:
//! CBOR from-server message content
//!     timestamp : integer, seconds from start,
//!     message : bytes, message from the server,
//!
//! Type 2:
//! Labelled timestamp w/ utf8 string identifier
//!     timestamp : integer, seconds from start,
//!     marker_name : string
//!
//! Type 3:
//! Buffer location
//!     content : string, relative location of buffer file
//!

use std::mem;

#[derive(Debug, Clone, Copy)]
pub struct PacketStamp(pub u32);

#[derive(Debug)]
pub enum Packet {
    DropMarker(PacketStamp, String),
    WriteCBOR(PacketStamp, Vec<u8>),
    BufferLocation(String),
}

impl Packet {
    pub fn message_stamp(&self) -> u8 {
        match &self {
            Packet::DropMarker(_, _) => 1,
            Packet::WriteCBOR(_, _) => 2,
            Packet::BufferLocation(_) => 3,
        }
    }
}

pub fn parse_record(mut value: ciborium::value::Value) -> Option<Packet> {
    let array = value.as_array_mut()?;
    let id: i128 = array.get(0)?.as_integer()?.into();
    let id: u8 = id.try_into().ok()?;

    fn value_to_int<T>(value: &ciborium::value::Value) -> Option<T>
    where
        T: TryFrom<ciborium::value::Integer>,
    {
        value.as_integer()?.try_into().ok()
    }

    match id {
        1 => {
            let timestamp: u32 = value_to_int::<u32>(array.get(1)?)?;

            let mut swapper = String::new();

            mem::swap(&mut swapper, array.get_mut(2)?.as_text_mut()?);

            Some(Packet::DropMarker(PacketStamp(timestamp), swapper))
        }
        2 => {
            let timestamp: u32 = value_to_int::<u32>(array.get(1)?)?;

            let mut swapper = Vec::<u8>::new();

            mem::swap(&mut swapper, array.get_mut(2)?.as_bytes_mut()?);

            Some(Packet::WriteCBOR(PacketStamp(timestamp), swapper))
        }
        3 => {
            let mut swapper = String::new();

            mem::swap(&mut swapper, array.get_mut(1)?.as_text_mut()?);

            Some(Packet::BufferLocation(swapper))
        }
        _ => None,
    }
}

/// Starts a record array using an indefinite array byte. Be sure to call [end_pack] when done.
pub fn start_pack(sink: &mut impl std::io::Write) {
    sink.write_all(&[0x9f])
        .expect("unable to write start marker to stream");
}

/// Ends a record array using an indefinite array stop byte. Be sure to have called [start_pack] beforehand at some point.
pub fn end_pack(mut sink: impl std::io::Write) {
    sink.write_all(&[0xff])
        .expect("unable to write end marker to stream");
}

/// Add a record to the pack
pub fn pack_record(record: Packet, sink: impl std::io::Write) {
    let id = record.message_stamp();

    let value: Vec<ciborium::value::Value> = match record {
        Packet::DropMarker(t, v) => {
            vec![id.into(), t.0.into(), v.into()]
        }
        Packet::WriteCBOR(t, v) => {
            vec![id.into(), t.0.into(), v.into()]
        }
        Packet::BufferLocation(loc) => {
            vec![id.into(), loc.into()]
        }
    };

    ciborium::ser::into_writer(&value, sink)
        .expect("unable to write packet to file");
}
