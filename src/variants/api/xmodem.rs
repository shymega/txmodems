use alloc::{boxed::Box, vec, vec::Vec};
use core::convert::From;

extern crate alloc;

#[cfg(any(core2, embedded_io_async))]
use crate::common::{
    calc_checksum, calc_crc, get_byte, get_byte_timeout, ModemError,
    ModemResult, ModemTrait, XModemTrait,
};
#[cfg(core2)]
use core2::io::{Read, Write};
#[cfg(embedded_io_async)]
use embedded_io_async::{Read, Write};

use crate::variants::xmodem::{
    common::{BlockLengthKind, ChecksumKind},
    Consts,
};

// TODO: Send CAN byte after too many errors
// TODO: Handle CAN bytes while sending
// TODO: Implement Error for Error

/// `Xmodem` acts as state for XMODEM transfers
#[derive(Default, Debug, Copy, Clone)]
pub struct XModem {
    /// The number of errors that can occur before the communication is
    /// considered a failure. Errors include unexpected bytes and timeouts waiting for bytes.
    pub max_errors: u32,

    /// The byte used to pad the last block. XMODEM can only send blocks of a certain size,
    /// so if the message is not a multiple of that size the last block needs to be padded.
    pub pad_byte: u8,

    /// The length of each block. There are only two options: 128-byte blocks (standard
    ///  XMODEM) or 1024-byte blocks (XMODEM-1k).
    pub block_length: BlockLengthKind,

    /// The checksum mode used by XMODEM. This is determined by the receiver.
    checksum_mode: ChecksumKind,
    errors: u32,
}

#[cfg(any(core2, embedded_io_async))]
impl ModemTrait for XModem {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            max_errors: 16,
            pad_byte: 0x1a,
            block_length: BlockLengthKind::Standard,
            checksum_mode: ChecksumKind::Standard,
            errors: 0,
        }
    }
}

#[cfg(any(core2, embedded_io_async))]
impl XModemTrait for XModem {
    fn send<D, R>(&mut self, dev: &mut D, inp: &mut R) -> ModemResult<()>
    where
        D: Read + Write,
        R: Read,
    {
        self.errors = 0;

        self.init_send(dev)?;

        self.send_stream(dev, inp)?;

        self.finish_send(dev)?;

        Ok(())
    }

    fn receive<D, W>(
        &mut self,
        dev: &mut D,
        out: &mut W,
        checksum: ChecksumKind,
    ) -> ModemResult<()>
    where
        D: Read + Write,
        W: Write,
    {
        self.errors = 0;
        self.checksum_mode = checksum;

        dev.write_all(&[match self.checksum_mode {
            ChecksumKind::Standard => Consts::NAK.into(),
            ChecksumKind::Crc16 => Consts::CRC.into(),
        }])?;

        let mut packet_num: u8 = 1;
        loop {
            match get_byte_timeout(dev)?.map(Consts::from) {
                bt @ Some(Consts::SOH | Consts::STX) => {
                    // Handle next packet
                    let packet_size = match bt {
                        Some(Consts::SOH) => 128,
                        Some(Consts::STX) => 1024,
                        _ => 0, // Why does the compiler need this?
                    };
                    let pnum = get_byte(dev)?; // specified packet number
                    let pnum_1c = get_byte(dev)?; // same, 1's complemented
                                                  // We'll respond with cancel later if the packet number is wrong
                    let cancel_packet =
                        packet_num != pnum || (255 - pnum) != pnum_1c;
                    let mut data: Vec<u8> = Vec::new();
                    data.resize(packet_size, 0);
                    dev.read_exact(&mut data)?;
                    let success = match self.checksum_mode {
                        ChecksumKind::Standard => {
                            let recv_checksum = get_byte(dev)?;
                            calc_checksum(&data) == recv_checksum
                        }
                        ChecksumKind::Crc16 => {
                            let recv_checksum = (u16::from(get_byte(dev)?)
                                << 8)
                                + u16::from(get_byte(dev)?);
                            calc_crc(&data) == recv_checksum
                        }
                    };

                    if cancel_packet {
                        dev.write_all(&[Consts::CAN.into()])?;
                        dev.write_all(&[Consts::CAN.into()])?;
                        return Err(ModemError::Canceled);
                    }
                    if success {
                        packet_num = packet_num.wrapping_add(1);
                        dev.write_all(&[Consts::ACK.into()])?;
                        out.write_all(&data)?;
                    } else {
                        dev.write_all(&[Consts::NAK.into()])?;
                        self.errors += 1;
                    }
                }
                #[allow(non_snake_case)]
                Some(_EOT) => {
                    // End of file
                    dev.write_all(&[Consts::ACK.into()])?;
                    break;
                }
                None => {
                    self.errors += 1;
                }
            }
            if self.errors >= self.max_errors {
                dev.write_all(&[Consts::CAN.into()])?;
                return Err(ModemError::ExhaustedRetries {
                    errors: Box::from(self.errors),
                });
            }
        }
        Ok(())
    }

    fn init_send<D>(&mut self, dev: &mut D) -> ModemResult<()>
    where
        D: Read + Write,
    {
        let mut cancels = 0u32;
        loop {
            if let Some(c) = get_byte_timeout(dev)?.map(Consts::from) {
                match c {
                    Consts::NAK => {
                        self.checksum_mode = ChecksumKind::Standard;
                        return Ok(());
                    }
                    Consts::CRC => {
                        self.checksum_mode = ChecksumKind::Crc16;
                        return Ok(());
                    }
                    Consts::CAN => {
                        cancels += 1;
                    }
                    _c => (),
                }
            }

            self.errors += 1;

            if cancels >= 2 {
                return Err(ModemError::Canceled);
            }

            if self.errors >= self.max_errors {
                // FIXME: Removed a unused 'if let' here. To be re-added?
                return Err(ModemError::ExhaustedRetries {
                    errors: Box::from(self.errors),
                });
            }
        }
    }

    fn finish_send<D>(&mut self, dev: &mut D) -> ModemResult<()>
    where
        D: Read + Write,
    {
        loop {
            dev.write_all(&[Consts::EOT.into()])?;

            if let Some(c) = get_byte_timeout(dev)? {
                // Appease Clippy with this conditional black.
                #[allow(clippy::redundant_else)]
                if c == Consts::ACK.into() {
                    return Ok(());
                }
            };

            self.errors += 1;

            if self.errors >= self.max_errors {
                return Err(ModemError::ExhaustedRetries {
                    errors: Box::from(self.errors),
                });
            }
        }
    }

    fn send_stream<D, R>(&mut self, dev: &mut D, inp: &mut R) -> ModemResult<()>
    where
        D: Read + Write,
        R: Read,
    {
        let mut block_num = 0u32;
        loop {
            let mut buff = vec![self.pad_byte; self.block_length as usize + 3];
            let n = inp.read(&mut buff[3..])?;
            if n == 0 {
                return Ok(());
            }

            block_num += 1;
            buff[0] = match self.block_length {
                BlockLengthKind::Standard => Consts::SOH.into(),
                BlockLengthKind::OneK => Consts::STX.into(),
            };
            buff[1] = (&block_num & 0xFF) as u8;
            buff[2] = 0xFF - &buff[1];

            match self.checksum_mode {
                ChecksumKind::Standard => {
                    let checksum = calc_checksum(&buff[3..]);
                    buff.push(checksum);
                }
                ChecksumKind::Crc16 => {
                    let crc = calc_crc(&buff[3..]);
                    buff.push(((crc >> 8) & 0xFF) as u8);
                    buff.push((&crc & 0xFF) as u8);
                }
            }

            dev.write_all(&buff)?;

            if let Some(c) = get_byte_timeout(dev)? {
                if c == Consts::ACK.into() {
                    continue;
                }
                // TODO handle CAN bytes
            }

            self.errors += 1;

            if self.errors >= self.max_errors {
                return Err(ModemError::ExhaustedRetries {
                    errors: Box::from(self.errors),
                });
            }
        }
    }
}
