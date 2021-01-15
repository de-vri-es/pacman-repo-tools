use crate::version::{Version, VersionFromStrError};
use crate::parse::partition;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Provides {
	pub name: String,
	pub version: Option<Version>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dependency {
	pub name: String,
	pub version: Option<VersionConstraint>,
}

/// A version constraint operator.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Constraint {
	Equal,
	Greater,
	GreaterEqual,
	Less,
	LessEqual,
}

/// A version constraint as used for package dependencies.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionConstraint {
	pub version: Version,
	pub constraint: Constraint,
}

impl Provides {
	pub fn unversioned(name: impl Into<String>) -> Self {
		let name = name.into();
		Self { name, version: None }
	}

	pub fn versioned(name: impl Into<String>, version: Version) -> Self {
		let name = name.into();
		let version = Some(version);
		Self { name, version }
	}
}

impl Dependency {
	pub fn unconstrained(name: impl Into<String>) -> Self {
		let name = name.into();
		Self { name, version: None }
	}

	pub fn constrained(name: impl Into<String>, version: VersionConstraint) -> Self {
		let name = name.into();
		let version = Some(version);
		Self { name, version }
	}

	pub fn constrained_equal(name: impl Into<String>, version: Version) -> Self {
		let name = name.into();
		let version = Some(VersionConstraint {
			version,
			constraint: Constraint::Equal,
		});
		Self { name, version }
	}

	pub fn constrained_less(name: impl Into<String>, version: Version) -> Self {
		let name = name.into();
		let version = Some(VersionConstraint {
			version,
			constraint: Constraint::Less,
		});
		Self { name, version }
	}

	pub fn constrained_less_equal(name: impl Into<String>, version: Version) -> Self {
		let name = name.into();
		let version = Some(VersionConstraint {
			version,
			constraint: Constraint::LessEqual,
		});
		Self { name, version }
	}

	pub fn constrained_greater(name: impl Into<String>, version: Version) -> Self {
		let name = name.into();
		let version = Some(VersionConstraint {
			version,
			constraint: Constraint::Greater,
		});
		Self { name, version }
	}

	pub fn constrained_greater_equal(name: impl Into<String>, version: Version) -> Self {
		let name = name.into();
		let version = Some(VersionConstraint {
			version,
			constraint: Constraint::GreaterEqual,
		});
		Self { name, version }
	}
}

impl std::str::FromStr for Provides {
	// TODO: also check for invalid package names
	type Err = VersionFromStrError;

	fn from_str(input: &str) -> Result<Self, Self::Err> {
		if let Some((name, version)) = partition(input, '=') {
			Ok(Provides {
				name: name.into(),
				version: Some(version.parse()?),
			})
		} else {
			Ok(Provides {
				name: input.into(),
				version: None,
			})
		}
	}
}

impl std::str::FromStr for Dependency {
	// TODO: also check for invalid package names and version constraints.
	type Err = VersionFromStrError;

	fn from_str(input: &str) -> Result<Self, Self::Err> {
		if let Some(start) = input.find(is_constraint_char) {
			let name = &input[..start];
			let (constraint, version) = parse_constraint(&input[start..]).unwrap();
			Ok(Dependency {
				name: name.into(),
				version: Some(VersionConstraint {
					version: version.parse()?,
					constraint,
				})
			})
		} else {
			Ok(Dependency {
				name: input.into(),
				version: None,
			})
		}
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

impl<'de> serde::Deserialize<'de> for Provides {
	fn deserialize<D: serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		struct Visitor;
		impl<'de> serde::de::Visitor<'de> for Visitor {
			type Value = Provides;

			fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(f, "a provided package name with optional version")
			}

			fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
				value.parse().map_err(|e| E::custom(format_args!("invalid version in provides declaration: {}", e)))
			}
		}

		deserializer.deserialize_str(Visitor)
	}
}

impl<'de> serde::Deserialize<'de> for Dependency {
	fn deserialize<D: serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		struct Visitor;
		impl<'de> serde::de::Visitor<'de> for Visitor {
			type Value = Dependency;

			fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(f, "a dependency name with optional version constraint")
			}

			fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
				value.parse().map_err(|e| E::custom(format_args!("invalid version in dependency declaration: {}", e)))
			}
		}

		deserializer.deserialize_str(Visitor)
	}
}
