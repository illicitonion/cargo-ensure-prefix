cargo-ensure-prefix
===================

A `cargo` subcommand to check that all target files have a fixed prefix. This may be useful for licence headers, ensuring particular clippy lints are en/disabled, or maybe some other things too.

[![crates.io](https://img.shields.io/crates/v/cargo-ensure-prefix.svg)](https://crates.io/crates/cargo-ensure-prefix)
[![Documentation](https://docs.rs/cargo-ensure-prefix/badge.svg)](https://docs.rs/cargo-ensure-prefix)
[![Build Status](https://travis-ci.org/illicitonion/cargo-ensure-prefix.svg?branch=master)](https://travis-ci.org/illicitonion/cargo-ensure-prefix)

Usage
-----

```
cargo-ensure-prefix 

USAGE:
    cargo-ensure-prefix [FLAGS] [OPTIONS] --manifest-path <manifest-path> --prefix-path <prefix-path>

FLAGS:
        --all        
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --manifest-path <manifest-path>    
    -p, --package <package>...             
        --prefix-path <prefix-path>
```

Wildcard matching
-----------------

Any byte will be accepted where a character `\x1A` is present in the prefix file. e.g. you could match any set of 4-digit years with a prefix file with contents: `Copyright \x1A\x1A\x1A\x1A`. Note that this is byte-wise matching, not character-wise matching.
