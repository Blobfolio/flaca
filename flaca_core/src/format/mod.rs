/*!
# Formatting Helpers
*/

pub mod grammar;
pub mod path;
pub mod strings;
pub mod time;



#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// Formatting options.
pub enum FormatKind {
	/// Longer output.
	Long,
	/// Condensed output.
	Short,
}
