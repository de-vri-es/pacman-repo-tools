#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ParseError<'a> {
	pub line_nr: usize,
	pub message: String,
	pub token: &'a str,
}

impl<'a> ParseError<'a> {
	pub fn new<T: Into<String>>(line_nr: usize, token: &'a str, message: T) -> ParseError {
		ParseError{line_nr, message: message.into(), token}
	}
}
