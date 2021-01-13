use crate::package::Constraint;
use crate::package::VersionConstraint;
use crate::version::Version;

pub fn partition(input: &str, split: char) -> Option<(&str, &str)> {
	if let Some(i) = input.find(split) {
		Some((&input[..i], &input[i + 1..]))
	} else {
		None
	}
}

pub fn rpartition(input: &str, split: char) -> Option<(&str, &str)> {
	if let Some(i) = input.rfind(split) {
		Some((&input[..i], &input[i + 1..]))
	} else {
		None
	}
}

pub fn parse_provides(blob: &str) -> (&str, Option<Version>) {
	if let Some((key, version)) = partition(blob, '=') {
		(key, Some(Version::from_str(version).into()))
	} else {
		(blob, None)
	}
}

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

fn is_constraint_char(c: char) -> bool {
	c == '>' || c == '<' || c == '='
}

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

#[cfg(test)]
mod test {
	use super::*;
	use assert2::assert;

	#[test]
	fn test_parse_provides() {
		assert!(parse_provides("aap") == ("aap", None));
		assert!(parse_provides("aap=1") == ("aap", Some(Version::new(0, "1", None).into())));
		assert!(parse_provides("aap=1=2") == ("aap", Some(Version::new(0, "1=2", None).into())));
		assert!(parse_provides("aap=") == ("aap", Some(Version::new(0, "", None).into())));
		assert!(parse_provides("=1") == ("", Some(Version::new(0, "1", None).into())));
	}

	fn some_constraint<'a>(version: Version<'a>, constraint: Constraint) -> Option<VersionConstraint<'a>> {
		Some(VersionConstraint { version, constraint })
	}

	#[test]
	fn test_parse_depends() {
		// No constraint.
		assert!(parse_depends("aap") == ("aap", None));

		// Simple constraints.
		assert!(parse_depends("aap=1") == ("aap", some_constraint(Version::from("1").into(), Constraint::Equal)));
		assert!(parse_depends("aap==2") == ("aap", some_constraint(Version::from("2").into(), Constraint::Equal))); // not official
		assert!(parse_depends("aap>=3") == ("aap", some_constraint(Version::from("3").into(), Constraint::GreaterEqual)));
		assert!(parse_depends("aap<=4") == ("aap", some_constraint(Version::from("4").into(), Constraint::LessEqual)));
		assert!(parse_depends("aap>5") == ("aap", some_constraint(Version::from("5").into(), Constraint::Greater)));
		assert!(parse_depends("aap<6") == ("aap", some_constraint(Version::from("6").into(), Constraint::Less)));

		// Strange cases.
		assert!(parse_depends("aap=1=2") == ("aap", some_constraint(Version::from("1=2").into(), Constraint::Equal)));
		assert!(parse_depends("aap=") == ("aap", some_constraint(Version::from("").into(), Constraint::Equal)));
		assert!(parse_depends("=1") == ("", some_constraint(Version::from("1").into(), Constraint::Equal)));

		// More complicated version.
		assert!(parse_depends("aap=1.2-3") == ("aap", some_constraint(Version::new(0, "1.2", Some("3")).into(), Constraint::Equal)));
		assert!(parse_depends("aap=:1.2-3") == ("aap", some_constraint(Version::new(0, "1.2", Some("3")).into(), Constraint::Equal)));
		assert!(parse_depends("aap=5:1.2-3") == ("aap", some_constraint(Version::new(5, "1.2", Some("3")).into(), Constraint::Equal)));
	}
}
