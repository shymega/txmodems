//! Collection of protocol bytes for internal usage in crate.
#![allow(dead_code)] // temporqary

const NUL: &[u8] = b"\x00";
const SOH: &[u8] = b"\x01";
const STX: &[u8] = b"\x02";
const EOT: &[u8] = b"\x04";
const ACK: &[u8] = b"\x06";
const ACK2: &[u8] = b"\x86";
const DLE: &[u8] = b"\x10";
const NAK: &[u8] = b"\x15";
const CAN: &[u8] = b"\x18";
const CAN2: &[u8] = b"\x98";
const CRC: &[u8; 1] = b"C";
const CRC2: &[u8] = b"\xc3";
const CRC3: &[u8] = b"\x83";
const ABT: &[u8; 1] = b"a";
