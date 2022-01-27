# Asyncdwmblocks

[![Build](https://github.com/aleksanderkrauze/asyncdwmblocks/actions/workflows/build.yml/badge.svg)](https://github.com/aleksanderkrauze/asyncdwmblocks/actions/workflows/build.yml)
[![Test](https://github.com/aleksanderkrauze/asyncdwmblocks/actions/workflows/test.yml/badge.svg)](https://github.com/aleksanderkrauze/asyncdwmblocks/actions/workflows/test.yml)
[![Clippy](https://github.com/aleksanderkrauze/asyncdwmblocks/actions/workflows/clippy.yml/badge.svg)](https://github.com/aleksanderkrauze/asyncdwmblocks/actions/workflows/clippy.yml)

An asynchronous version of popular dwm statusbar `dwmblocks`.

### Table of contents

 1. [About](#about)
 2. [Installation](#installation)
 3. [Configuration](#configuration)
 4. [Features](#features)
 5. [Semantic versioning](#semantic-versioning)
 6. [Contributing](#contributing)

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

## Installation

## Configuration

## Features

## Semantic versioning

## Contributing
