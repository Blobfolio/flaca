# Flaca

[![Build Status](https://github.com/Blobfolio/flaca/workflows/Build/badge.svg)](https://github.com/Blobfolio/flaca/actions)
[![Dependency Status](https://deps.rs/repo/github/blobfolio/flaca/status.svg)](https://deps.rs/repo/github/blobfolio/flaca)

Flaca is a CLI tool for x86-64 Linux machines that simplifies the task of maximally, **losslessly** compressing JPEG and PNG images for use in production **web environments**.

It prioritizes compression over speed or resource modesty, and runs best on systems with multiple CPUs. There are only so many ways to be a JPEG, but calculating the optimal construction for a PNG can take a lot of work!

Compression is mainly achieved through the removal of metadata and optimization of pixel tables. Under the hood, Flaca leverages the `jpegtran` functionality from [MozJPEG](https://github.com/mozilla/mozjpeg) for JPEG images, and a combination of [Oxipng](https://github.com/shssoichiro/oxipng) and [Zopflipng](https://github.com/google/zopfli) for PNG images.



## Metadata

For web images, metadata is just so much wasted bandwidth. Stock photos in particular can be bloated 50% or more with embedded keywords and descriptions that browsers make zero use of. Removing that data — particularly at scale — leads to both lower hosting costs for site operators and faster page loads for visitors.

And it helps close the [digital divide](https://en.wikipedia.org/wiki/Digital_divide).

But in other contexts, _metadata may matter_.

As a general rule, you should _not_ try to feed your entire personal media library or raw print/design assets to Flaca or it may eat something important.



## Installation

Debian and Ubuntu users can just grab the pre-built `.deb` package from the [latest release](https://github.com/Blobfolio/flaca/releases/latest).

This application is written in [Rust](https://www.rust-lang.org/) and can alternatively be built from source using [Cargo](https://github.com/rust-lang/cargo):

```bash
# Clone the source.
git clone https://github.com/Blobfolio/flaca.git

# Go to it.
cd flaca

# Build as usual. Specify additional flags as desired.
cargo build \
    --bin flaca \
    --release
```

(This should work under other 64-bit Unix environments too, like MacOS.)

In addition to up-to-date `Rust`/`Cargo`, you'll also need `gcc`, `git`, `make`, `nasm`, a C++ compiler, and the dev libraries for `libjpeg` and `libpng`.

The above list may not be exhaustive, though. If you find you need anything else, please open a ticket so this list can be updated!



## Usage

It's easy. Just run `flaca [FLAGS] [OPTIONS] <PATH(S)>…`.

The following flags and options are available:
```
-h, --help           Prints help information
-l, --list <list>    Read file paths from this list (one per line).
-p, --progress       Show progress bar while minifying.
-V, --version        Prints version information
```

You can feed it any number of file or directory paths in one go, and/or toss it a text file using the `-l` option. Directories are recursively searched.

Flaca can cross filesystem and user boundaries, provided the user running the program has the relevant read/write access. (Not that you should run it as `root`, but if you did, images would still be owned by `www-data` or whatever after compression.)

Some quick examples:
```bash
# Compress one file.
flaca /path/to/image.jpg

# Tackle a whole folder at once with a nice progress bar:
flaca -p /path/to/assets

# Or load it up with a lot of places separately:
flaca /path/to/assets /path/to/favicon.png …
```



## License

See also: [CREDITS.md](CREDITS.md)

Copyright © 2022 [Blobfolio, LLC](https://blobfolio.com) &lt;hello@blobfolio.com&gt;

This work is free. You can redistribute it and/or modify it under the terms of the Do What The Fuck You Want To Public License, Version 2.

    DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
    Version 2, December 2004

    Copyright (C) 2004 Sam Hocevar <sam@hocevar.net>

    Everyone is permitted to copy and distribute verbatim or modified
    copies of this license document, and changing it is allowed as long
    as the name is changed.

    DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
    TERMS AND CONDITIONS FOR COPYING, DISTRIBUTION AND MODIFICATION

    0. You just DO WHAT THE FUCK YOU WANT TO.
