#![allow(dead_code)]

use core2::io::{Read, Result, Write};
pub use utils::*;

#[derive(Copy, Clone, Debug)]
pub enum ChecksumKind {
    Standard,
    Crc16,
}

#[derive(Copy, Clone, Debug)]
pub enum BlockLengthKind {
    Standard = 128,
    OneK = 1024,
}

mod utils {
    use super::{Read, Result};
    use core2::io::ErrorKind;

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
    fn new() -> Self
    where
        Self: Sized;
    fn send<D, R>(&mut self, dev: &mut D, data: &mut R) -> Result<((), usize)>
    where
        D: Read + Write,
        R: Read;
    fn receive<D, W>(
        &mut self,
        dev: &mut D,
        out: &mut W,
        checksum: ChecksumKind,
    ) -> Result<((), usize)>
    where
        D: Read + Write,
        W: Write;
    fn init_send<D>(&mut self, dev: &mut D) -> Result<()>
    where
        D: Read + Write;
    fn finish_send<D>(&mut self, dev: &mut D) -> Result<()>
    where
        D: Read + Write;
    fn send_stream<D, R>(&mut self, dev: &mut D, data: &mut R) -> Result<()>
    where
        D: Read + Write,
        R: Read;
}
