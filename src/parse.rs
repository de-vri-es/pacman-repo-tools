/// Partition a string by splitting around the first occurence of a character.
pub fn partition(input: &str, split: char) -> Option<(&str, &str)> {
	if let Some(i) = input.find(split) {
		Some((&input[..i], &input[i + 1..]))
	} else {
		None
	}
}

/// Partition a string by splitting around the last occurence of a character.
pub fn rpartition(input: &str, split: char) -> Option<(&str, &str)> {
	if let Some(i) = input.rfind(split) {
		Some((&input[..i], &input[i + 1..]))
	} else {
		None
	}
}

// /// Parse a string in the form `$pkgname-$pkgver-$pkgrel` into separate components.
// pub fn parse_pkgname_pkgver(input: &str) -> Result<(&str, Version), ParseError> {
// 	let (name, pkgrel) = partition(input, '-')
// 		.ok_or_else(|| ParseError::new("missing pkver", Some(input)))?;
// 	let (name, pkgver) = partition(name, '-')
// 		.ok_or_else(|| ParseError::new("missing pkgrel", Some(input)))?;
// 	let (epoch, pkgver) = match partition(pkgver, ':') {
// 		Some((epoch, pkgver)) => {
// 			let epoch: i32 = epoch.parse()
// 				.map_err(|_| ParseError::new("invalid epoch in package version", Some(input)))?;
// 			(epoch, pkgver)
// 		},
// 		None => (0, pkgver),
// 	};

// 	Ok((name, Version::new(epoch, pkgver.to_string(), pkgrel.to_string())))
// }
