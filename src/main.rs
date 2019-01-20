/*!
Flaca is a lossless JPEG and PNG image optimizer.

Or more precisely, it wraps several lossless JPEG and PNG in one simple
command-line interface to brute-force la mierda out of its targets.

For more information, run `flaca --help`.
*/

#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_import_braces)]

#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]

extern crate clap;
#[macro_use]
extern crate lazy_static;
extern crate num_cpus;
extern crate walkdir;

pub mod ajustes;
pub mod imagen;

use ajustes::Ajustes;



/// The magic is in the libraries.
fn main() {
	Ajustes::init();
}
