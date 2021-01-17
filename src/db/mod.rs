//! Types and parsers for repository database files.
//!
//! The types represent the contents of *.db.tar files.
//! This module currently does not support reading (compressed) tar files directly.

use serde::Deserialize;

use crate::version::PackageVersion;
use crate::package::{Dependency, Provides};

mod deserializer;

pub use deserializer::{from_bytes, from_str, from_file};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[serde(deny_unknown_fields)]
pub struct DatabasePackageDesc {
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
	pub licenses: Vec<String>,
	pub arch: String,
	#[serde(rename = "BUILDDATE")]
	pub build_date: usize,
	pub packager: String,
	#[serde(default)]
	pub replaces: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[serde(deny_unknown_fields)]
pub struct DatabasePackageDepends {
	#[serde(default)]
	pub depends: Vec<Dependency>,
	#[serde(default)]
	pub conflicts: Vec<Dependency>,
	#[serde(default)]
	pub provides: Vec<Provides>,
	// TODO: Use dedicated type for opt depends that allow for hint why it is needed
	#[serde(default)]
	pub optdepends: Vec<Dependency>,
	#[serde(default)]
	pub makedepends: Vec<Dependency>,
	#[serde(default)]
	pub checkdepends: Vec<Dependency>,
}

#[cfg(test)]
mod test {
	use super::*;
	use assert2::{assert, let_assert};
	use crate::version::Version;

	const PACKAGE_DESC: &[u8] = include_bytes!("../../tests/database-package/desc");
	const PACKAGE_DEPENDS: &[u8] = include_bytes!("../../tests/database-package/depends");

	#[test]
	fn test_parse_package_desc() {
		let_assert!(Ok(parsed) = from_bytes::<DatabasePackageDesc>(PACKAGE_DESC));
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
			Dependency::unconstrained("crda: to set the correct wireless channels of your country"),
		]);
		assert!(parsed.makedepends == vec![]);
		assert!(parsed.checkdepends == vec![]);
	}
}
