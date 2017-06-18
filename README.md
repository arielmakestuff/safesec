# safesec

A simple program that stores information. Accepts commands to set, get, or
delete a key-data pair via an RPC protocol based on MessagePack.

The primary use of safesec is to simply store encrypted data. This program
does not contain any encryption or decryption routines and so cannot read any
stored encrypted information.

While in heavy development, safesec will accept RPC commands to set, get, and
delete key-data pairs. However, it will be a requirement for version 1.0 that
only the get RPC command is accepted without any form of authentication, with
the set and delete commands requiring authentication.

At this time, the exact authentication protocol has not been decided upon yet.

## Disclaimer

Please keep in mind that I am using this project as a way to learn the Rust
programming language. As such, there will definitely be things that are coded
sub-optimally.

## Getting started

As safesec is in heavy development, there are no pre-built binaries. To build,
first install the Rust toolchain. Depending on your operating system, rust may
be installable from a package management system. Alternatively, you may
install rust via [`rustup`].

[`rustup`]: https://www.rustup.rs

Once Rust is installed, simply enter these commands:

```shell
git clone https://github.com/arielmakestuff/safesec.git
cd safesec
cargo run
```

Currently, this should do nothing more than print "Hello world" on the
console.

## Features

safesec simply stores and retrieves data
* Create, update, retrieve, or delete data via RPC commands
* Storage is provided via [`LMDB`]
* Contains no encryption/decryption routines so cannot understand any stored
  encrypted data

[`LMDB`]: http://www.lmdb.tech/doc/

## Licensing

This project is licensed under the MIT license.
