//! Parse the memcached binary format
//!
//! Based on the description provided here:
//! https://github.com/memcached/memcached/wiki/BinaryProtocolRevamped
//!
//! and a bit of `tcpdump -i lo udp port 11211 -vv -X`

#![allow(unused)] // For now

pub mod serialize;

/// Data format description for a parsed packet
#[derive(Debug, Eq, PartialEq)]
pub enum Value {
    Get(Vec<u8>),
    Set(Vec<u8>, Vec<u8>),
    Value(Vec<u8>, Vec<u8>, Vec<u8>),
    Stored,
    NotStored,
    NoReply,
}

/// A decoder error
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum DecodeError {
    InvalidOpCode,
    UnexpectedEof,
}
