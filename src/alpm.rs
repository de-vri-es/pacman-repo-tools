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
