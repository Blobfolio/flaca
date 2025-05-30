# Flaca

[![ci](https://img.shields.io/github/actions/workflow/status/Blobfolio/flaca/ci.yaml?style=flat-square&label=ci)](https://github.com/Blobfolio/flaca/actions)
[![deps.rs](https://deps.rs/repo/github/blobfolio/flaca/status.svg?style=flat-square&label=deps.rs)](https://deps.rs/repo/github/blobfolio/flaca)<br>
[![license](https://img.shields.io/badge/license-wtfpl-ff1493?style=flat-square)](https://en.wikipedia.org/wiki/WTFPL)
[![contributions welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square&label=contributions)](https://github.com/Blobfolio/flaca/issues)

Flaca is a CLI tool for x86-64 Linux machines that simplifies the task of maximally, **losslessly** compressing GIF, JPEG, and PNG images for use in production **web environments**.

It prioritizes compression over speed or resource modesty, and runs best on systems with multiple CPUs. There are only so many ways to be a GIF or JPEG, but calculating the optimal construction for a PNG can take a lot of work!

Compression is mainly achieved through the removal of metadata and optimization of pixel tables. Under the hood, flaca leverages [Gifsicle](https://github.com/kohler/gifsicle) for GIFs, the `jpegtran` functionality from [MozJPEG](https://github.com/mozilla/mozjpeg) for JPEG images, and a combination of [Oxipng](https://github.com/shssoichiro/oxipng) and [Zopflipng](https://github.com/google/zopfli) for PNG images.



## Metadata

For web images, metadata is just so much wasted bandwidth. Stock photos in particular can be bloated 50% or more with embedded keywords and descriptions that browsers make zero use of. Removing that data — particularly at scale — leads to both lower hosting costs for site operators and faster page loads for visitors.

And it helps close the [digital divide](https://en.wikipedia.org/wiki/Digital_divide).

But in other contexts, _metadata may matter_.

As a general rule, you should _not_ try to feed your entire personal media library or raw print/design assets to flaca or it may eat something important.



## Installation

Debian and Ubuntu users can just grab the pre-built `.deb` package from the [latest release](https://github.com/Blobfolio/flaca/releases/latest).

This application is written in [Rust](https://www.rust-lang.org/) and can alternatively be built/installed from source using [Cargo](https://github.com/rust-lang/cargo):

```bash
# See "cargo install --help" for more options.
cargo install \
    --git https://github.com/Blobfolio/flaca.git \
    --bin flaca
```

Note that when building from source, you'll need to have `make`, `nasm`, and the development headers for `libjpeg` and `libpng` installed beforehand or Cargo will pop an error. (If that happens, just install the missing thing and try again.)



## Usage

It's easy. Just run `flaca [FLAGS] [OPTIONS] <PATH(S)>…`.

The following flags and options are available:

| Short | Long | Value | Description |
| ----- | ---- | ----- | ----------- |
| `-h` | `--help` | | Print help information and exit. |
| `-j` | | `<NUM>` | Limit[^1] parallelization to this many threads (instead of using all logical cores). |
| `-l` | `--list` | `<FILE>` | Read (absolute) image and/or directory paths from this text file — or STDIN if "-" — one entry per line, instead of or in addition to the trailing `<PATH(S)>`. |
| | `--max-resolution` | `<NUM>` | Skip images containing more than `<NUM>` total pixels. |
| | `--no-gif` | | Skip GIF images. |
| | `--no-jpeg` | | Skip JPEG images. |
| | `--no-png` | | Skip PNG Images. |
| | `--no-symlinks` | | Ignore symlinks (rather than following them). |
| | `--preserve-times` | | Preserve file access/modification times. |
| `-p` | `--progress` | | Show pretty progress while minifying. |
| `-z` | `--zopfli-iterations` | `<NUM>` | Override the number of zopfli iterations when compressing PNGs. |
| `-V` | `--version` | | Print version information and exit. |

You can feed it any number of file or directory paths in one go, and/or toss it a text file using the `-l` option. Directories are searched recursively.

To reduce pointless I/O, flaca will silently ignore file paths lacking an appropriate extension, i.e. `.gif`, `.jp(e)g`, or `.png` (case-insensitive).

Flaca can cross filesystem and user boundaries, provided the user running the program has the relevant read/write access. (Not that you should run it as `root`, but if you did, images should still be owned by `www-data` or whatever after re-compression.)

Some quick examples:

```bash
# Compress one file.
flaca /path/to/image.jpg

# Tackle a whole folder at once with a nice progress bar:
flaca -p /path/to/assets

# Tackle a whole folder, but only look for JPEG images.
flaca --no-png /path/to/assets

# Or load it up with a lot of places separately:
flaca /path/to/assets /path/to/favicon.png …

# Limit parallel processing to two images at a time.
flaca -j2 /path/to/assets

# Zopfli compression is slow and scales more or less linearly with the number
# of iterations set. Flaca uses the same default as zopflipng: 60 for small
# images, 20 for larger ones. If you're willing to trade longer processing 
# times for extra (potential) byte savings, you can try scaling up the 
# iteration count:
flaca -z 500 /path/to/favicon.png

# Or, conversely, if you want to speed up PNG compression at the expense of a
# few extra bytes, try dialing the count back:
flaca /path/to/huge.png -z 1
```

[^1] GIF images require a single, dedicated thread for optimization, separate from the `-j` limit applied to all other work. Unless/until a GIF turns up, it will just sit idle, so shouldn't noticeably impact most workloads.
