use super::compare::compare_version_string;
use super::parse::split_parts;

use std;

/// A view into a string, split into version parts.
#[derive(Debug)]
pub struct Version<'a> {
	pub epoch: i32,
	pub pkgver: &'a str,
	pub pkgrel: Option<&'a str>,
}

/// A package version with epoch, pkgver and pkgrel.
#[derive(Debug)]
pub struct VersionBuf {
	pub epoch: i32,
	pub pkgver: String,
	pub pkgrel: Option<String>,
}

impl<'a> Version<'a> {
	pub fn new(epoch: i32, pkgver: &'a str, pkgrel: Option<&'a str>) -> Version<'a>{
		Version{epoch, pkgver, pkgrel}
	}

	pub fn from_str(s: &'a str) -> Version<'a> {
		split_parts(s)
	}
}

impl VersionBuf {
	pub fn new(epoch: i32, pkgver: String, pkgrel: Option<String>) -> VersionBuf {
		VersionBuf{epoch, pkgver, pkgrel}
	}

	pub fn from_string(s: String) -> VersionBuf {
		split_parts(s.as_ref()).into()
	}
}

impl_ord_requisites!('a; Version<'a>);
impl<'a> Ord for Version<'a> {
	fn cmp(&self, other: &Version) -> std::cmp::Ordering {
		return_not_equal!(self.epoch.cmp(&other.epoch));
		return_not_equal!(compare_version_string(self.pkgver, other.pkgver));
		match (self.pkgrel, other.pkgrel) {
			(None, None)       => std::cmp::Ordering::Equal,
			(None, Some(_))    => std::cmp::Ordering::Less,
			(Some(_), None)    => std::cmp::Ordering::Greater,
			(Some(a), Some(b)) => compare_version_string(a, b)
		}
	}
}

impl_ord_requisites!(VersionBuf);
impl Ord for VersionBuf {
	fn cmp(&self, other: &VersionBuf) -> std::cmp::Ordering {
		let as_ref : Version = self.into();
		as_ref.cmp(&other.into())
	}
}

// Conversion from Version to VersionBuf
impl<'a> Into<VersionBuf> for Version<'a> {
	fn into(self) -> VersionBuf {
		VersionBuf::new(
			self.epoch,
			self.pkgver.to_string(),
			self.pkgrel.as_ref().map(|x| x.to_string())
		)
	}
}

// Conversion from &VersionBuf to Version.
impl<'a> Into<Version<'a>> for &'a VersionBuf {
	fn into(self) -> Version<'a> {
		Version::new(
			self.epoch,
			self.pkgver.as_ref(),
			self.pkgrel.as_ref().map(|x| x.as_ref())
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

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
