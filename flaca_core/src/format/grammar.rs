/*!
# Formatting Helpers: Grammar
*/

/// Inflect.
///
/// Return a string like "NUMBER LABEL" where the label is
/// appropriately singular or plural given the value.
pub fn inflect<N, S> (num: N, singular: S, plural: S) -> String
where N: Into<usize>, S: Into<String> {
	let num = num.into();
	match num {
		1 => format!("1 {}", singular.into()),
		_ => format!("{} {}", num, plural.into()),
	}
}

/// Oxford Join
///
/// Join a `Vec<String>` with correct comma usage and placement. If
/// there is one item, that item is returned. If there are two, they
/// are joined with the operator. Three or more entries will use
/// the Oxford Comma.
pub fn oxford_join<S> (mut list: Vec<String>, glue: S) -> String
where S: Into<String> {
	let glue: String = glue.into().trim().to_string();

	match list.len() {
		0 => "".to_string(),
		1 => list[0].to_string(),
		2 => list.join(&format!(" {} ", &glue)),
		_ => {
			let last = list.pop().unwrap_or("".to_string());
			format!("{}, and {}", list.join(", "), last)
		}
	}
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	/// Test Inflect.
	fn test_inflect() {
		let data = vec![
			(0, "book", "books", "0 books"),
			(1, "book", "books", "1 book"),
			(2, "book", "books", "2 books"),
		];

		for d in data.as_slice() {
			let (num, singular, plural, expected) = *d;
			assert_eq!(inflect(num as usize, singular, plural), expected);
		}
	}

	#[test]
	/// Test Oxford Join.
	fn test_oxford_join() {
		let data = vec![
			(Vec::new(), "and", ""),
			(vec!["apples".to_string()], "and", "apples"),
			(vec!["apples".to_string(), "bananas".to_string()], "and", "apples and bananas"),
			(vec!["apples".to_string(), "bananas".to_string(), "carrots".to_string()], "and", "apples, bananas, and carrots"),
			(vec!["apples".to_string(), "bananas".to_string()], "or", "apples or bananas"),
		];

		for d in data.as_slice() {
			let (set, glue, expected) = d;
			assert_eq!(oxford_join(set.clone(), *glue), *expected);
		}
	}
}
