use package::Constraint;
use package::VersionConstraint;
use version::Version;
use version::VersionBuf;

use util::ConsumableStr;

pub fn parse_provides(blob: &str) -> (&str, Option<VersionBuf>) {
	if let Some((key, _, version)) = blob.partition('=') {
		(key, Some(Version::from_str(version).into()))
	} else {
		(blob, None)
	}
}

fn eat_constraint(blob: &str) -> (&str, Option<Constraint>) {
	let mut blob = blob;
	if false { unreachable!() }
	else if blob.consume_front(">=").is_some() { (blob, Some(Constraint::GreaterEqual)) }
	else if blob.consume_front("<=").is_some() { (blob, Some(Constraint::LessEqual)) }
	else if blob.consume_front(">").is_some()  { (blob, Some(Constraint::Greater)) }
	else if blob.consume_front("<").is_some()  { (blob, Some(Constraint::Less)) }
	else if blob.consume_front("==").is_some() { (blob, Some(Constraint::Equal)) } // shame on you, packagers
	else if blob.consume_front("=").is_some()  { (blob, Some(Constraint::Equal)) }
	else { (blob, None) }
}

fn is_constraint_char(c: char) -> bool {
	c == '>' || c == '<' || c == '='
}

pub fn parse_depends(blob: &str) -> (&str, Option<VersionConstraint>) {
	if let Some(start) = blob.find(is_constraint_char) {
		let name = &blob[..start];
		let (version, constraint) = eat_constraint(&blob[start..]);
		(name, Some(VersionConstraint{version: Version::from_str(version).into(), constraint: constraint.unwrap()}))
	} else {
		(blob, None)
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_parse_provides() {
		assert_eq!(parse_provides("aap"),     ("aap", None));
		assert_eq!(parse_provides("aap=1"),   ("aap", Some(Version::new(0, "1",   None).into())));
		assert_eq!(parse_provides("aap=1=2"), ("aap", Some(Version::new(0, "1=2", None).into())));
		assert_eq!(parse_provides("aap="),    ("aap", Some(Version::new(0, "",    None).into())));
		assert_eq!(parse_provides("=1"),      ("",    Some(Version::new(0, "1",   None).into())));
	}

	fn some_constraint(version: VersionBuf, constraint: Constraint) -> Option<VersionConstraint> {
		Some(VersionConstraint{version, constraint})
	}

	#[test]
	fn test_parse_depends() {
		// No constraint.
		assert_eq!(parse_depends("aap"),     ("aap", None));

		// Simple constraints.
		assert_eq!(parse_depends("aap=1"),   ("aap", some_constraint(Version::from("1").into(), Constraint::Equal)));
		assert_eq!(parse_depends("aap==2"),  ("aap", some_constraint(Version::from("2").into(), Constraint::Equal))); // not official
		assert_eq!(parse_depends("aap>=3"),  ("aap", some_constraint(Version::from("3").into(), Constraint::GreaterEqual)));
		assert_eq!(parse_depends("aap<=4"),  ("aap", some_constraint(Version::from("4").into(), Constraint::LessEqual)));
		assert_eq!(parse_depends("aap>5"),   ("aap", some_constraint(Version::from("5").into(), Constraint::Greater)));
		assert_eq!(parse_depends("aap<6"),   ("aap", some_constraint(Version::from("6").into(), Constraint::Less)));

		// Strange cases.
		assert_eq!(parse_depends("aap=1=2"), ("aap", some_constraint(Version::from("1=2").into(), Constraint::Equal)));
		assert_eq!(parse_depends("aap="),    ("aap", some_constraint(Version::from("").into(),    Constraint::Equal)));
		assert_eq!(parse_depends("=1"),      ("",    some_constraint(Version::from("1").into(),   Constraint::Equal)));

		// More complicated version.
		assert_eq!(parse_depends("aap=1.2-3"),   ("aap", some_constraint(Version::new(0, "1.2", Some("3")).into(), Constraint::Equal)));
		assert_eq!(parse_depends("aap=:1.2-3"),  ("aap", some_constraint(Version::new(0, "1.2", Some("3")).into(), Constraint::Equal)));
		assert_eq!(parse_depends("aap=5:1.2-3"), ("aap", some_constraint(Version::new(5, "1.2", Some("3")).into(), Constraint::Equal)));
	}
}
