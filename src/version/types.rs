use super::compare::compare_version_string;
use super::parse::consume_epoch;
use super::parse::consume_pkgrel;

use std;

/// A view into a string, split into version parts.
#[derive(Copy, Clone, Debug)]
pub struct Version<'a> {
	pub epoch: i32,
	pub pkgver: &'a str,
	pub pkgrel: Option<&'a str>,
}

/// A package version with epoch, pkgver and pkgrel.
#[derive(Clone, Debug)]
pub struct VersionBuf {
	pub epoch: i32,
	pub pkgver: String,
	pub pkgrel: Option<String>,
}

impl<'a> Version<'a> {
	pub fn new(epoch: i32, pkgver: &'a str, pkgrel: Option<&'a str>) -> Version<'a> {
		Version { epoch, pkgver, pkgrel }
	}

	pub fn from_str(version: &str) -> Version {
		let mut version = version;
		let epoch = consume_epoch(&mut version).unwrap_or(0);
		let pkgrel = consume_pkgrel(&mut version).map(|x| x.into());
		let pkgver = version.into();
		Version { epoch, pkgver, pkgrel }
	}
}

impl<'a> From<&'a str> for Version<'a> {
	fn from(blob: &'a str) -> Self {
		Self::from_str(blob)
	}
}

impl VersionBuf {
	pub fn new(epoch: i32, pkgver: String, pkgrel: Option<String>) -> VersionBuf {
		VersionBuf { epoch, pkgver, pkgrel }
	}

	pub fn from_string(s: String) -> VersionBuf {
		Version::from_str(&s).into()
	}
}

impl<'a> From<String> for VersionBuf {
	fn from(blob: String) -> Self {
		Self::from_string(blob)
	}
}

impl_ord_requisites!('a; Version<'a>);
impl<'a> Ord for Version<'a> {
	fn cmp(&self, other: &Version) -> std::cmp::Ordering {
		match self.epoch.cmp(&other.epoch) {
			std::cmp::Ordering::Equal => (),
			x => return x,
		}
		match compare_version_string(self.pkgver, other.pkgver) {
			std::cmp::Ordering::Equal => (),
			x => return x,
		}
		match (self.pkgrel, other.pkgrel) {
			(None, None) => std::cmp::Ordering::Equal,
			(None, Some(_)) => std::cmp::Ordering::Less,
			(Some(_), None) => std::cmp::Ordering::Greater,
			(Some(a), Some(b)) => compare_version_string(a, b),
		}
	}
}

impl_ord_requisites!(VersionBuf);
impl Ord for VersionBuf {
	fn cmp(&self, other: &VersionBuf) -> std::cmp::Ordering {
		let as_ref: Version = self.into();
		as_ref.cmp(&other.into())
	}
}

// Conversion from Version to VersionBuf
impl<'a> From<Version<'a>> for VersionBuf {
	fn from(version: Version<'a>) -> VersionBuf {
		VersionBuf::new(version.epoch, version.pkgver.to_string(), version.pkgrel.as_ref().map(|x| x.to_string()))
	}
}

// Conversion from &VersionBuf to Version.
impl<'a> From<&'a VersionBuf> for Version<'a> {
	fn from(version: &'a VersionBuf) -> Version<'a> {
		Version::new(version.epoch, version.pkgver.as_ref(), version.pkgrel.as_ref().map(|x| x.as_ref()))
	}
}

// Display for Version.
impl<'a> std::fmt::Display for Version<'a> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match (self.epoch, self.pkgrel) {
			(0, None) => f.write_str(self.pkgver),
			(0, Some(pkgrel)) => f.write_fmt(format_args!("{}-{}", self.pkgver, pkgrel)),
			(epoch, None) => f.write_fmt(format_args!("{}:{}", epoch, self.pkgver)),
			(epoch, Some(pkgrel)) => f.write_fmt(format_args!("{}:{}-{}", epoch, self.pkgver, pkgrel)),
		}
	}
}

// Display for VersionBuf.
impl std::fmt::Display for VersionBuf {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		Version::from(self).fmt(f)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use assert2::assert;

	#[test]
	fn test_compare_version() {
		assert!(Version::new(0, "1", Some("2")) < Version::new(0, "1", Some("3")));
		assert!(Version::new(0, "1", Some("2")) < Version::new(0, "2", Some("1")));
		assert!(Version::new(0, "1", Some("2")) < Version::new(1, "0", Some("1")));
	}

	#[test]
	fn test_compare_version_buf() {
		assert!(VersionBuf::new(0, "1".to_string(), Some("2".to_string())) < VersionBuf::new(0, "1".to_string(), Some("3".to_string())));
		assert!(VersionBuf::new(0, "1".to_string(), Some("2".to_string())) < VersionBuf::new(0, "2".to_string(), Some("1".to_string())));
		assert!(VersionBuf::new(0, "1".to_string(), Some("2".to_string())) < VersionBuf::new(1, "0".to_string(), Some("1".to_string())));
	}
}
