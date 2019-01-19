/*!
Flaca is a lossless JPEG and PNG image optimizer.

Or more precisely, it wraps several lossless JPEG and PNG in one simple
command-line interface to brute-force compression savings.

For more information, run `flaca --help`.
*/

#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_import_braces)]

#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]

#[macro_use]
extern crate lazy_static;
extern crate clap;
extern crate walkdir;

pub mod ajustes;
pub mod imagen;

use ajustes::Ajustes;



fn main() {
	let settings = Ajustes::init();
}
