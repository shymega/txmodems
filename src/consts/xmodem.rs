//! Collection of protocol bytes for internal usage in `txmodems` (XMODEM-specific)
#![allow(dead_code)] // temporary

pub const NUL: &[u8] = b"\x00";
pub const SOH: &[u8] = b"\x01";
pub const STX: &[u8] = b"\x02";
pub const EOT: &[u8] = b"\x04";
pub const ACK: &[u8] = b"\x06";
pub const ACK2: &[u8] = b"\x86";
pub const DLE: &[u8] = b"\x10";
pub const NAK: &[u8] = b"\x15";
pub const CAN: &[u8] = b"\x18";
pub const CAN2: &[u8] = b"\x98";
pub const CRC: &[u8; 1] = b"C";
pub const CRC2: &[u8] = b"\xc3";
pub const CRC3: &[u8] = b"\x83";
pub const ABT: &[u8; 1] = b"a";
