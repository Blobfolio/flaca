use clap::{
	App,
	Arg,
};



/// CLI Menu.
pub fn menu() -> App<'static, 'static> {
	App::new("Flaca")
		.version(env!("CARGO_PKG_VERSION"))
		.author("Blobfolio, LLC. <hello@blobfolio.com>")
		.about(env!("CARGO_PKG_DESCRIPTION"))
		.arg(Arg::with_name("progress")
			.short("p")
			.long("progress")
			.help("Show progress bar while minifying.")
		)
		.arg(Arg::with_name("list")
			.short("l")
			.long("list")
			.help("Read file paths from this list.")
			.takes_value(true)
		)
		.arg(Arg::with_name("path")
			.index(1)
			.help("One or more files or directories to compress.")
			.multiple(true)
			.required_unless("list")
			.value_name("PATH(S)")
			.use_delimiter(false)
		)
		.after_help("OPTIMIZERS USED:
    Jpegoptim <https://github.com/tjko/jpegoptim>
    MozJPEG   <https://github.com/mozilla/mozjpeg>
    Oxipng    <https://github.com/shssoichiro/oxipng>
    Pngout    <http://advsys.net/ken/utils.htm>
    Zopflipng <https://github.com/google/zopfli>")
}
