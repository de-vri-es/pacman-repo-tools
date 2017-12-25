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

use error::ParseError;
use util::ConsumableStr;

/// Iterate over key,value pairs in an INFO blob.
///
/// INFO blobs consist of 'key = value' lines.
/// All whitespace around keys or values is removed.
///
/// Empty lines are yielded as `None`.
/// Data lines are yielded as `Some((&str, &str))`.
pub fn iterate_info<'a>(blob: &'a str) -> impl Iterator<Item = Result<Option<(&'a str, &'a str)>, ParseError<'a>>> {
	blob.split('\n').enumerate().map(|(i, line)| {
		let line = line.trim();
		if line.is_empty() {
			return Ok(None);
		}
		line.partition('=').map(|(key, _, value)| Some((key.trim(), value.trim())))
		.ok_or_else(|| ParseError::new(i, line, "expected 'key = value' or empty line"))
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_simple() {
		let blob = ["a=b", "c=d"].join("\n");
		assert_seq!(iterate_info(&blob), [
			Ok(Some(("a", "b"))),
			Ok(Some(("c", "d"))),
		])
	}

	#[test]
	fn spaces_are_stripped() {
		assert_seq!(iterate_info(" a   =    b  "), [Ok(Some(("a", "b")))])
	}

	#[test]
	fn empty_lines_are_none() {
		let blob = ["  ", "a=b", "", "c=d", ""].join("\n");
		assert_seq!(iterate_info(&blob), [
			Ok(None),
			Ok(Some(("a", "b"))),
			Ok(None),
			Ok(Some(("c", "d"))),
			Ok(None),
		])
	}

	#[test]
	fn garbage_gives_error() {
		let blob = ["ab", "a = b"].join("\n");
		let mut iterator = iterate_info(&blob);
		assert!(iterator.next().unwrap().is_err());
		assert_eq!(iterator.next(), Some(Ok(Some(("a", "b")))));
		assert_eq!(iterator.next(), None);
	}

}
