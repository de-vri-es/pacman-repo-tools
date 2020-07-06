use std;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ParseError<'a> {
	pub message: String,
	pub token: Option<&'a str>,
}

impl<'a> ParseError<'a> {
	pub fn new(message: impl Into<String>, token: Option<&'a str>) -> ParseError<'a> {
		ParseError{message: message.into(), token: token}
	}

	pub fn with_token(token: &'a str, message: impl Into<String>) -> ParseError<'a> {
		ParseError::new(message, Some(token))
	}

	pub fn no_token(message: impl Into<String>) -> ParseError<'a> {
		ParseError::new(message, None)
	}
}

impl std::fmt::Display for ParseError<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.write_str(&self.message)
	}
}

impl std::error::Error for ParseError<'_> {}
