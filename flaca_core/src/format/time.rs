/*!
# Formatting Helpers: Time
*/

use crate::format::{FormatKind, grammar};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};



/// Human-Readable Elapsed Time.
///
/// The short format will return a string in "HH:MM:SS" format, unless
/// your duration has crossed into days, in which case it will be in
/// "DD:HH:MM:SS" format.
///
/// The long format will be a list of the non-empty bits in English,
/// like "15 seconds" or "3 hours and 2 seconds" or "4 days, 3 hours,
/// 2 minutes, and 13 seconds".
pub fn human_elapsed(time: Instant, format: FormatKind) -> String {
	let time: usize = time.elapsed().as_secs() as usize;
	if time <= 0 {
		return match format {
			FormatKind::Short => "00:00:00".to_string(),
			FormatKind::Long => "0 seconds".to_string(),
		};
	}

	// Break down the time.
	let bits: Vec<(usize, &str, &str)> = vec![
		(time / 86400, "day", "days"),
		((time % 86400) / 3600, "hour", "hours"),
		((time % 86400 % 3600) / 60, "minute", "minutes"),
		(time % 86400 % 3600 % 60, "second", "seconds"),
	];

	// Return a shortened version.
	if FormatKind::Short == format {
		return bits.iter()
			.filter_map(|(num, singular, _)| match (*num > 0) | (&"day" != singular) {
				true => Some(format!("{:02}", num)),
				false => None,
			})
			.collect::<Vec<String>>()
			.join(":");
	}

	// A longer version.
	let out = bits.iter()
		.filter_map(|(num, singular, plural)| match *num {
			0 => None,
			_ => Some(grammar::inflect(*num, *singular, *plural)),
		})
		.collect::<Vec<String>>();

	// Let's grammar-up the response with Oxford joins.
	let joined = grammar::oxford_join(out, " and ");
	match joined.len() {
		0 => "0 seconds".to_string(),
		_ => joined
	}
}

/// Unix Time.
pub fn unixtime() -> usize {
	SystemTime::now().duration_since(UNIX_EPOCH)
		.unwrap_or(Duration::new(5, 0))
		.as_secs() as usize
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	/// Test Human Elapsed.
	fn test_human_elapsed() {
		// Make sure brand new instants are zeroed correctly.
		assert_eq!(human_elapsed(Instant::now(), FormatKind::Short), "00:00:00");
		assert_eq!(human_elapsed(Instant::now(), FormatKind::Long), "0 seconds");
	}
}
