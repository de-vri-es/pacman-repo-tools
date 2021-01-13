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

use crate::error::ParseError;

fn parse_key(line: &str) -> Option<&str> {
	line.strip_prefix('%')?.strip_suffix('%')
}

pub fn parse_dict(blob: &str) -> Result<BTreeMap<&str, Vec<&str>>, ParseError> {
	// Iterator over trimmed lines, skipping empty lines.
	let mut lines  = blob.split('\n').map(|x| x.trim()).filter(|x| !x.is_empty());

	// Parse a key from the first line.
	let mut key = match lines.next() {
		None       => return Ok(BTreeMap::default()),
		Some(line) => parse_key(line).ok_or(ParseError::with_token(line, "expected first non-empty line to be a key in the format %NAME%"))?,
	};

	// Loop until all lines are processed.
	let mut result = BTreeMap::new();
	'key: loop {
		// Make sure we have a vector to push values to.
		let values = result.entry(key).or_insert(Vec::default());

		// Loop over value lines.
		for line in &mut lines {
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


#[cfg(test)]
mod test {
	use super::*;
	use maplit::btreemap;
	use assert2::assert;

	#[test]
	fn simple() {
		let blob = ["%FOO%", "aap", "noot", "mies"].join("\n");
		assert!(parse_dict(blob.as_ref()) == Ok(btreemap!{"FOO" => vec!["aap", "noot", "mies"]}));
	}

	#[test]
	fn lines_are_trimmed() {
		let blob = ["  %FOO%  ", "    aap  ", "   noot", "mies  "].join("\n");
		assert!(parse_dict(blob.as_ref()) == Ok(btreemap!{"FOO" => vec!["aap", "noot", "mies"]}));
	}

	#[test]
	fn empty_lines_are_ignored() {
		let blob = ["", "", "", "%FOO%", "", "", "", "aap", "", "noot", "mies", "", ""].join("\n");
		assert!(parse_dict(blob.as_ref()) == Ok(btreemap!{"FOO" => vec!["aap", "noot", "mies"]}));
	}

	#[test]
	fn multiple_keys() {
		let blob = [
			"%FOO%", "aap", "noot", "mies",
			"%BAR%", "wim", "zus", "jet",
		].join("\n");
		assert!(parse_dict(blob.as_ref()) == Ok(btreemap!{
			"FOO" => vec!["aap", "noot", "mies"],
			"BAR" => vec!["wim", "zus", "jet"]
		}));
	}

	#[test]
	fn keys_can_be_reopened() {
		let blob = [
			"%FOO%", "aap", "noot",
			"%BAR%", "wim",
			"%FOO%", "mies",
			"%BAR%", "zus",
			"%BAR%", "jet",
		].join("\n");
		assert!(parse_dict(blob.as_ref()) == Ok(btreemap!{
			"FOO" => vec!["aap", "noot", "mies"],
			"BAR" => vec!["wim", "zus", "jet"]
		}));
	}
}
