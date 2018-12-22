// Flaca
//
// ©2018 Blobfolio, LLC <hello@blobfolio.com>
// License: WTFPL <http://www.wtfpl.net>


#[macro_use]
extern crate lazy_static;

extern crate ansi_term;
extern crate term_size;

pub mod lugar;
pub mod diario;

fn main() {
	let x = lugar::Lugar::Path;
}
