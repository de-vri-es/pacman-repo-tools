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

impl<'a> std::fmt::Display for ParseError<'a> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.write_str(&self.message)
	}
}

impl<'a> std::error::Error for ParseError<'a> {
	fn description(&self) -> &str { &self.message }
	fn cause(&self)       -> Option<&std::error::Error> { None }
}
