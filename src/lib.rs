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

// Run checks for -MODEM features.
#[cfg(not(any(feature = "xmodem", feature = "zmodem", feature = "ymodem")))]
compile_error!("No `-MODEM` specified, please specify at least one!");

mod common;
pub mod variants;
