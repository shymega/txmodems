//! This crate implements various MODEM file transfer protocols.
//! Current priority is YMODEM, which is to be used with the Planet Computers Cosmo Communicator.
#![deny(
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

mod consts;
