use core::str::from_utf8;
use core2::io::*;
use core::alloc;

use crate::common::*;
#[cfg(defmt)]
use defmt::*;
use heapless::{String, Vec};

const SOH: u8 = 0x01;
const STX: u8 = 0x02;
const EOT: u8 = 0x04;
const ACK: u8 = 0x06;
const NAK: u8 = 0x15;
const CAN: u8 = 0x18;
const CRC: u8 = 0x43;

/// `YModem` acts as state for XMODEM transfers
#[derive(Default, Debug, Copy, Clone)]
#[allow(dead_code)] // TODO: Temporarily allow this lint, whilst I work out YMODEM support.
pub struct YModem {
    /// The number of errors that can occur before the communication is
    /// considered a failure. Errors include unexpected bytes and timeouts waiting for bytes.
    pub max_errors: u32,

    /// The number of *initial errors* that can occur before the communication is
    /// considered a failure. Errors include unexpected bytes and timeouts waiting for bytes.
    pub max_initial_errors: u32,

    /// The byte used to pad the last block. XMODEM can only send blocks of a certain size,
    /// so if the message is not a multiple of that size the last block needs to be padded.
    pub pad_byte: u8,

    /// Boolean value to ignore non digits on file size.
    pub ignore_non_digits_on_file_size: bool,

    errors: u32,
    initial_errors: u32,
}

impl YModem {
    fn add_error(&mut self) -> ModemResult<()> {
        self.errors += 1;

        if self.errors >= self.max_errors {
            #[cfg(defmt)]
            error!("Exhausted max retries ({}) while sending start frame in YMODEM transfer", self.max_errors);
            return Err(ModemError::ExhaustedRetries { errors: self.max_errors });
        } else {
            Ok(())
        }
    }
}

impl ModemTrait for YModem {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            max_errors: 16,
            max_initial_errors: 16,
            pad_byte: 0x1a,
            errors: 0,
            initial_errors: 0,
            ignore_non_digits_on_file_size: false,
        }
    }
}

impl YModemTrait for YModem {
    /// Receive a YMODEM transmission.
    ///
    /// `dev` should be the serial communication channel (e.g. the serial device).
    /// The received data will be written to `out`.
    /// `checksum` indicates which checksum mode should be used; ChecksumKind::Crc16 is
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
    ) -> ModemResult<()> {
        let mut file_buf: Vec<u8, 1024> = Vec::new();

        self.errors = 0;
        #[cfg(defmt)]
        debug!("Starting YMODEM receive");

        loop {
            dev.write(&[CRC])?;

            match get_byte_timeout(dev) {
                Ok(v) => {
                    // the first SOH is used to initialize the transfer
                    if v == Some(SOH) {
                        break;
                    }
                },
                Err(_err) => {
                    self.initial_errors += 1;
                    if self.initial_errors > self.max_initial_errors {
                        #[cfg(defmt)]
                        error!("Exhausted max retries ({}) while waiting for SOH or STX", self.max_initial_errors);
                        return Err(ModemError::ExhaustedRetries { errors: self.errors }); // TODO: Remove Box
                    }
                },
            }
        }
        // First packet
        // In YModem the header packet is 0
        let mut packet_num: u8 = 0;
        let mut file_name_buf:  Vec<u8, 32> = Vec::new();
        let mut file_size_buf:  Vec<u8, 32> = Vec::new();
        let mut padding_buf:    Vec<u8, 32> = Vec::new();

        loop {
            let pnum    = (get_byte(dev))?; // specified packet number
            let pnum_1c = (get_byte(dev))?; // specified packet number 1's complemented

            let cancel_packet = packet_num != pnum || (255 - pnum) != pnum_1c;

            loop {
                let b = get_byte(dev)?;
                file_name_buf.push(b).unwrap();
                if b == 0x00 { break; };
            }
            *file_name = String::<32>::from_utf8(file_name_buf.clone()).unwrap();

            loop {
                let b = get_byte(dev)?;
                file_size_buf.push(b).unwrap();
                if b == 0x00 {
                    break;
                };
            }

            // We read the padding
            // The 2 is the 2 zeroes
            for _ in 0..(128 - file_name_buf.len() - file_size_buf.len()) {
                padding_buf.push(get_byte(dev)?).unwrap();
            }

            let recv_checksum = (((get_byte(dev))? as u16) << 8) + (get_byte(dev))? as u16;

            let mut data_buf: Vec<u8, 1024> = Vec::new();
            data_buf.extend(file_name_buf.clone());
            data_buf.extend(file_size_buf.clone());
            data_buf.extend(padding_buf.clone());

            let success = calc_crc(&mut data_buf) == recv_checksum;

            if cancel_packet {
                (dev.write(&[CAN]))?;
                (dev.write(&[CAN]))?;
                return Err(ModemError::Canceled);
            }
            if !success {
                (dev.write(&[NAK]))?;
                self.errors += 1;
            } else {
                // First packet recieved succesfully
                packet_num = packet_num.wrapping_add(1);
                (dev.write(&[ACK]))?;
                (dev.write(&[CRC]))?;
                break;
            }

        }

        let mut file_size_str = String::from_utf8(file_size_buf).unwrap();
        if self.ignore_non_digits_on_file_size {
            file_size_str = file_size_str.chars().filter(|c| c.is_digit(10)).collect();
        }

        let file_size_num: u32 = match file_size_str.parse::<u32>() {
            Ok(v) => v,
            Err(_) => file_size_str.split(" ").next().unwrap().parse::<u32>().unwrap(),
        };
        *file_size = file_size_num;

        let num_of_packets = file_size_num + 1023 / 1024;
        let final_packet = num_of_packets + 2;
        let mut received_first_eot = false;

        for range in 0..=final_packet {
            #[cfg(defmt)]
            debug!("{}", range);
            match get_byte_timeout(dev)? {
                bt @ Some(SOH) | bt @ Some(STX) => {
                    // handle next packet
                    let packet_size = match bt {
                        Some(SOH) => 128,
                        Some(STX) => 1024,
                        _ => 0,
                    };
                    let pnum = get_byte(dev)?;      // specifed packet number
                    let pnum_1c = get_byte(dev)?;   // specifed packet number 1's complement

                    let cancel_packet = match range {
                        // Final packet num is 0
                        cp if cp == final_packet => 0x00 != pnum || (0xFF - pnum) != pnum_1c,
                        _ => packet_num != pnum || (0xFF - pnum) != pnum_1c,
                    };
                    let mut data: Vec<u8, 1024> = Vec::new();
                    data.resize(packet_size, 0).unwrap();
                    dev.read_exact(&mut data)?;
                    let recv_checksum = (((get_byte(dev))? as u16) << 8) + (get_byte(dev))? as u16;
                    let success = calc_crc(&data) == recv_checksum;

                    if cancel_packet {
                        dev.write(&[CAN])?;
                        dev.write(&[CAN])?;
                        return Err(ModemError::Canceled);
                    }
                    if success {
                        packet_num = packet_num.wrapping_add(1);
                        dev.write(&[ACK])?;
                        let array = &data.into_array::<1024>().unwrap();
                        let s = from_utf8(array.as_slice()).unwrap();
                        core::fmt::Write::write_str(&mut file_buf, s).unwrap();
                    } else {
                        dev.write(&[NAK])?;
                        self.add_error()?;
                    }
                },
                Some(EOT) => {
                    packet_num = packet_num.wrapping_add(1);
                    // End of file
                    if !received_first_eot {
                        dev.write(&[NAK])?;
                        received_first_eot = true;
                    } else {
                        dev.write(&[ACK])?;
                        dev.write(&[CRC])?;
                    }
                }
                Some(_) => {
                    #[cfg(defmt)]
                    warn!("Unrecognized symbol!")
                },
                None    => {
                    self.add_error()?;
                    #[cfg(defmt)]
                    error!("Timeout!")
                },
            }
        }

        out.write_all(&file_buf[0..file_size_num as usize]).unwrap();

        Ok(())
    }

    /// Starts the YMODEM transmission.
    ///
    /// `dev` should be the serial communication channel (e.g. the serial device).
    /// `stream` should be the message to send (e.g. a file).
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
    ) -> ModemResult<()> {
        self.errors = 0;
        let packets_to_send = (file_size + 1023 / 1024) as u32;
        let last_packet_size = file_size % 1024;

        #[cfg(defmt)]
        debug!("Starting YMODEM transfer");
        self.start_send(dev)?;

        #[cfg(defmt)]
        debug!("First byte recieved. Sendiong start frame.");
        self.send_start_frame(dev, file_name, file_size)?;

        #[cfg(defmt)]
        debug!("Start frame acknowleded. Sending stream");
        self.send_stream(dev, inp, packets_to_send, last_packet_size)?;

        #[cfg(defmt)]
        debug!("Sending EOT");
        self.finish_send(dev)?;

        Ok(())
    }

    fn start_send<D: Read + Write>(&mut self, dev: &mut D) -> ModemResult<()> {
        let mut cancels = 0u32;
        loop {
            match get_byte_timeout(dev)? {
                Some(c) => match c {
                    CRC => {
                        #[cfg(defmt)]
                        debug!("16-bit CRC requested");
                        return Ok(());
                    },
                    CAN => {
                        #[cfg(defmt)]
                        warn!("Cancel (CAN) byte recived");
                        cancels += 1;
                    }
                    c   => {
                        #[cfg(defmt)]
                        warn!("Unknown byte recived at start of YMODEM tranfer: {}", c)
                    },
                },
                None    => {
                    #[cfg(defmt)]
                    warn!("Timed out waiting for start of YMODEM transfer")
                },
            }
            self.errors += 1;

            if cancels >= 2 {
                #[cfg(defmt)]
                error!("Transmission canceled: recived two cancel (CAN) bytes at start of YMODEM transfer");
                return Err(ModemError::Canceled);
            }
            if self.errors >= self.max_errors {
                #[cfg(defmt)]
                error!("Exhausted max retries ({}) at start of YMODEM transfer.", self.max_errors);
                if let Err(err) = dev.write_all(&[CAN]) {
                    #[cfg(defmt)]
                    warn!("Error sending CAN byte: {}", err);
                }
                return Err(ModemError::ExhaustedRetries { errors: self.errors });
            }
        }
    }

    fn send_start_frame<D: Read + Write>(
        &mut self,
        dev: &mut D,
        file_name: String<32>,
        file_size: u64,
    ) -> ModemResult<()> {
        let mut buf = [0; 128 + 5];
        buf[0] = SOH;
        buf[1] = 0x00;
        buf[2] = 0xFF;

        let mut i = 3;
        for byte in file_name.as_bytes() {
            buf[i] = *byte;
            i += 1;
        }

        // zero terminate the string
        i += 1;

        for byte in alloc::format!("{:x}", file_size).as_bytes() {
            buf[i] = *byte;
            i += 1;
        }

        let crc = calc_crc(&buf[3..128 + 3]);
        buf[buf.len() - 2] = ((crc >> 8) & 0xFF) as u8;
        buf[buf.len() - 1] = (crc & 0xFF) as u8;

        dev.write_all(&buf)?;

        loop {
            match get_byte_timeout(dev)? {
                Some(ACK)   => {#[cfg(defmt)] debug!("Recived ACK for start frame"); break;},
                Some(CAN)   => {#[cfg(defmt)] warn!("TODO: handle cancel")},
                Some(c)     => {#[cfg(defmt)] warn!("Expected ACK, got {}", c)},
                None        => {#[cfg(defmt)] warn!("Timeout waiting for ACK for start frame")},
            }
            self.add_error()?;
        }
        loop {
            match get_byte_timeout(dev)? {
                Some(CRC)   => {#[cfg(defmt)] debug!("Recieved C for start frame"); break;},
                Some(CAN)   => {#[cfg(defmt)] warn!("TODO: handle cancel")},
                Some(c)     => {#[cfg(defmt)] warn!("Expected C, got {}", c)},
                None        => {#[cfg(defmt)] warn!("Timeout waiting for CRC start frame")},
            }
            self.add_error()?;
        }
        Ok(())
    }

    fn send_stream<D: Read + Write, R: Read>(
        &mut self,
        dev: &mut D,
        stream: &mut R,
        packets_to_send: u32,
        last_packet_size: u64,
    ) -> ModemResult<()> {
        let mut block_num = 0u32;
        loop {
            let packet_size = if block_num + 1 == packets_to_send && last_packet_size <= 128 {
                128
            } else {
                1024
            };

            let mut buf = [self.pad_byte; 1024 + 5];
            let n = stream.read(&mut buf[3..])?;
            if n == 0 {
                #[cfg(defmt)]
                debug!("Reached EOF");
                return Ok(());
            }

            block_num += 1;
            if packet_size == 128 {
                buf[0] = SOH;
            } else {
                buf[0] = STX;
            }
            buf[1] = (block_num & 0xFF) as u8;
            buf[2] = 0xFF - buf[1];

            let crc = calc_crc(&buf[3..packet_size+3]);
            buf[packet_size+3] = ((crc >> 8) & 0xFF) as u8;
            buf[packet_size+4] = (crc & 0xFF) as u8;

            #[cfg(defmt)]
            info!("Sending block {}", block_num);
            dev.write_all(&buf[0..packet_size+5])?;

            match get_byte_timeout(dev)? {
                Some(ACK)   => {
                    #[cfg(defmt)]
                    debug!("Recived ACK for block {}", block_num);
                    continue;
                },
                Some(CAN)   => {#[cfg(defmt)] warn!("TODO: handle CAN cancel")},
                Some(c)     => {#[cfg(defmt)] warn!("Expected ACK, got {}", c)},
                None        => {#[cfg(defmt)] warn!("Timeout waiting for ACK for block {}", block_num)},
            }
            self.add_error()?;

        }

    }

    fn finish_send<D: Read + Write>(&mut self, dev: &mut D) -> ModemResult<()> {
        loop {
            dev.write_all(&[EOT])?;
            match get_byte_timeout(dev)? {
                Some(NAK)   => break,
                Some(c)     => {#[cfg(defmt)] warn!("Expected NAK, got {}", c)},
                None        => {#[cfg(defmt)] warn!("Timeout waiting for NAK for EOT")},
            }
            self.add_error()?;
        }

        loop {
            dev.write_all(&[EOT])?;
            match get_byte_timeout(dev)? {
                Some(ACK)   => break,
                Some(c)     => {#[cfg(defmt)] warn!("Expected ACK, got {}", c)},
                None        => {#[cfg(defmt)] warn!("Timeout waiting for ACK for EOT")},
            }

            self.add_error()?;
        }

        loop {
            match get_byte_timeout(dev)? {
                Some(CRC)   => {#[cfg(defmt)]info!("YMODEM transmission successful"); break;},
                Some(c)     => {#[cfg(defmt)] warn!("Expected C, got {}", c)},
                None        => {#[cfg(defmt)] warn!("Timeout waiting for CRC for EOT")},
            }
            self.add_error()?;
        }
        self.send_end_frame(dev)?;
        Ok(())
    }

    fn send_end_frame<D: Read + Write>(&mut self, dev: &mut D) -> ModemResult<()> {
        let mut buf = [0; 128 + 5];
        buf[0] = SOH;
        buf[1] = 0x00;
        buf[2] = 0xFF;

        let crc = calc_crc(&buf[3..128+3]);
        buf[buf.len() - 2] = ((crc >> 8) & 0xFF) as u8;
        buf[buf.len() - 1] = (crc & 0xFF) as u8;

        dev.write_all(&buf)?;
        loop {
            match get_byte_timeout(dev)? {
                Some(ACK)   => {#[cfg(defmt)] debug!("Recived ACK for end frame"); break;},
                Some(CAN)   => {#[cfg(defmt)] warn!("TODO: handle CAN cancel")},
                Some(c)     => {#[cfg(defmt)] warn!("Expected ACK, got {}", c)},
                None        => {#[cfg(defmt)] warn!("Timeout waiting for ACK for end frame")},
            }
            self.add_error()?;
        }
        Ok(())
    }
}


