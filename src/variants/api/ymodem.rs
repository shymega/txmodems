use crate::common::ModemTrait;

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
