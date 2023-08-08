extern crate alloc;

use log::{debug, error, info, warn};

use alloc::{boxed::Box, vec, vec::Vec};
use core::convert::From;

use core2::io::{Read, Write};
use crate::common::{calc_checksum, calc_crc, get_byte, get_byte_timeout, Modem, ModemError, ModemResult};

use crate::variants::xmodem::{common::{ChecksumKind, BlockLengthKind}, Consts};

// TODO: Send CAN byte after too many errors
// TODO: Handle CAN bytes while sending
// TODO: Implement Error for Error


/// Configuration for the XMODEM transfer.
#[derive(Copy, Clone, Debug)]
pub struct Xmodem {
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

impl Default for Xmodem {
    fn default() -> Self {
        Xmodem::new()
    }
}

impl Modem for Xmodem {
    fn new() -> Self where Self: Sized {
        Self {
            max_errors: 16,
            pad_byte: 0x1a,
            block_length: BlockLengthKind::Standard,
            checksum_mode: ChecksumKind::Standard,
            errors: 0,
        }
    }

    fn send<D, R>(&mut self, dev: &mut D, inp: &mut R) -> ModemResult<()> where D: Read + Write, R: Read {
        self.errors = 0;

        debug!("Starting XMODEM transfer");
        self.init_send(dev)?;

        debug!("First byte received. Sending stream.");
        self.send_stream(dev, inp)?;

        debug!("Sending EOT");
        self.finish_send(dev)?;

        Ok(())
    }

    fn receive<D, W>(&mut self, dev: &mut D, out: &mut W, checksum: ChecksumKind) -> ModemResult<()> where D: Read + Write, W: Write {
        self.errors = 0;
        self.checksum_mode = checksum;

        debug!("Starting XMODEM receiver");

        dev.write_all(&[match self.checksum_mode {
            ChecksumKind::Standard => Consts::NAK.into(),
            ChecksumKind::Crc16 => Consts::CRC.into(),
        }])?;
        debug!("NCG sent. Receiving stream.");
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
                        return Err(ModemError::Canceled)
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
                    warn!("Timeout!");
                }
            }
            if self.errors >= self.max_errors {
                error!(
                    "Exhausted max retries ({}) while waiting for ACK for EOT",
                    self.max_errors
                );
                dev.write_all(&[Consts::CAN.into()])?;
                return Err(ModemError::ExhaustedRetries {
                    errors: Box::from(self.errors),
                });
            }
        }
        Ok(())
    }

    fn init_send<D>(&mut self, dev: &mut D) -> ModemResult<()> where D: Read + Write {
        let mut cancels = 0u32;
        loop {
            match get_byte_timeout(dev)?.map(Consts::from) {
                Some(c) => match c {
                    Consts::NAK => {
                        debug!("Standard checksum requested");
                        self.checksum_mode = ChecksumKind::Standard;
                        return Ok(());
                    }
                    Consts::CRC => {
                        debug!("16-bit CRC requested");
                        self.checksum_mode = ChecksumKind::Crc16;
                        return Ok(());
                    }
                    Consts::CAN => {
                        warn!("Cancel (CAN) byte received");
                        cancels += 1;
                    }
                    c => warn!(
                        "Unknown byte received at start of XMODEM transfer: {:?}",
                        c
                    ),
                },
                None => {
                    warn!("Timed out waiting for start of XMODEM transfer.");
                }
            }

            self.errors += 1;

            if cancels >= 2 {
                error!(
                    "Transmission canceled: received two cancel (CAN) bytes \
                        at start of XMODEM transfer"
                );
                return Err(ModemError::Canceled);
            }

            if self.errors >= self.max_errors {
                error!(
                    "Exhausted max retries ({}) at start of XMODEM transfer.",
                    self.max_errors
                );
                if let Err(err) = dev.write_all(&[Consts::CAN.into()]) {
                    warn!("Error sending CAN byte: {}", err);
                }
                return Err(ModemError::ExhaustedRetries {
                    errors: Box::from(self.errors)
                });
            }
        }
    }

    fn finish_send<D>(&mut self, dev: &mut D) -> ModemResult<()> where D: Read + Write {
        loop {
            dev.write_all(&[Consts::EOT.into()])?;

            if let Some(c) = get_byte_timeout(dev)? {
                // Appease Clippy with this conditional black.
                #[allow(clippy::redundant_else)]
                if c == Consts::ACK.into() {
                    info!("XMODEM transmission successful");
                    return Ok(());
                }
                warn!("Expected ACK, got {}", c);
            };

            self.errors += 1;

            if self.errors >= self.max_errors {
                error!(
                    "Exhausted max retries ({}) while waiting for ACK for EOT",
                    self.max_errors
                );
                return Err(ModemError::ExhaustedRetries {
                    errors: Box::from(self.errors)
                });
            }
        }
    }

    fn send_stream<D, R>(&mut self, dev: &mut D, inp: &mut R) -> ModemResult<()> where D: Read + Write, R: Read {
        let mut block_num = 0u32;
        loop {
            let mut buff = vec![self.pad_byte; self.block_length as usize + 3];
            let n = inp.read(&mut buff[3..])?;
            if n == 0 {
                debug!("Reached EOF");
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

            debug!("Sending block {}", block_num);
            dev.write_all(&buff)?;

            match get_byte_timeout(dev)? {
                Some(c) => {
                    // Appease Clippy with this conditional block.

                    if c == Consts::ACK.into() {
                        debug!("Received ACK for block {}", block_num);
                        continue;
                    }

                    warn!("Expected ACK, got {}", c);
                    // TODO handle CAN bytes
                }
                None => {
                    warn!("Timeout waiting for ACK for block {}", block_num);
                }
            }

            self.errors += 1;

            if self.errors >= self.max_errors {
                error!("Exhausted max retries ({}) while sending block {} in XMODEM transfer",
                       self.max_errors, block_num);
                return Err(ModemError::ExhaustedRetries {
                    errors: Box::from(self.errors)
                });
            }
        }
    }
}