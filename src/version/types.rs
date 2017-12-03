use super::compare::compare_version_string;
use super::parse::split_parts;

use std;

#[derive(Debug)]
pub struct VersionStr<'a>(&'a str);

#[derive(Debug)]
pub struct VersionString(String);

impl<'a> VersionStr<'a> { pub fn new<T: Into<&'a str>>(s: T) -> VersionStr<'a> { VersionStr   (s.into()) }}
impl     VersionString  { pub fn new<T: Into<String >>(s: T) -> VersionString  { VersionString(s.into()) }}

impl_ord!(    VersionString;  self, other => { compare_version_string(self.0.as_ref(), other.0.as_ref()) });
impl_ord!('a; VersionStr<'a>; self, other => { compare_version_string(self.0, other.0) });

/// A package version with epoch, pkgver and pkgrel.
#[derive(PartialEq,PartialOrd,Eq,Ord,Debug)]
pub struct Version {
	pub epoch: i32,
	pub pkgver: VersionString,
	pub pkgrel: Option<VersionString>,
}

/// A view into a string splitting it into version parts.
#[derive(PartialEq,PartialOrd,Eq,Ord,Debug)]
pub struct VersionParts<'a> {
	pub epoch: i32,
	pub pkgver: VersionStr<'a>,
	pub pkgrel: Option<VersionStr<'a>>,
}

// Conversions to/from VersionString and string.
impl From<String> for VersionString { fn from(s: String) -> VersionString {VersionString(s)}}
impl Into<String> for VersionString { fn into(self) -> String {self.0}}

// Conversions to/from VersionString and str.
impl<'a> From<&'a str> for VersionString { fn from(s: &'a str) -> VersionString {VersionString(s.into())}}
impl<'a> Into<&'a str> for &'a VersionString { fn into(self) -> &'a str {self.0.as_ref()}}

// Conversions to/from VersionString and VersionStr.
impl<'a> From<VersionStr<'a>> for VersionString { fn from(s: VersionStr<'a>) -> VersionString {VersionString(s.0.into())}}
impl<'a> Into<VersionStr<'a>> for &'a VersionString { fn into(self) -> VersionStr<'a> {VersionStr(self.0.as_ref())}}

// Conversions to/from VersionStr and str.
impl<'a> From<&'a str> for VersionStr<'a> { fn from(s: &'a str) -> VersionStr<'a> {VersionStr(s)}}
impl<'a> Into<&'a str> for VersionStr<'a> { fn into(self) -> &'a str {self.0}}

// Conversion from VersionParts to Version.
impl<'a> Into<Version> for VersionParts<'a> { fn into(self) -> Version {
	Version{
		epoch: self.epoch,
		pkgver: self.pkgver.into(),
		pkgrel: self.pkgrel.map(|x| x.into()),
	}
}}

// Conversion from Version to VersionParts
impl<'a> Into<VersionParts<'a>> for &'a Version { fn into(self) -> VersionParts<'a> {
	VersionParts {
		epoch: self.epoch,
		pkgver: (&self.pkgver).into(),
		pkgrel: self.pkgrel.as_ref().map(|x| x.into()),
	}
}}

impl Version {
	pub fn from_string(s: String) -> Version {
		split_parts(s.as_ref()).into()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_compare_version_str() {
		assert!(VersionStr::new("0") <  VersionStr::new("1"));
		assert!(VersionStr::new("0") <= VersionStr::new("1"));
		assert!(VersionStr::new("0") != VersionStr::new("1"));
		assert!(VersionStr::new("2") >  VersionStr::new("1"));
		assert!(VersionStr::new("2") >= VersionStr::new("1"));
		assert!(VersionStr::new("2") != VersionStr::new("1"));
		assert!(VersionStr::new("1") == VersionStr::new("1"));
	}

	#[test]
	fn test_compare_version_string() {
		assert!(VersionString::new("0") <  VersionString::new("1"));
		assert!(VersionString::new("0") <= VersionString::new("1"));
		assert!(VersionString::new("0") != VersionString::new("1"));
		assert!(VersionString::new("2") >  VersionString::new("1"));
		assert!(VersionString::new("2") >= VersionString::new("1"));
		assert!(VersionString::new("2") != VersionString::new("1"));
		assert!(VersionString::new("1") == VersionString::new("1"));
	}
}
