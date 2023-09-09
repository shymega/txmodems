# txmodems

## About

`txmodems` is a Rust `no_std` crate for the - eventual - (X/Y/Z)MODEM
data transfer protocols.

This crate supports `no_std` first and foremost, but also supports `std`. No
additional flags are required to use on `std`, it should work as-is.

Currently, by using the code from this
[crate](https://github.com/awelkie/xmodem.rs), we have an initial codebase for
XMODEM.

The plan is to use Cargo features for different -MODEM protocol supports. Soon,
once ready, YMODEM support will be available via code derived from
[here](https://github.com/TGMM/xymodem.rs).

All attributions to these code usages is [here][mit]. I have licensed
`txmodems` under the same license as the code used from the aforementioned
crates. I also aim to use the 'REUSE' tool made by FSFE, to correctly provide attribution to the original authors.

Some things do differ, however. For example, I have aimed for `no_std` support
from the start, and used traits to implement functionality for each -MODEM
type.

## Usage

I've published this crate to [crates.io](https://crates.io). Currently, only
XMODEM compiles correctly. You can enable it using Cargo's 'features'. By
default, it is not enabled.

To use each different type of -MODEM (currently it's only XMODEM), you need to
explicitly enable each corresponding feature.

## License

Licensed under the [MIT license][mit].

[mit]: /LICENSE
