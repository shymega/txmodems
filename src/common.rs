//! Common X/Y/Z-MODEM module

#![allow(dead_code)]


use anyhow::Result;
#[cfg(core2)]
use core2::io::{Error, Read, Write};
#[cfg(embedded_io_async)]
use embedded_io_async::{Error, Read, Write};
use heapless::String;
use thiserror_no_std::Error;
#[cfg(any(core2, embedded_io_async))]
pub use utils::*;

/// Which Checksum is used
#[derive(Default, Copy, Clone, Debug)]
pub enum ChecksumKind {
    /// 1 byte checksum
    #[default]
    Standard,
    /// Cyclic redundany check 16bit
    Crc16,
}

/// Block length 128 byte / 1KiB
#[derive(Default, Copy, Clone, Debug)]
pub enum BlockLengthKind {
    /// 128 byte
    #[default]
    Standard = 128,
    /// 1 KiB
    OneK = 1024,
}

/// Enum of various `Error` variants.
#[derive(Debug, Error)]
pub enum ModemError {
    /// Boxed `core2::io::Error`, used for storing I/O errors.
    #[error("Error during I/O on the channel.")]
    #[cfg(any(core2, embedded_io_async))]
    Io(#[from] Error),

    /// The number of communications errors exceeded `max_errors` in a single
    /// transmission.
    #[error("Too many errors, aborting - max errors: {errors}")]
    ExhaustedRetries {
        /// Errors
        errors: u32
    },

    /// The transmission was canceled by the other end of the channel.
    #[error("Cancelled by the other party.")]
    Canceled,
}

/// Modem Result type
pub type ModemResult<T, E = ModemError> = Result<T, E>;

#[cfg(any(core2, embedded_io_async))]
mod utils {
    use super::Read;
    #[cfg(core2)]
    use core2::io::{ErrorKind, Result};
    #[cfg(embedded_io_async)]
    use embedded_io_async::ErrorKind;

    /// Calculate checksum
    pub fn calc_checksum(data: &[u8]) -> u8 {
        data.iter().fold(0, |x, &y| x.wrapping_add(y))
    }

    /// Calculate cyclic redundancy check
    pub fn calc_crc(data: &[u8]) -> u16 {
        crc16::State::<crc16::XMODEM>::calculate(data)
    }

    /// get byte
    pub fn get_byte<R: Read>(reader: &mut R) -> Result<u8> {
        let mut buff = [0];
        reader.read_exact(&mut buff)?;
        Ok(buff[0])
    }

    /// Turns timeout errors into `Ok(None)`
    pub fn get_byte_timeout<R: Read>(reader: &mut R) -> Result<Option<u8>> {
        match get_byte(reader) {
            Ok(c) => Ok(Some(c)),
            Err(err) => {
                if err.kind() == ErrorKind::TimedOut {
                    Ok(None)
                } else {
                    Err(err)
                }
            }
        }
    }
}

/// constructor trait
pub trait ModemTrait {
    /// Return a new instance of the `modem` struct.
    fn new() -> Self
    where
        Self: Sized;
}

/// Xmodem specific
#[cfg(any(core2, embedded_io_async))]
pub trait XModemTrait: ModemTrait {
    /// Starts the XMODEM transmission.
    ///
    /// `dev` should be the serial communication channel (e.g. the serial device).
    /// `inp` should be the message to send (e.g. a file).
    ///
    /// # Timeouts
    /// This method has no way of setting the timeout of `dev`, so it's up to the caller
    /// to set the timeout of the device before calling this method. Timeouts on receiving
    /// bytes will be counted against `max_errors`, but timeouts on transmitting bytes
    /// will be considered a fatal error.
    fn send<D: Read + Write, R: Read>(
        &mut self,
        dev: &mut D,
        inp: &mut R,
    ) -> ModemResult<()>;

    /// Receive an XMODEM transmission.
    ///
    /// `dev` should be the serial communication channel (e.g. the serial device).
    /// The received data will be written to `out`.
    /// `checksum` indicates which checksum mode should be used; `ChecksumKind::Standard` is
    /// a reasonable default.
    ///
    /// # Timeouts
    /// This method has no way of setting the timeout of `dev`, so it's up to the caller
    /// to set the timeout of the device before calling this method. Timeouts on receiving
    /// bytes will be counted against `max_errors`, but timeouts on transmitting bytes
    /// will be considered a fatal error.
    fn receive<D: Read + Write, W: Write>(
        &mut self,
        dev: &mut D,
        out: &mut W,
        checksum: ChecksumKind,
    ) -> ModemResult<()>;

    /// Internal function for initializing a transmission.
    /// FIXME: Document.
    fn init_send<D: Read + Write>(&mut self, dev: &mut D) -> ModemResult<()>;

    /// Internal function for finishing a transmission.
    /// FIXME: Document.
    fn finish_send<D: Read + Write>(&mut self, dev: &mut D) -> ModemResult<()>;

    /// Internal function for sending a stream.
    /// FIXME: Document.
    fn send_stream<D: Read + Write, R: Read>(
        &mut self,
        dev: &mut D,
        inp: &mut R,
    ) -> ModemResult<()>;
}

/// Ymodem specific trait
#[cfg(any(core2, embedded_io_async))]
#[allow(dead_code)] // TODO: Temporarily allow this lint, whilst I work out YMODEM support.
pub trait YModemTrait: ModemTrait {
    /// Receive an YMODEM transmission.
    ///
    /// `dev` should be the serial communication channel (e.g. the serial device).
    /// The received data will be written to `out`.
    /// `checksum` indicates which checksum mode should be used; `ChecksumKind::Crc16` is
    /// a reasonable default.
    ///
    /// # Timeouts
    /// This method has no way of setting the timeout of `dev`, so it's up to the caller
    /// to set the timeout of the device before calling this method. Timeouts on receiving
    /// bytes will be counted against `max_errors`, but timeouts on transmitting bytes
    /// will be considered a fatal error.
    fn recv<D: Read + Write, W: Write>(
        &mut self,
        dev: &mut D,
        out: &mut W,
        file_name: &mut String<32>,
        file_size: &mut u32,
    ) -> ModemResult<()>;

    /// Starts the YMODEM transmission.
    ///
    /// `dev` should be the serial communication channel (e.g. the serial device).
    /// `inp` should be the message to send (e.g. a file).
    ///
    /// # Timeouts
    /// This method has no way of setting the timeout of `dev`, so it's up to the caller
    /// to set the timeout of the device before calling this method. Timeouts on receiving
    /// bytes will be counted against `max_errors`, but timeouts on transmitting bytes
    /// will be considered a fatal error.
    fn send<D: Read + Write, R: Read>(
        &mut self,
        dev: &mut D,
        inp: &mut R,
        file_name: String<32>,
        file_size: u64,
    ) -> ModemResult<()>;

    /// Internal function for starting a transmission.
    /// FIXME: Document.
    fn start_send<D: Read + Write>(
        &mut self,
        dev: &mut D
    ) -> ModemResult<()>;

    /// Internal function for initializing a transmission.
    /// FIXME: Document.
    fn send_start_frame<D: Read + Write>(
        &mut self,
        dev: &mut D,
        file_name: String<32>,
        file_size: u64,
    ) -> ModemResult<()>;

    /// Internal function for sending a stream.
    /// FIXME: Document.
    fn send_stream<D: Read + Write, R: Read>(
        &mut self,
        dev: &mut D,
        stream: &mut R,
        packets_to_send: u32,
        last_packet_size: u64,
    ) -> ModemResult<()>;

    /// Internal function for finishing a transmission.
    /// FIXME: Document.
    fn finish_send<D: Read + Write>(
        &mut self,
        dev: &mut D,
    ) -> ModemResult<()>;

    /// Internal function for finishing a transmission.
    /// FIXME: Document.
    fn send_end_frame<D: Read + Write>(
        &mut self,
        dev: &mut D,
    ) -> ModemResult<()>;
}
