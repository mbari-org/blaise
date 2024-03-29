# blaise

A Rust implementation of [voc-cropper](https://github.com/mbari-org/voc-cropper).

This repo started as a learning exercise on how things like xml parsing and basic image processing
can be done in Rust. Along with the use of Rust mechanisms for CLI, file handling, multithreading,
progress report, etc., it only intended to reproduce the main functionality in voc-imagecropper,
but more features were subsequently added.

Notable differences wrt voc-imagecropper include:
- cropped images are written out in png format (not in jpeg)
- no checks for minimum size
- no summary of average of the images
- for location of the images, along with the `--image-dir` option, only the `filename` attribute
  is used from the xml 
- blaise can also ingest annotations in Yolo format (option `--yolo`)
  (translation logic adopted from [yolo_to_voc.py](
   https://bitbucket.org/mbari/m3-download/src/main/scripts/yolo_to_voc.py))
- some additional options:
  - `--bb-info <csv-file>`
  - `--max-ar <value>`
  - `-j` to indicate number of threads to use

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
blaise --pascal data/annotations  --image-dir data/imgs --output-dir data/out
```

## Usage
```shell
blaise --help
```
```text
Creates image crops for given annotations

Usage: blaise [OPTIONS] --output-dir <dir>

Options:
  -p, --pascal <dir>
          Base directory to scan for pascal voc annotations
  -y, --yolo <image-dir> <label-dir> <names-file>
          Use yolo annotations
  -i, --image-dir <dir>
          Image base directory
      --max-ar <AR>
          Only process images having at most the given aspect ratio
  -r, --resize <width> <height>
          Resize the resulting crops (aspect ratio not necessarily preserved)
  -L, --select-labels <labels>
          Comma separated list of labels to crop. Defaults to everything
  -o, --output-dir <dir>
          Path to store image crops
  -b, --bb-info <csv-file>
          Generate csv with size, aspect ratio of loaded bounding boxes
      --verbose
          Verbose output (disables progress bars)
      --npb
          Do not show progress bars
  -j <N>
          Number of threads to use (by default, all available)
  -h, --help
          Print help
  -V, --version
          Print version
```


## Development

We use [just](https://github.com/casey/just) for [various tasks](justfile).
Run `just list` to see the available recipes.

```shell
$ just list 
Available recipes:
    default                 # A convenient default for development: test and format
    all                     # default + clippy; good to run before committing changes
    list                    # List recipes
    check                   # cargo check
    test                    # Run tests
    test-nocapture          # Run tests with --nocapture
    run *args='-p data -o data/out' # Run program with basic example
    rrun *args='-p data -o data/out' # Run program in release mode
    tgz                     # Package source code
    format                  # Format source code
    clippy                  # Run clippy
    build *args='--release' # Build
    install                 # Install
    outdated                # Show outdated dependencies
    udeps                   # Find unused dependencies
    update                  # cargo update
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
