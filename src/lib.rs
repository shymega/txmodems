//! This crate implements various MODEM file transfer protocols.
#![no_std]
#![deny(
    warnings,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    clippy::all,
    clippy::cargo,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    unused_extern_crates,
    variant_size_differences
)]

#[macro_use]
extern crate log;

mod xmodem;

pub mod variants {
    #[cfg(feature = "xmodem")]
    pub mod xmodem {
        pub use crate::xmodem::*;
        pub mod consts {
            //! Collection of protocol bytes for internal usage in  `txmodems` (XMODEM-specific)

            pub const NUL: u8 = 0x00;
            pub const SOH: u8 = 0x01;
            pub const STX: u8 = 0x02;
            pub const EOT: u8 = 0x04;
            pub const ACK: u8 = 0x06;
            pub const ACK2: u8 = 0x86;
            pub const DLE: u8 = 0x10;
            pub const NAK: u8 = 0x15;
            pub const CAN: u8 = 0x18;
            pub const CAN2: u8 = 0x98;
            pub const CRC: u8 = 0x43;
            pub const CRC2: u8 = 0xC3;
            pub const CRC3: u8 = 0x83;
            pub const ABT: u8 = 0x61;
        }
    }
}
