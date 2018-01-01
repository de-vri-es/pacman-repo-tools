use std::error;
use std::fmt;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ParseError {
	pub message: String,
	pub token_start: usize,
	pub token_end:   usize,
}

impl ParseError {
	pub fn for_token<S: Into<String>>(blob: &str, token: &str, message: S) -> ParseError {
		let token_start = token.as_ptr() as usize - blob.as_ptr() as usize;
		let token_end   = token_start + token.len();
		ParseError{message: message.into(), token_start, token_end}
	}

	pub fn extract_token<'a>(&self, data: &'a str) -> &'a str {
		&data[self.token_start..self.token_end]
	}
}

impl fmt::Display for ParseError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.write_str(&self.message)
	}
}

impl error::Error for ParseError {
	fn description(&self) -> &str                  { &self.message }
	fn cause(&self)       -> Option<&error::Error> { None }
}
