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

### Install Rust toolchain

As safesec is in heavy development, there are no pre-built binaries. To build,
first install the Rust toolchain. While the toolchain may be installable from
a package management system depending on your platform, it currently is best
to install via [rustup][1]. Please visit [www.rustup.rs][1] to download and
run the installer.

[1]: https://www.rustup.rs

### Run safesec

Once Rust is installed, simply enter these commands to confirm that the test
suite succeeds:

```shell
$ git clone https://github.com/arielmakestuff/safesec.git
$ cd safesec
$ cargo test
```

This will run all unit, integration, and doc tests.

To run the safesec server itself, run the following commands from the root of
the project directory:

```shell
$ cargo build --release
$ target/release/safesec
```

## Features

safesec simply stores and retrieves data
* Create, update, retrieve, or delete data via RPC commands
* Storage is provided via [`LMDB`]
* Contains no encryption/decryption routines so cannot understand any stored
  encrypted data

[`LMDB`]: http://www.lmdb.tech/doc/

## Licensing

This project is licensed under the MIT license.
