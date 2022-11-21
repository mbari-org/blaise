# blaise

A Rust implementation of [voc-cropper](https://github.com/mbari-org/voc-cropper).

😲 what?! This is just an learning exercise on how things like xml parsing and basic image processing
can be done in Rust. Along with the use of Rust mechanisms for CLI, file handling, multithreading,
progress report, etc., it only intends to reproduce the main functionality in voc-imagecropper,
not necessarily all its options or features (at least initially).

Notable differences wrt voc-imagecropper include:
- cropped images retain the same format as the input images (that is, not forced to jpeg)
- no checks for minimum size, or option for resizing
- no summary of average of the images
- option `-j` allows to control number of threads to use

## Installation

A GitHub workflow builds and [releases](../../releases/) Linux and macOS binaries
of the program.

Alternatively, with Rust on your system, you can clone this repo and run:

```shell
cargo build --release
cargo install --path .
``` 
This should put the executable in your `~/.cargo/bin` directory.

Then, run `blaise --help` to see the usage.

Simple example:

```shell
blaise -d data -o data/out
```

## Development

We use [just](https://github.com/casey/just) for [various tasks](justfile).
Run `just list` to see the available recipes.

```shell
$ just list 
Available recipes:
    default                        # A convenient default for development: test and format
    all                            # default + clippy; good to run before committing changes
    list                           # List recipes
    check                          # cargo check
    test                           # Run tests
    test-nocapture                 # Run tests with --nocapture
    run args='-d data -o data/out' # Run program with basic example
    format                         # Format source code
    clippy                         # Run clippy
    build args='--release'         # Build
    outdated                       # Show outdated dependencies
    udeps                          # Find unused dependencies
    update                         # cargo update
```

In particular, be sure to run `just all`
before committing/pushing any changes.

## Misc links/refs

- <https://docs.rs/serde-xml-rs/latest/serde_xml_rs/>
- <https://github.com/image-rs/image>
- <https://deterministic.space/>
- <https://www.reddit.com/r/rust/comments/7mu7q1/comment/drwoat0>
- <https://github.com/ritiek/auto-image-cropper>
- <https://github.com/console-rs/indicatif>
