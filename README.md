# Flaca

Flaca is a CLI tool for x86-64 Linux machines that simplifies the task of **losslessly** compressing JPEG and PNG images for use in production **web environments**.

It prioritizes compression over speed or resource modesty, and runs best on systems with multiple CPUs. There are only so many ways to be a JPEG, but calculating the optimal construction for a PNG takes a lot of work!

Compression is mainly achieved through the removal of metadata and optimization of pixel tables. Under the hood, Flaca leverages the `jpegtran` functionality from [MozJPEG](https://github.com/mozilla/mozjpeg) for JPEG images, and a combination of [Oxipng](https://github.com/shssoichiro/oxipng) and [Zopflipng](https://github.com/google/zopfli) for PNG images.



## Metadata

For web images, metadata is just so much wasted bandwidth. Stock photos in particular can be bloated 50% or more with embedded keywords and descriptions that browsers make zero use of. Removing that data — particularly at scale — leads to both lower hosting costs for site operators and faster page loads for visitors.

And it helps close the [digital divide](https://en.wikipedia.org/wiki/Digital_divide).

**However**, the removal of metadata is only "lossless" in the context of images destined for view in web browsers. Image editors, printers, and gallery programs extensively use metadata for all sorts of different reasons ranging from gamma correction to geolocation.

**Do not** run Flaca against your personal media library or raw design/print sources or else Google Photos won't know what to make of all your selfies!

If your personal computer is _that_ strapped for disk space, just buy an external hard drive. :)



## Installation

Installable `.deb` packages are included with each [release](https://github.com/Blobfolio/flaca/releases/latest). They should always work for the latest stable Debian and Ubuntu.



## Usage

It's easy. Just run `flaca [FLAGS] [OPTIONS] <PATH(S)>…`.

The following flags and options are available:
```bash
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



## Credits

| Library | License | Author |
| ---- | ---- | ---- |
| [imghdr](https://crates.io/crates/imghdr) | Apache-2.0 OR MIT | svartalf |
| [lazy_static](https://crates.io/crates/lazy_static) | Apache-2.0 OR MIT | Marvin Löbel |
| [mozjpeg_sys](https://crates.io/crates/mozjpeg_sys) | IJG | Kornel |
| [oxipng](https://crates.io/crates/oxipng) | MIT | Joshua Holmer |
| [tempfile-fast](https://crates.io/crates/tempfile-fast) | MIT | Chris West (Faux) |
| [tempfile](https://crates.io/crates/tempfile) | Apache-2.0 OR MIT | Ashley Mannix, Jason White, Steven Allen, The Rust Project Developers |
| [zopflipng](https://github.com/google/zopfli) | Apache-2.0 | Google |



## License

Copyright © 2021 [Blobfolio, LLC](https://blobfolio.com) &lt;hello@blobfolio.com&gt;

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
