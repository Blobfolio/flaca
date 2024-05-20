/*!
# Flaca Job Server
*/

use crate::{
	E_PNG,
	EncodingError,
	FlacaError,
	ImageKind,
};
use crossbeam_channel::Receiver;
use dactyl::{
	NiceElapsed,
	NiceU64,
	traits::NiceInflection,
};
use dowser::Extension;
use fyi_msg::{
	BeforeAfter,
	Msg,
	MsgKind,
	Progless,
};
use std::{
	num::NonZeroUsize,
	path::{
		Path,
		PathBuf,
	},
	sync::{
		Arc,
		atomic::{
			AtomicBool,
			AtomicU64,
			Ordering::{
				Acquire,
				Relaxed,
				SeqCst,
			},
		},
	},
	thread,
};



/// # Progress Counters.
static SKIPPED: AtomicU64 = AtomicU64::new(0);
static BEFORE: AtomicU64 = AtomicU64::new(0);
static AFTER: AtomicU64 = AtomicU64::new(0);



#[inline(never)]
/// # Crunch Everything!
///
/// This processes each image in `files` in parallel using up to `threads`
/// threads.
pub(super) fn exec(mut threads: NonZeroUsize, kinds: ImageKind, files: &[PathBuf])
-> Result<(), FlacaError> {
	// Sort out the threads and job server.
	let total = NonZeroUsize::new(files.len()).ok_or(FlacaError::NoImages)?;
	if total < threads { threads = total; }

	// Set up the killswitch.
	let killed = Arc::new(AtomicBool::new(false));
	sigint(Arc::clone(&killed), None);

	// Thread business!
	let (tx, rx) = crossbeam_channel::bounded::<&Path>(threads.get());
	thread::scope(#[inline(always)] |s| {
		// Set up the worker threads.
		let mut workers = Vec::with_capacity(threads.get());
		for _ in 0..threads.get() {
			workers.push(s.spawn(#[inline(always)] ||
				while let Ok(p) = rx.recv() {
					let _res = crate::image::encode(p, kinds);
				}
			));
		}

		// Push all the files to it, then drop the sender to disconnect.
		for file in files {
			if killed.load(Acquire) || tx.send(file).is_err() { break; }
		}
		drop(tx);

		// Wait for the threads to finish!
		for worker in workers { let _res = worker.join(); }
	});
	drop(rx);

	// Early abort?
	if killed.load(Acquire) { Err(FlacaError::Killed) }
	else { Ok(()) }
}

#[inline(never)]
/// # Crunch Everything (with Progress)!
///
/// This is the same as `exec`, but includes a progress bar and summary.
pub(super) fn exec_pretty(mut threads: NonZeroUsize, kinds: ImageKind, files: &[PathBuf])
-> Result<(), FlacaError> {
	#[inline(never)]
	/// # Worker Business.
	///
	/// This is the worker callback; it listens for image paths, processing
	/// them as they come in.
	fn work(rx: &Receiver::<&Path>, progress: &Progless, kinds: ImageKind) {
		while let Ok(p) = rx.recv() {
			let name = p.to_string_lossy();
			progress.add(&name);

			match crate::image::encode(p, kinds) {
				// Happy.
				Ok((b, a)) => {
					BEFORE.fetch_add(b, Relaxed);
					AFTER.fetch_add(a, Relaxed);
				},
				// Skipped.
				Err(e) => {
					SKIPPED.fetch_add(1, Relaxed);
					if ! matches!(e, EncodingError::Skipped) {
						skip_warn(p, kinds, e, progress);
					}
				},
			}

			progress.remove(&name);
		}
	}

	let total = NonZeroUsize::new(files.len()).ok_or(FlacaError::NoImages)?;
	if total < threads { threads = total; }

	// Boot up a progress bar.
	let progress = Progless::try_from(total.get())?.with_reticulating_splines("Flaca");

	// Set up the killswitch.
	let killed = Arc::new(AtomicBool::new(false));
	sigint(Arc::clone(&killed), Some(progress.clone()));

	// Thread business!
	let (tx, rx) = crossbeam_channel::bounded::<&Path>(threads.get());
	thread::scope(#[inline(always)] |s| {
		// Set up the worker threads.
		let mut workers = Vec::with_capacity(threads.get());
		for _ in 0..threads.get() {
			workers.push(s.spawn(#[inline(always)] ||
				work(&rx, &progress, kinds)
			));
		}

		// Push all the files to it, then drop the sender to disconnect.
		for file in files {
			if killed.load(Acquire) || tx.send(file).is_err() { break; }
		}
		drop(tx);

		// Wait for the threads to finish!
		for worker in workers { let _res = worker.join(); }
	});
	drop(rx);

	// Summarize!
	let elapsed = progress.finish();
	let skipped = SKIPPED.load(Acquire);
	if skipped == 0 {
		progress.summary(MsgKind::Crunched, "image", "images")
	}
	else {
		// And summarize what we did do.
		Msg::crunched(format!(
			"{}\x1b[2m/\x1b[0m{} in {}.",
			NiceU64::from(total.get() as u64 - skipped),
			total.nice_inflect("image", "images"),
			NiceElapsed::from(elapsed),
		))
	}
		.with_bytes_saved(BeforeAfter::from((
			BEFORE.load(Acquire),
			AFTER.load(Acquire),
		)))
		.eprint();

	// Early abort?
	if killed.load(Acquire) { Err(FlacaError::Killed) }
	else { Ok(()) }
}



#[inline(never)]
/// # Hook Up CTRL+C.
///
/// Once stops processing new items, twice forces immediate shutdown.
fn sigint(killed: Arc<AtomicBool>, progress: Option<Progless>) {
	let _res = ctrlc::set_handler(move ||
		if killed.compare_exchange(false, true, SeqCst, Relaxed).is_ok() {
			if let Some(p) = &progress { p.sigint(); }
		}
		else { std::process::exit(1); }
	);
}

#[cold]
#[inline(never)]
/// # Maybe Warn About a Skipped File.
fn skip_warn(file: &Path, kinds: ImageKind, err: EncodingError, progress: &Progless) {
	// If we're only compressing one or the other kind of image, make sure the
	// file extension belongs to that kind before complaining about it.
	if ! matches!(kinds, ImageKind::All) {
		if Some(E_PNG) == Extension::try_from3(file) {
			if ! kinds.supports_png() { return; }
		}
		else if ! kinds.supports_jpeg() { return; }
	}

	progress.push_msg(Msg::custom("Skipped", 11, &format!(
		"{} \x1b[2m({})\x1b[0m",
		file.to_string_lossy(),
		err.as_str(),
	)), true);
}
