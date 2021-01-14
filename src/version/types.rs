use super::compare::compare_version_string;
use super::parse::consume_epoch;
use super::parse::consume_pkgrel;

use std;

/// A view into a string, split into version parts.
#[derive(Clone, Debug)]
pub struct Version {
	pub epoch: i32,
	pub pkgver: String,
	pub pkgrel: Option<String>,
}

impl Version {
	pub fn new(epoch: i32, pkgver: String, pkgrel: Option<String>) -> Version {
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

impl_ord_requisites!(Version);
impl<'a> Ord for Version {
	fn cmp(&self, other: &Version) -> std::cmp::Ordering {
		match self.epoch.cmp(&other.epoch) {
			std::cmp::Ordering::Equal => (),
			x => return x,
		}
		match compare_version_string(&self.pkgver, &other.pkgver) {
			std::cmp::Ordering::Equal => (),
			x => return x,
		}
		match (&self.pkgrel, &other.pkgrel) {
			(None, None) => std::cmp::Ordering::Equal,
			(None, Some(_)) => std::cmp::Ordering::Less,
			(Some(_), None) => std::cmp::Ordering::Greater,
			(Some(a), Some(b)) => compare_version_string(&a, &b),
		}
	}
}

// Display for Version.
impl std::fmt::Display for Version {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match (self.epoch, &self.pkgrel) {
			(0, None) => f.write_str(&self.pkgver),
			(0, Some(pkgrel)) => f.write_fmt(format_args!("{}-{}", self.pkgver, pkgrel)),
			(epoch, None) => f.write_fmt(format_args!("{}:{}", epoch, self.pkgver)),
			(epoch, Some(pkgrel)) => f.write_fmt(format_args!("{}:{}-{}", epoch, self.pkgver, pkgrel)),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use assert2::assert;

	fn version(epoch: i32, pkgver: &str, pkgrel: &str) -> Version {
		Version {
			epoch,
			pkgver: pkgver.into(),
			pkgrel: Some(pkgrel.into()),
		}
	}

	#[test]
	fn test_compare_version() {
		assert!(version(0, "1", "2") < version(0, "1", "3"));
		assert!(version(0, "1", "2") < version(0, "2", "1"));
		assert!(version(0, "1", "2") < version(1, "0", "1"));
	}
}
