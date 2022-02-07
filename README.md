# Asyncdwmblocks

[![Build](https://github.com/aleksanderkrauze/asyncdwmblocks/actions/workflows/build.yml/badge.svg)](https://github.com/aleksanderkrauze/asyncdwmblocks/actions/workflows/build.yml)
[![Test](https://github.com/aleksanderkrauze/asyncdwmblocks/actions/workflows/test.yml/badge.svg)](https://github.com/aleksanderkrauze/asyncdwmblocks/actions/workflows/test.yml)
[![Clippy](https://github.com/aleksanderkrauze/asyncdwmblocks/actions/workflows/clippy.yml/badge.svg)](https://github.com/aleksanderkrauze/asyncdwmblocks/actions/workflows/clippy.yml)

An asynchronous version of popular dwm statusbar `dwmblocks`.

### Table of contents

 1. [About](#about)
 2. [Usage](#usage)
 3. [Installation](#installation)
 4. [Configuration](#configuration)
 5. [Features](#features)
 6. [Documentation](#documentation)
	-	6.1 [Config file](#config-file)
	- 6.2 [Source code](#source-code)
 7. [Semantic versioning](#semantic-versioning)
 8. [Contributing](#contributing)

## About

This project was heavily inspired by [torrinfail/dwmblocks](https://github.com/torrinfail/dwmblocks)
and [ashish-yadav11/dwmblocks](https://github.com/ashish-yadav11/dwmblocks). However I made different
decisions about design principles. They are as follows:

 0. use Rust as a programming language
 1. use asynchronous programming paradigm
 2. provide many features through conditional compilation rather than patches
 3. use more powerful inter process communication mechanisms than signals
 4. by default use user-provided configuration files

Decision to use Rust instead of classic C was twofold. Firstly it is language I am actively learning,
so this is a learning project where I try to build something larger and more useful than another
`Hello world!` example. Secondly though [suckless](http://suckless.org/)'es philosophy has some good points,
some of them are (in my humble opinion) wrong. One of them is documentation. By using Rust, generating
good and up-to-date documentation is very easy. Rust is also much more "structural" language than C,
meaning that projects can be much more hierarchical and thus easier to understand. By combining this
two thing I hope that it will be easier for someone who wants to contribute to start changing this code.

Another design principle was to create this program in an asynchronous way. It boils down to this.
Instead of in main loop sleeping for 1 second and then for each block checking if it should be updated
(and updating it), each block has it's "timer". When this timer "ticks" block is updated. This *should*
result in less syscalls and less resource usage.

Using patches to enable additional features is *fine* in case of something like `dwm`, but they are pain
to use (anybody who has more than a couple patches in dwm should know this). In this project, which is
designed to do one thing (and do it well) additional features are enabled by conditional compilation
(to see how to use it see [features](#features) section). This makes tradeoff between ease to use and
ease to maintain change in favour of user.

Each time I had to write something like `pgrep dwmblocks >/dev/null && pkill -RTMIN+12 dwmblocks`
I wished there was some easier way to do this. Which block is referred by number 12? It would be much
easier if could say "refresh battery block" instead. Well, now with this project you can do it!
Refreshing block is as easy as running `asyncdwmblocks-notify <BLOCK_NAME>`. This is accomplished
by using more powerful IPC (inter process communication) methods. Currently available choices are
*Transmission Control Protocol* and *Unix Domain Sockets*.

While recompiling whole program each time you change your blocks might result in a (very, very slightly)
faster binary it very fast becomes extremely tedious. Because of that by default `asyncdwmblocks` uses
config file (written in YAML format) to load it's configuration and blocks. If on the other hand you like
recompiling your projects you can manually change source and compile it (with feature flag `config-file` disabled)
to have more similar experience as when using classic dwmblocks. To learn how to write configuration files
see [configuration](#configuration) section.

## Usage

This project compiles to two binaries: `asyncdwmblocks` and `asyncdwmblocks-notify`. To see more detailed
information about their usage run them with `--help` flag.

`asyncdwmblocks` is used very similar to how you would use classic `dwmblocks`. Add following line to
`.xprofile` and you will be good to go: `asyncdwmblocks &`. To see how to configure it read
[configuration](#configuration) section.

`asyncdwmblocks-notify`'s basic usage is: `asyncdwmblocks-notify <block>`. This will refresh given `<block>`.
You can also pass flag `--button <button>` to refresh `<block>` with `$BUTTON` (or whichever is set in configuration)
environment variable set to `<button>`. **Note**: this binary will be only produced when at least one
[feature](#features) enabling IPC (*tcp*, *uds*) will be enabled.

## Installation

## Configuration

### Config file

### Source code

## Features

Features are Rust's way of conditionally compiling code. You can think of a feature as
enabling some functionality if it is enabled and not including it otherwise (features are additive!).

This project provides following features:

- **config-file**: enables loading configuration from file
- **tcp**: enables communication through TCP (Transmission Control Protocol)
- **uds**: enables communication through UDS (Unix Domain Sockets)

Features are enabled by passing to `cargo` flag `--features` followed by comma separated list of features.
There is one special feature called **default**, witch contains a list of default features. In this
project **all** features are enabled by default. To opt-out of default features pass to `cargo` flag
`--no-default-features`. So for example to install this application only with features *tcp* and
*config-file* enabled run `cargo install --no-default-features --features=config-file,tcp`.

## Documentation

If you want to learn how this code works, change it's functionality or change it's default configuration
the best way to start this is by reading the documentation. You can easily generate it by running
`cargo doc`. You can pass additional flags: `--open` to open compiled documentation automatically in
your default browser and `--no-deps` to skip generating documentation for dependencies
(will greatly reduce generation time).

## Semantic versioning

This project follows standard semantic versioning in format `<major>.<minor>.<patch>`.
Change of `<major>` version number means breaking change, such as different configuration format.
Change of `<minor>` version number means additive change, such as added support of new IPC method,
new CLI flags or new configuration option. Change of `<patch>` version number means internal change,
such as fixed bugs, refactored codebase or updated dependencies.

## Contributing

All contribution is very welcome!

The simplest way you can help is to install this application, use it and send me a feedback.
If you find a bug, or have an idea for an improvement open an issue and if you want
to contribute code create a PR. All contribution, unless explicitly stated otherwise,
will be distributed under GPLv3 licence.
