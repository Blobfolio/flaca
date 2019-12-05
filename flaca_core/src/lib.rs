/*!
# Flaca Core

Flaca losslessly compresses JPEG and PNG images *as much as possible*.
It achieves this through guided brute-force, passing images through a
series of independent optimizers — some of which are better at handling
certain types of content than others.

These third-party optimizers are not bundled with Flaca and must be
present on the host system to do their thing. Flaca will skip optimizers
it can't find, but for best results, it is recommended systems install
everything in the lists below.

JPEG images are sent to (in order):
* [MozJPEG](https://github.com/mozilla/mozjpeg)
* [Jpegoptim](https://github.com/tjko/jpegoptim)

PNG images are sent to (in order):
* [PNGOUT](http://advsys.net/ken/utils.htm)
* [Oxipng](https://github.com/shssoichiro/oxipng)
* [Zopflipng](https://github.com/google/zopfli)
*/

#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_import_braces)]

#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]


#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde;
#[macro_use]
mod macros;

extern crate crossbeam_channel;
extern crate imghdr;
extern crate nix;
extern crate paste;
extern crate rayon;
extern crate serde_yaml;
extern crate walkdir;

mod alert;
mod core;
mod error;
pub mod format;
mod image;
mod timer;

pub use crate::alert::{Alert, AlertKind};
pub use crate::core::{Core, CoreSettings, CoreState};
pub use crate::error::Error;
pub use crate::format as Format;
pub use crate::image::{App, ImageKind};
pub use crate::timer::Timer;
