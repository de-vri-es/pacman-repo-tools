use std::collections::BTreeMap;

use util::ConsumableStr;

pub fn parse_dict(blob: &str) -> Result<BTreeMap<String, Vec<String>>, String> {
	let mut result = BTreeMap::<String, Vec<String>>::new();

	for (i, line) in blob.split('\n').enumerate() {
		let line = line.trim();
		if line.is_empty() { continue }

		if let Some((name, _, value)) = line.partition('=') {
			let name = name.trim();
			let value = value.trim().into();
			result.entry(name.into()).or_insert_with(|| Vec::<String>::new()).push(value);
		} else {
			return Err(format!("expected `=` on line {}", i))
		}
	}

	return Ok(result);
}
