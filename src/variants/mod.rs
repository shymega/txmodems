//! Feature-flag guarded module for different X/Y/Z-MODEM implementations.

mod api;

#[cfg(feature = "xmodem")]
pub mod xmodem {
    //! XMODEM module for XMODEM communications.
    //! Guarded by the `xmodem` feature flag.
    //! Disabled by default.
    pub(crate) use crate::common;
    pub use crate::variants::api::xmodem::*;

    #[derive(Default, Debug, Copy, Clone)]
    #[repr(u8)]
    #[allow(missing_docs)]
    pub enum Consts {
        NUL = 0x00,
        SOH = 0x01,
        STX = 0x02,
        EOT = 0x04,
        ACK = 0x06,
        ACK2 = 0x86,
        DLE = 0x10,
        NAK = 0x15,
        CAN = 0x18,
        CAN2 = 0x98,
        CRC = 0x43,
        CRC2 = 0xC3,
        CRC3 = 0x83,
        ABT = 0x61,
        #[default]
        Unknown = 0x99,
    }

    impl From<Consts> for u8 {
        fn from(v: Consts) -> Self {
            v as Self
        }
    }

    impl From<u8> for Consts {
        fn from(v: u8) -> Self {
            match v {
                0x00 => Self::NUL,
                0x01 => Self::SOH,
                0x02 => Self::STX,
                0x04 => Self::EOT,
                0x06 => Self::ACK,
                0x86 => Self::ACK2,
                0x10 => Self::DLE,
                0x15 => Self::NAK,
                0x18 => Self::CAN,
                0x98 => Self::CAN2,
                0x43 => Self::CRC,
                0xC3 => Self::CRC2,
                0x83 => Self::CRC3,
                0x61 => Self::ABT,
                _ => Self::Unknown,
            }
        }
    }
}

#[cfg(feature = "ymodem")]
pub mod ymodem {
    //! YMODEM module for YMODEM communications.
    //! Guarded by the `xmodem` feature flag.
    //! Disabled by default.
    pub use crate::variants::api::ymodem::*;

    #[derive(Default, Debug, Copy, Clone)]
    #[repr(u8)]
    #[allow(missing_docs)]
    pub enum Consts {
        SOH = 0x01,
        STX = 0x02,
        EOT = 0x04,
        ACK = 0x06,
        NAK = 0x15,
        CAN = 0x18,
        CRC = 0x43,
        #[default]
        Unknown = 0x99,
    }

    impl From<Consts> for u8 {
        fn from(v: Consts) -> Self {
            v as Self
        }
    }

    impl From<u8> for Consts {
        fn from(v: u8) -> Self {
            match v {
                0x01 => Self::SOH,
                0x02 => Self::STX,
                0x04 => Self::EOT,
                0x06 => Self::ACK,
                0x15 => Self::NAK,
                0x18 => Self::CAN,
                0x43 => Self::CRC,
                _ => Self::Unknown,
            }
        }
    }

}
