use std::cmp::Ordering;

use super::compare::compare_version_string;
use crate::parse::{partition, rpartition};

/// A version with optional pkgrel.
#[derive(Clone, Debug)]
pub struct Version {
	pub epoch: i32,
	pub pkgver: String,
	pub pkgrel: Option<String>,
}

/// A package version including mandetory pkgrel.
#[derive(Clone, Debug)]
pub struct PackageVersion {
	pub epoch: i32,
	pub pkgver: String,
	pub pkgrel: String,
}

/// Error when parsing a [`Version`] from a string.
#[derive(Debug, Eq, PartialEq)]
pub enum VersionFromStrError {
	InvalidEpoch,
	InvalidPkgver,
	InvalidPkgrel,
}

/// Error when parsing a [`Version`] from a string.
#[derive(Debug, Eq, PartialEq)]
pub enum PackageVersionFromStrError {
	InvalidEpoch,
	InvalidPkgver,
	InvalidPkgrel,
	MissingPkgRel,
}

impl Version {
	/// Create a new version with epoch, pkgver and optional pkgrel.
	pub fn new(epoch: i32, pkgver: impl Into<String>, pkgrel: Option<String>) -> Self {
		let pkgver = pkgver.into();
		let pkgrel = pkgrel.into();
		Self { epoch, pkgver, pkgrel }
	}
}

impl PackageVersion {
	/// Create a new version with epoch, pkgver and pkgrel.
	pub fn new(epoch: i32, pkgver: impl Into<String>, pkgrel: impl Into<String>) -> Self {
		let pkgver = pkgver.into();
		let pkgrel = pkgrel.into();
		Self { epoch, pkgver, pkgrel }
	}
}

impl std::str::FromStr for Version {
	type Err = VersionFromStrError;

	fn from_str(input: &str) -> Result<Self, Self::Err> {
		// Parse the epoch if it exists.
		let (epoch, version) = if let Some((epoch, rest)) = partition(input, ':') {
			let epoch = epoch.parse().map_err(|_| VersionFromStrError::InvalidEpoch)?;
			(epoch, rest)
		} else {
			(0, input)
		};

		// Split into pkgver and pkgrel.
		let (pkgver, pkgrel) = if let Some((pkgver, pkgrel)) = rpartition(version, '-') {
			(pkgver, Some(pkgrel))
		} else {
			(version, None)
		};

		// TODO: this is disabled because makepkg generates invalid versions for .so provides under some conditions.
		// TODO: re-enable when makepkg and the real databases are fixed.
		// Check the pkgver for invalid characters.
		// if pkgver.chars().any(|c| !c.is_ascii() || c.is_ascii_whitespace() || c == '/' || c == ':' || c == '-') {
		// 	return Err(VersionFromStrError::InvalidPkgver);
		// }

		// Check the pkgrel for invalid characters.
		if let Some(pkgrel) = pkgrel {
			if pkgrel.chars().any(|c| !c.is_ascii_digit() && c != '.') {
				return Err(VersionFromStrError::InvalidPkgrel);
			}
		}

		Ok(Self::new(epoch, pkgver, pkgrel.map(|x| x.into())))
	}
}

impl std::str::FromStr for PackageVersion {
	type Err = PackageVersionFromStrError;

	fn from_str(input: &str) -> Result<Self, Self::Err> {
		let Version { epoch, pkgver, pkgrel } = input.parse()?;
		if let Some(pkgrel) = pkgrel {
			Ok(PackageVersion { epoch, pkgver, pkgrel })
		} else {
			Err(PackageVersionFromStrError::MissingPkgRel)
		}
	}
}

impl<'de> serde::Deserialize<'de> for Version {
	fn deserialize<D: serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		struct Visitor;
		impl<'de> serde::de::Visitor<'de> for Visitor {
			type Value = Version;

			fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(f, "a version string")
			}

			fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Version, E> {
				value.parse().map_err(|e| E::custom(e))
			}
		}

		deserializer.deserialize_str(Visitor)
	}
}

impl<'de> serde::Deserialize<'de> for PackageVersion {
	fn deserialize<D: serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		struct Visitor;
		impl<'de> serde::de::Visitor<'de> for Visitor {
			type Value = PackageVersion;

			fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(f, "a version string")
			}

			fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<PackageVersion, E> {
				value.parse().map_err(|e| E::custom(e))
			}
		}

		deserializer.deserialize_str(Visitor)
	}
}

impl std::error::Error for VersionFromStrError {}
impl std::error::Error for PackageVersionFromStrError {}

impl std::fmt::Display for VersionFromStrError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::InvalidEpoch => write!(f, "invalid epoch in version"),
			Self::InvalidPkgver => write!(f, "invalid pkgver in version"),
			Self::InvalidPkgrel => write!(f, "invalid pkgrel in version"),
		}
	}
}

impl std::fmt::Display for PackageVersionFromStrError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::InvalidEpoch => write!(f, "invalid epoch in version"),
			Self::InvalidPkgver => write!(f, "invalid pkgver in version"),
			Self::InvalidPkgrel => write!(f, "invalid pkgrel in version"),
			Self::MissingPkgRel => write!(f, "missing pkgrel in version"),
		}
	}
}

impl From<VersionFromStrError> for PackageVersionFromStrError {
	fn from(other: VersionFromStrError) -> Self {
		match other {
			VersionFromStrError::InvalidEpoch => Self::InvalidEpoch,
			VersionFromStrError::InvalidPkgver => Self::InvalidPkgver,
			VersionFromStrError::InvalidPkgrel => Self::InvalidPkgrel,
		}
	}
}

impl<'a> Ord for Version {
	fn cmp(&self, other: &Version) -> Ordering {
		match self.epoch.cmp(&other.epoch) {
			Ordering::Equal => (),
			x => return x,
		}
		match compare_version_string(&self.pkgver, &other.pkgver) {
			Ordering::Equal => (),
			x => return x,
		}
		match (&self.pkgrel, &other.pkgrel) {
			(None, None) => Ordering::Equal,
			(None, Some(_)) => Ordering::Less,
			(Some(_), None) => Ordering::Greater,
			(Some(a), Some(b)) => compare_version_string(&a, &b),
		}
	}
}

impl<'a> Ord for PackageVersion {
	fn cmp(&self, other: &PackageVersion) -> Ordering {
		match self.epoch.cmp(&other.epoch) {
			Ordering::Equal => (),
			x => return x,
		}
		match compare_version_string(&self.pkgver, &other.pkgver) {
			Ordering::Equal => (),
			x => return x,
		}
		compare_version_string(&self.pkgrel, &other.pkgrel)
	}
}

impl PartialOrd for Version {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl PartialOrd for PackageVersion {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Eq for Version {}
impl Eq for PackageVersion {}

impl PartialEq for Version {
	fn eq(&self, other: &Self) -> bool {
		self.cmp(other) == Ordering::Equal
	}
}

impl PartialEq for PackageVersion {
	fn eq(&self, other: &Self) -> bool {
		self.cmp(other) == Ordering::Equal
	}
}

impl std::fmt::Display for Version {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match (self.epoch, &self.pkgrel) {
			(0, None) => write!(f, "{}", self.pkgver),
			(epoch, None) => write!(f, "{}:{}", epoch, self.pkgver),
			(0, Some(pkgrel)) => write!(f, "{}-{}", self.pkgver, pkgrel),
			(epoch, Some(pkgrel)) => write!(f, "{}:{}-{}", epoch, self.pkgver, pkgrel),
		}
	}
}

impl std::fmt::Display for PackageVersion {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		if self.epoch == 0 {
			write!(f, "{}-{}", self.pkgver, self.pkgrel)
		} else {
			write!(f, "{}:{}-{}", self.epoch, self.pkgver, self.pkgrel)
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use assert2::assert;

	#[test]
	fn test_compare_package_version() {
		assert!(PackageVersion::new(0, "1", "2") < PackageVersion::new(0, "1", "3"));
		assert!(PackageVersion::new(0, "1", "2") < PackageVersion::new(0, "2", "1"));
		assert!(PackageVersion::new(0, "1", "2") < PackageVersion::new(1, "0", "1"));
	}

	#[test]
	fn test_parse() {
		let parse = |x: &str| -> Result<PackageVersion, _> {
			x.parse()
		};

		assert!(parse("1.2.3-4") == Ok(PackageVersion::new(0, "1.2.3", "4")));
		assert!(parse("1.2.3-4.5") == Ok(PackageVersion::new(0, "1.2.3", "4.5")));
		assert!(parse("5:1.2.3-4") == Ok(PackageVersion::new(5, "1.2.3", "4")));

		assert!(parse("aap:1.2.3-4") == Err(PackageVersionFromStrError::InvalidEpoch));
		// TODO: checking pkgver was disabled to work around makepkg bug
		// assert!(parse("aap-noot-1") == Err(PackageVersionFromStrError::InvalidPkgver));
		assert!(parse("1.2.3-foo") == Err(PackageVersionFromStrError::InvalidPkgrel));
		assert!(parse("1.2.3") == Err(PackageVersionFromStrError::MissingPkgRel));
	}
}
