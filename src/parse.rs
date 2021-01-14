use crate::package::Constraint;
use crate::package::VersionConstraint;
use crate::version::Version;
use crate::error::ParseError;

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

/// Parse a `provides` declaration into a package name and an optional version.
pub fn parse_provides(blob: &str) -> (&str, Option<Version>) {
	if let Some((key, version)) = partition(blob, '=') {
		(key, Some(Version::from_str(version).into()))
	} else {
		(blob, None)
	}
}

/// Parse a string in the form `$pkgname-$pkgver-$pkgrel` into separate components.
pub fn parse_pkgname_pkgver(input: &str) -> Result<(&str, Version), ParseError> {
	let (name, pkgrel) = partition(input, '-')
		.ok_or_else(|| ParseError::new("missing pkver", Some(input)))?;
	let (name, pkgver) = partition(name, '-')
		.ok_or_else(|| ParseError::new("missing pkgrel", Some(input)))?;
	let (epoch, pkgver) = match partition(pkgver, ':') {
		Some((epoch, pkgver)) => {
			let epoch: i32 = epoch.parse()
				.map_err(|_| ParseError::new("invalid epoch in package version", Some(input)))?;
			(epoch, pkgver)
		},
		None => (0, pkgver),
	};

	Ok((name, Version::new(epoch, pkgver.to_string(), Some(pkgrel.to_string()))))
}

/// Parse a dependency declaration into a package name and an optional version constraint.
pub fn parse_depends(blob: &str) -> (&str, Option<VersionConstraint>) {
	if let Some(start) = blob.find(is_constraint_char) {
		let name = &blob[..start];
		let (constraint, version) = parse_constraint(&blob[start..]).unwrap();
		(
			name,
			Some(VersionConstraint {
				version: Version::from_str(version).into(),
				constraint,
			}),
		)
	} else {
		(blob, None)
	}
}

/// Check if a character is part of a version constraint operator.
fn is_constraint_char(c: char) -> bool {
	c == '>' || c == '<' || c == '='
}

/// Parse a version constraint.
fn parse_constraint(contraint: &str) -> Option<(Constraint, &str)> {
	if let Some(version) = contraint.strip_prefix(">=") {
		Some((Constraint::GreaterEqual, version))
	} else if let Some(version) = contraint.strip_prefix("<=") {
		Some((Constraint::LessEqual, version))
	} else if let Some(version) = contraint.strip_prefix(">") {
		Some((Constraint::Greater, version))
	} else if let Some(version) = contraint.strip_prefix("<") {
		Some((Constraint::Less, version))
	} else if let Some(version) = contraint.strip_prefix("==") {
		// Shame on you, packagers.
		Some((Constraint::Equal, version))
	} else if let Some(version) = contraint.strip_prefix("=") {
		Some((Constraint::Equal, version))
	} else {
		None
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use assert2::assert;

	fn version(epoch: i32, pkgver: &str) -> Version {
		Version {
			epoch,
			pkgver: pkgver.into(),
			pkgrel: None,
		}
	}

	#[test]
	fn test_parse_provides() {
		assert!(parse_provides("aap") == ("aap", None));
		assert!(parse_provides("aap=1") == ("aap", Some(version(0, "1").into())));
		assert!(parse_provides("aap=1=2") == ("aap", Some(version(0, "1=2").into())));
		assert!(parse_provides("aap=") == ("aap", Some(version(0, "").into())));
		assert!(parse_provides("=1") == ("", Some(version(0, "1").into())));
	}

	fn some_constraint(version: Version, constraint: Constraint) -> Option<VersionConstraint> {
		Some(VersionConstraint { version, constraint })
	}

	#[test]
	fn test_parse_depends() {
		// No constraint.
		assert!(parse_depends("aap") == ("aap", None));

		// Simple constraints.
		assert!(parse_depends("aap=1") == ("aap", some_constraint(version(0, "1"), Constraint::Equal)));
		assert!(parse_depends("aap==2") == ("aap", some_constraint(version(0, "2"), Constraint::Equal))); // not official
		assert!(parse_depends("aap>=3") == ("aap", some_constraint(version(0, "3"), Constraint::GreaterEqual)));
		assert!(parse_depends("aap<=4") == ("aap", some_constraint(version(0, "4"), Constraint::LessEqual)));
		assert!(parse_depends("aap>5") == ("aap", some_constraint(version(0, "5"), Constraint::Greater)));
		assert!(parse_depends("aap<6") == ("aap", some_constraint(version(0, "6"), Constraint::Less)));

		// Strange cases.
		assert!(parse_depends("aap=1=2") == ("aap", some_constraint(Version::from_str("1=2").into(), Constraint::Equal)));
		assert!(parse_depends("aap=") == ("aap", some_constraint(Version::from_str("").into(), Constraint::Equal)));
		assert!(parse_depends("=1") == ("", some_constraint(Version::from_str("1").into(), Constraint::Equal)));

		// More complicated version.
		assert!(parse_depends("aap=1.2-3") == ("aap", some_constraint(Version::new(0, "1.2".into(), Some("3".into())), Constraint::Equal)));
		assert!(parse_depends("aap=:1.2-3") == ("aap", some_constraint(Version::new(0, "1.2".into(), Some("3".into())), Constraint::Equal)));
		assert!(parse_depends("aap=5:1.2-3") == ("aap", some_constraint(Version::new(5, "1.2".into(), Some("3".into())), Constraint::Equal)));
	}
}
