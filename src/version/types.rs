use std::cmp::Ordering;

use super::compare::compare_version_string;
use super::parse::split_parts;

#[derive(Debug)]
pub struct VersionStr<'a>(&'a str);

#[derive(Debug)]
pub struct VersionString(String);

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

impl Ord        for VersionString { fn         cmp(&self, other: &VersionString) -> Ordering { compare_version_string(self.0.as_ref(), other.0.as_ref()) }}
impl PartialOrd for VersionString { fn partial_cmp(&self, other: &VersionString) -> Option<Ordering> { Some(self.cmp(other)) }}
impl PartialEq  for VersionString { fn          eq(&self, other: &VersionString) -> bool { self.cmp(other) == Ordering::Equal }}
impl Eq         for VersionString {}

impl<'a> Ord        for VersionStr<'a> { fn         cmp(&self, other: &VersionStr<'a>) -> Ordering { compare_version_string(self.0, other.0) }}
impl<'a> PartialOrd for VersionStr<'a> { fn partial_cmp(&self, other: &VersionStr<'a>) -> Option<Ordering> { Some(self.cmp(other)) }}
impl<'a> PartialEq  for VersionStr<'a> { fn          eq(&self, other: &VersionStr<'a>) -> bool { self.cmp(other) == Ordering::Equal }}
impl<'a> Eq         for VersionStr<'a> {}

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
