// Copyright (c) 2017, Maarten de Vries
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// * Redistributions of source code must retain the above copyright notice, this
//   list of conditions and the following disclaimer.
//
// * Redistributions in binary form must reproduce the above copyright notice,
//   this list of conditions and the following disclaimer in the documentation
//   and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::collections::BTreeMap;

use util::ConsumableStr;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ParseError<'a> {
	pub line_nr: usize,
	pub message: String,
	pub token: &'a str,
}

impl<'a> ParseError<'a> {
	fn new<T: Into<String>>(line_nr: usize, token: &'a str, message: T) -> ParseError {
		ParseError{line_nr, message: message.into(), token}
	}
}

fn parse_key(line: &str) -> Option<&str> {
	let mut line = line;
	if line.consume_front_n(1) == Some("%") && line.consume_back_n(1) == Some("%") {
		Some(line)
	} else {
		None
	}
}

pub fn parse_dict<'a>(blob: &'a str) -> Result<BTreeMap<&'a str, Vec<&'a str>>, ParseError<'a>> {
	// Iterator over trimmed lines, skipping empty lines.
	let mut lines  = blob.split('\n').map(|x| x.trim()).enumerate().filter(|&(_, x)| !x.is_empty());

	// Parse a key from the first line.
	let mut key = match lines.next() {
		None            => return Ok(BTreeMap::default()),
		Some((i, line)) => parse_key(line).ok_or(ParseError::new(i, line, "expected first non-empty line to be a key in the format %NAME%"))?,
	};

	// Loop until all lines are processed.
	let mut result = BTreeMap::new();
	'key: loop {
		// Make sure we have a vector to push values to.
		let values = result.entry(key).or_insert(Vec::default());

		// Loop over value lines.
		for (_, line) in &mut lines {
			// If a key is found, continue the outer loop.
			if let Some(new_key) = parse_key(line) {
				key = new_key;
				continue 'key;
			}
			values.push(line);
		}
		break;
	}

	Ok(result)
}
