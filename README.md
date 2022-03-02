# txmodems

## About

`txmodems` is a Rust `no_std` crate for the - eventual - support of X/Y/Z 
MODEM 
data transfer protocols.

The main purpose of this crate is for being used on `std`-capable 
environments, as well as embedded environments. Officially though, we will 
only be supporting the `no_std` targetd *at this moment* in time.

Currently, by using the code from this [crate](https://github.
com/awelkie/xmodem.rs), we have an initial codebase for XMODEM. 

The plan is to use Cargo features for different -MODEM protocol supports. 
Soon, once ready, YMODEM support will be available via code derived from 
[here](https://github.com/TGMM/xymodem.rs).

All attributions to these code usages is [here][mit]. I have licensed 
`txmodems` under the same license as the code used from the aforementioned 
crates.

## Usage

You can get this crate from Git right now, but once the `XMODEM` support is 
working and stable, I'll be releasing `v0.1.0` on crates.io.

To use each different type of -MODEM (currently it's only XMODEM), you need 
to explicitly enable each corresponding feature. In the case of XMODEM, enable 
the `xmodem` feature.

## License

Licensed under the [MIT license][mit].

[mit]: /LICENSE
