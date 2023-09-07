#![allow(dead_code)]

use alloc::boxed::Box;
use alloc::string::String;

use anyhow::Result;
use core2::io::{Error, Read, Write};
use thiserror_no_std::Error;
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
    ExhaustedRetries { errors: Box<u32> },

    /// The transmission was canceled by the other end of the channel.
    #[error("Cancelled by the other party.")]
    Canceled,
}

pub type ModemResult<T, E = ModemError> = Result<T, E>;

mod utils {
    use super::Read;
    use core2::io::{ErrorKind, Result};

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

pub trait ModemTrait {
    /// Return a new instance of the `Xmodem` struct.
    fn new() -> Self
    where
        Self: Sized;
}

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

#[allow(dead_code)] // TODO: Temporarily allow this lint, whilst I work out YMODEM support.
pub trait YModemTrait: ModemTrait {
    fn recv<D: Read + Write, W: Write>(
        &mut self,
        dev: &mut D,
        out: &mut W,
        file_name: &mut String,
        file_size: &mut u32,
    ) -> ModemResult<()>;
    fn send<D: Read + Write, R: Read>(
        &mut self,
        dev: &mut D,
        inp: &mut R,
        file_name: String,
        file_size: u64,
    ) -> ModemResult<()>;
    fn send_stream<D: Read + Write, R: Read>(
        &mut self,
        dev: &mut D,
        stream: &mut R,
        packets_to_send: u32,
        last_packet_size: u64,
    ) -> ModemResult<()>;
    fn send_start_frame<D: Read + Write>(
        &mut self,
        dev: &mut D,
        file_name: String,
        file_size: u64,
    ) -> ModemResult<()>;
    fn send_end_frame<D: Read + Write>(
        &mut self,
        dev: &mut D,
    ) -> ModemResult<()>;
}
