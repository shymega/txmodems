# txmodems

## About

`txmodems` is a Rust `no_std` crate for the - eventual - support of X/Y/Z MODEM
data transfer protocols.

This crate supports `no_std` first and foremost, but also supports `std`. No
additional flags are required to use on `std`, it should work as-is.`

Currently, by using the code from this
[crate](https://github.com/awelkie/xmodem.rs), we have an initial codebase for
XMODEM.

The plan is to use Cargo features for different -MODEM protocol supports. Soon,
once ready, YMODEM support will be available via code derived from
[here](https://github.com/TGMM/xymodem.rs).

All attributions to these code usages is [here][mit]. I have licensed
`txmodems` under the same license as the code used from the aforementioned
crates.

## Usage

You can get this crate from Git right now, but once all MODEMs are working and
stable, I'll be releasing `v1.0.0` on crates.io.

To use each different type of -MODEM (currently it's only XMODEM), you need to
explicitly enable each corresponding feature. In the case of XMODEM, enable the
`xmodem` feature.

## License

Licensed under the [MIT license][mit].

[mit]: /LICENSE
