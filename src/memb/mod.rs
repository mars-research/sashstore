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
    Get(u16, Vec<u8>),
    Set(u16, Vec<u8>, u32, Vec<u8>),
    Value(u16, Vec<u8>, u32, Vec<u8>),
    Stored(u16),
    NotStored(u16),
    NoReply,
}

/// A decoder error
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum DecodeError {
    InvalidOpCode,
    UnexpectedEof,
}
