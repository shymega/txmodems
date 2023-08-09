#![allow(dead_code)]

use alloc::boxed::Box;

use thiserror_no_std::Error;
use anyhow::Result;
use core2::io::{Error, Read, Write};
pub use utils::*;

#[derive(Default, Copy, Clone, Debug)]
pub enum ChecksumKind {
    #[default]
    Standard,
    Crc16,
}

#[derive(Default, Copy, Clone, Debug)]
pub enum BlockLengthKind {
    #[default]
    Standard = 128,
    OneK = 1024,
}

/// Enum of various `Error` variants.
#[derive(Debug, Error)]
pub enum ModemError {
    /// Boxed `core2::io::Error`, used for storing I/O errors.
    #[error("Error during I/O on the channel.")]
    Io(#[from] Error),

    /// The number of communications errors exceeded `max_errors` in a single
    /// transmission.
    #[error("Too many errors, aborting - max errors: {errors}")]
    ExhaustedRetries {
        errors: Box<u32>
    },

    /// The transmission was canceled by the other end of the channel.
    #[error("Cancelled by the other party.")]
    Canceled,
}

pub type ModemResult<T, E = ModemError> = Result<T, E>;

mod utils {
    use super::Read;
    use core2::io::{Result, ErrorKind};

    pub fn calc_checksum(data: &[u8]) -> u8 {
        data.iter().fold(0, |x, &y| x.wrapping_add(y))
    }

    pub fn calc_crc(data: &[u8]) -> u16 {
        crc16::State::<crc16::XMODEM>::calculate(data)
    }

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

pub trait Modem {
    /// Return a new instance of the `Xmodem` struct.
    fn new() -> Self
    where
        Self: Sized;

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
    fn send<D, R>(&mut self, dev: &mut D, inp: &mut R) -> ModemResult<()>
    where
        D: Read + Write,
        R: Read;

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
    fn receive<D, W>(
        &mut self,
        dev: &mut D,
        out: &mut W,
        checksum: ChecksumKind,
    ) -> ModemResult<()>
    where
        D: Read + Write,
        W: Write;

    /// Internal function for initializing a transmission.
    /// FIXME: Document.
    fn init_send<D>(&mut self, dev: &mut D) -> ModemResult<()>
    where
        D: Read + Write;

    /// Internal function for finishing a transmission.
    /// FIXME: Document.
    fn finish_send<D>(&mut self, dev: &mut D) -> ModemResult<()>
    where
        D: Read + Write;

    /// Internal function for sending a stream.
    /// FIXME: Document.
    fn send_stream<D, R>(&mut self, dev: &mut D, inp: &mut R) -> ModemResult<()>
    where
        D: Read + Write,
        R: Read;
}
