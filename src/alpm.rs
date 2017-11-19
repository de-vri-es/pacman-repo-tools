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
use std::collections::btree_map::{Entry};
use std::mem;

pub fn parse_dict<'a>(blob: &'a str) -> Result<BTreeMap<&'a str, Vec<&'a str>>, String> {
	let mut result = BTreeMap::<&str, Vec<&str>>::new();
	let mut key: Option<&'a str> = None;
	let mut values = Vec::<&'a str>::new();

	for (i, line) in blob.split('\n').enumerate() {
		let line = line.trim();
		if line.is_empty() { continue }

		if line.starts_with('%') && line.ends_with('%') {
			if let Some(key) = key {
				match result.entry(key) {
					Entry::Vacant(mut entry)   => { entry.insert(mem::replace(&mut values, Vec::new())); },
					Entry::Occupied(mut entry) => { entry.get_mut().append(&mut values); },
				}
			}
			key = Some(&line[1..line.len()-2]);
		} else {
			if key.is_none() {
				return Err(format!("got value without key on line {}", i));
			}
			values.push(line.into());
		}
	}

	return Ok(result);
}
