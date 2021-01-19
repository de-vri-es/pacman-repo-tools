//! Types and parsers for repository database files.
//!
//! The types represent the contents of *.db.tar files.
//! This module currently does not support reading (compressed) tar files directly.

use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::package::{Dependency, OptionalDependency, Provides};
use crate::version::PackageVersion;

mod deserializer;

pub use deserializer::{from_bytes, from_file, from_str, Error as ParseError};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[serde(deny_unknown_fields)]
pub struct DatabasePackage {
	pub filename: String,

	pub name: String,

	pub base: Option<String>,

	pub version: PackageVersion,

	#[serde(rename = "DESC")]
	pub description: String,

	#[serde(default)]
	pub groups: Vec<String>,

	#[serde(rename = "CSIZE")]
	pub compressed_size: usize,

	#[serde(rename = "ISIZE")]
	pub installed_size: usize,

	pub md5sum: String,

	pub sha256sum: String,

	pub pgpsig: Option<String>,

	pub url: Option<String>,

	#[serde(rename = "LICENSE")]
	#[serde(default)]
	pub licenses: Vec<String>,

	pub arch: String,

	#[serde(rename = "BUILDDATE")]
	pub build_date: usize,

	pub packager: String,

	#[serde(default)]
	pub replaces: Vec<String>,

	#[serde(default)]
	pub depends: Vec<Dependency>,

	#[serde(default)]
	pub conflicts: Vec<Dependency>,

	#[serde(default)]
	pub provides: Vec<Provides>,

	#[serde(default)]
	pub optdepends: Vec<OptionalDependency>,

	#[serde(default)]
	pub makedepends: Vec<Dependency>,

	#[serde(default)]
	pub checkdepends: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[serde(deny_unknown_fields)]
struct DatabasePackageDepends {
	#[serde(default)]
	depends: Vec<Dependency>,
	#[serde(default)]
	conflicts: Vec<Dependency>,
	#[serde(default)]
	provides: Vec<Provides>,
	#[serde(default)]
	optdepends: Vec<OptionalDependency>,
	#[serde(default)]
	makedepends: Vec<Dependency>,
	#[serde(default)]
	checkdepends: Vec<Dependency>,
}

impl DatabasePackage {
	pub fn from_directory(path: impl AsRef<Path>) -> Result<Self, ParseError> {
		let path = path.as_ref();
		let mut package: Self = from_file(path.join("desc"))?;
		let depends_path = path.join("depends");
		if depends_path.exists() {
			package.add_depends(from_file(&depends_path)?);
		}
		Ok(package)
	}

	fn add_depends(&mut self, mut other: DatabasePackageDepends) {
		self.depends.append(&mut other.depends);
		self.conflicts.append(&mut other.conflicts);
		self.provides.append(&mut other.provides);
		self.optdepends.append(&mut other.optdepends);
		self.makedepends.append(&mut other.makedepends);
		self.checkdepends.append(&mut other.checkdepends);
	}
}

#[derive(Debug)]
pub enum ReadDbDirError {
	ReadDir(PathBuf, std::io::Error),
	Parse(ParseError),
}

/// Read packages information from a folder containing an extracted repository database.
pub fn read_db_dir(path: impl AsRef<Path>) -> Result<Vec<DatabasePackage>, ReadDbDirError> {
	let path = path.as_ref();
	let readdir_error = |e| ReadDbDirError::ReadDir(path.into(), e);

	let dir = std::fs::read_dir(path).map_err(readdir_error)?;
	let mut packages = Vec::with_capacity(dir.size_hint().0);
	for entry in dir {
		let entry = entry.map_err(readdir_error)?;
		let stat = entry.metadata().map_err(readdir_error)?;
		if !stat.file_type().is_dir() {
			continue;
		}
		packages.push(DatabasePackage::from_directory(entry.path())?);
	}

	Ok(packages)
}

impl From<ParseError> for ReadDbDirError {
	fn from(other: ParseError) -> Self {
		Self::Parse(other)
	}
}

impl std::error::Error for ReadDbDirError {}

impl std::fmt::Display for ReadDbDirError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::ReadDir(path, e) => write!(f, "failed to read directory {}: {}", path.display(), e),
			Self::Parse(e) => e.fmt(f),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::version::Version;
	use assert2::{assert, let_assert};

	const PACKAGE_DESC: &[u8] = include_bytes!("../../tests/database-package/desc");
	const PACKAGE_DEPENDS: &[u8] = include_bytes!("../../tests/database-package/depends");

	#[test]
	fn test_parse_package_desc() {
		let_assert!(Ok(parsed) = from_bytes::<DatabasePackage>(PACKAGE_DESC));
		assert!(parsed.filename == "linux-aarch64-5.8.9-2-aarch64.pkg.tar.xz");
		assert!(parsed.name == "linux-aarch64");
		assert!(parsed.base.as_deref() == Some("linux-aarch64"));
		assert!(parsed.version.epoch == 0);
		assert!(parsed.version.pkgver == "5.8.9");
		assert!(parsed.version.pkgrel == "2");
		assert!(parsed.description == "The Linux Kernel and modules - AArch64 multi-platform");
		assert!(parsed.compressed_size == 77694896);
		assert!(parsed.installed_size == 141654447);
		assert!(parsed.md5sum == "18c5b105fe6481a61cb138e6bb9df18c");
		assert!(parsed.sha256sum == "190f99c182acf2dfd7f9db4f244fe1f9bb6a2ad2a40a2a7f72235c3c2a2ab3f6");
		assert!(parsed.pgpsig.as_deref() == Some("iQIzBAABCAAdFiEEaLNTfzmjE7PldNBndxk/FSvb5qYFAl9gGXIACgkQdxk/FSvb5qZfZg/9G1CsbW3WCQkFtYh2OMUgXEMrhUSv2+uGjjRafvaM0Mx/6iLgJUYz1Cwz16HmcBPr9y/IuVOtPGZ9FIuI578RTvAqIUmPUBNcNYjqjpPGiQyy7VD/XAvzRZ2Y5AVuVBdIHlj/5xldDQgDY5jmeEWd3Qkl6B9Zm101/2HVXMLihWTKDXHX0j0WOJvrSZGGQZfvdS3o4NaD1m+HLIZN1V2k64whroZBZn2aeZEkjWHKYfi0Jo1xHQE7VwTfft/ekvRbGOwkXx5mVshwQIDbejVQPNc1f56UqTCDB780DwboEtZ04m0ObYz68cObL0InorShFHvTwsSu+/9sQ3y57ROJCCibEnfNwHewCjlkPAaqP66zmMG+dI0Y2fZegK9KeVkYOLeU1W5Z59qTv777Qyid0tQ5YoZvwnSp+w9/sbNdXpBC7892cSn8wBgc0DUeXR7jL1yQgtfcvnHNU7dc+WVaACChgXhHG7pBcQXz8RFiChIwyEDisinpfJIEm9kaMzVn38BQUaraiYJnocIpjg6aVIU93iF3zfsIrAUHVxQlWonm/aDep35bvI7qxKcUclRq16xAfocdY3jiWQdTC3LKQXtvyGI2WMDjRYmQh3DqH7k972Ims2NPhzgHwbJcrcGQ7IGa411R2TQ7QaUZw1O8/NdmAEMJQ/tr+ahLLBMyOHY="));
		assert!(parsed.url.as_deref() == Some("http://www.kernel.org/"));
		assert!(parsed.licenses == vec!["GPL2"]);
		assert!(parsed.arch == "aarch64");
		assert!(parsed.build_date == 1600130886);
		assert!(parsed.packager == "Arch Linux ARM Build System <builder+n1@archlinuxarm.org>");
		assert!(parsed.replaces == vec!["linux-armv8"]);
	}

	#[test]
	#[rustfmt::skip]
	fn test_parse_package_depends() {
		let_assert!(Ok(parsed) = from_bytes::<DatabasePackageDepends>(PACKAGE_DEPENDS));
		assert!(parsed.depends == vec![
			Dependency::unconstrained("coreutils"),
			Dependency::unconstrained("linux-firmware"),
			Dependency::unconstrained("kmod"),
			Dependency::constrained_greater_equal("mkinitcpio", Version::new(0, "0.7", None)),
		]);
		assert!(parsed.conflicts == vec![
			Dependency::unconstrained("linux"),
		]);
		assert!(parsed.provides == vec![
			Provides::versioned("linux", Version::new(0, "5.8.9", None)),
			Provides::unversioned("WIREGUARD-MODULE"),
		]);
		assert!(parsed.optdepends == vec![
			OptionalDependency::new("crda", None, "to set the correct wireless channels of your country"),
		]);
		assert!(parsed.makedepends == vec![]);
		assert!(parsed.checkdepends == vec![]);
	}
}
