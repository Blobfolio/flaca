/*!
# Flaca: Zopflipng Errors.
*/

use std::fmt;

#[derive(Debug, Clone, Copy)]
pub(crate) enum ZopfliError {
	HistogramRange,
	LeafSize,
	LitLen,
	LitLenLiteral,
	LMCDistance,
	MatchRange,
	NoDistance,
	NoLength,
	PathLength,
	SplitRange,
	SublenLength,
	TreeSymbol,
	Write,
}

impl fmt::Display for ZopfliError {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl std::error::Error for ZopfliError {}

impl ZopfliError {
	/// # As String Slice.
	pub(crate) const fn as_str(self) -> &'static str {
		match self {
			Self::HistogramRange => "invalid histogram range",
			Self::LeafSize => "insufficient maxbits for leaves",
			Self::LitLen => "invalid litlen",
			Self::LitLenLiteral => "invalid litlen literal",
			Self::LMCDistance => "LMC returned an unexpected distance",
			Self::MatchRange => "invalid match range",
			Self::NoDistance => "expected non-zero distance",
			Self::NoLength => "expected non-zero length",
			Self::PathLength => "invalid path length",
			Self::SplitRange => "invalid split range",
			Self::SublenLength => "incorrectly sized sublength array",
			Self::TreeSymbol => "invalid tree symbol",
			Self::Write => "failed to write output",
		}
	}
}
