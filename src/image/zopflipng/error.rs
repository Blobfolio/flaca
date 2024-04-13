/*!
# Flaca: Zopflipng Errors.
*/

use std::fmt;

#[derive(Debug, Clone, Copy)]
pub(crate) enum ZopfliError {
	HistogramRange,
	LeafSize(usize, usize),
	LitLen(u16),
	LitLenLiteral(u16),
	LMCDistance,
	MatchRange(usize, usize, u16),
	PathLength,
	NoDistance,
	NoLength,
	SplitRange(usize, usize, usize),
	SublenLength,
	Write,
}

impl fmt::Display for ZopfliError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("(zopfli) ")?;

		match self {
			Self::HistogramRange => f.write_str("invalid histogram range"),
			Self::LeafSize(bits, leaves) => f.write_fmt(format_args!("{bits} insufficient for {leaves}")),
			Self::LitLen(n) => f.write_fmt(format_args!("invalid LitLen: {n}")),
			Self::LitLenLiteral(n) => f.write_fmt(format_args!("invalid LitLen literal: {n}")),
			Self::LMCDistance => f.write_str("incorrect cache distance found"),
			Self::MatchRange(start, end, pos) => f.write_fmt(format_args!("invalid match length for {start}..{end}: {pos}")),
			Self::NoDistance => f.write_str("expected non-zero distance"),
			Self::NoLength => f.write_str("expected non-zero length"),
			Self::PathLength => f.write_str("path length/find mismatch"),
			Self::SplitRange(start, end, pos) => f.write_fmt(format_args!("invalid split position for {start}..{end}: {pos}")),
			Self::SublenLength => f.write_str("invalid sublen slice length"),
			Self::Write => f.write_str("failed to write output"),
		}
	}
}

impl std::error::Error for ZopfliError {}
