// Copyright (c) 2017, Maarten de Vries
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// * Redistributions of source code must retain the above copyright notice, this
//   list of conditions and the following disclaimer.
//
// * Redistributions in binary form must reproduce the above copyright notice,
//   this list of conditions and the following disclaimer in the documentation
//   and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::cmp::Ordering;

use util::ConsumableStr;

struct VersionString<'a>(&'a str);

impl<'a> From<&'a str> for VersionString<'a> { fn from(s: &'a str) -> VersionString<'a> {VersionString(s)} }
impl<'a> Into<&'a str> for VersionString<'a> { fn into(self) -> &'a str {self.0} }

impl<'a> Ord        for VersionString<'a> { fn         cmp(&self, other: &VersionString<'a>) -> Ordering { compare_version_string(self.0, other.0) } }
impl<'a> PartialOrd for VersionString<'a> { fn partial_cmp(&self, other: &VersionString<'a>) -> Option<Ordering> { Some(self.cmp(other)) } }
impl<'a> PartialEq  for VersionString<'a> { fn          eq(&self, other: &VersionString<'a>) -> bool { self.cmp(other) == Ordering::Equal } }
impl<'a> Eq         for VersionString<'a> {}

#[derive(PartialEq,PartialOrd,Eq,Ord)]
struct VersionParts<'a> {
	epoch: i32,
	pkgver: VersionString<'a>,
	pkgrel: Option<VersionString<'a>>,
}

fn consume_epoch(v: &mut &str) -> Option<i32> {
	let mut a: &str = v;
	let epoch = a.consume_front_while(|x: char| x.is_digit(10));
	if a.consume_front_n(1) == Some(":") {
		*v = a;
		Some(if epoch.is_empty() {0} else {epoch.parse().unwrap()})
	} else {
		None
	}
}

fn consume_pkgrel<'a>(v: &mut &'a str) -> Option<&'a str> {
	v.rpartition('-').map(|(rest, _, pkgrel)| {
		*v = rest;
		pkgrel
	})
}

fn split_parts(version: &str) -> VersionParts {
	let mut version = version;
	let epoch  = consume_epoch(&mut version).unwrap_or(0);
	let pkgrel = consume_pkgrel(&mut version).map(|x| x.into());
	let pkgver = version.into();
	VersionParts{epoch, pkgver, pkgrel}
}

pub fn compare_package_version(a: &str, b: &str) -> Ordering {
	split_parts(a).cmp(&split_parts(b))
}

pub fn compare_version_string(a: &str, b: &str) -> Ordering {
	let mut a = a;
	let mut b = b;

	// Loop over the alphanumeric parts.
	while !a.is_empty() || !b.is_empty() {
		// Get the first alphanumeric component.
		let mut a_alnum = a.consume_front_while(|x: char| x.is_alphanumeric());
		let mut b_alnum = b.consume_front_while(|x: char| x.is_alphanumeric());

		// Loop over the numeric and alphabetical parts.
		while !a_alnum.is_empty() || !b_alnum.is_empty() {
			let a_num   = a_alnum.consume_front_while(|x: char| x.is_digit(10));
			let b_num   = b_alnum.consume_front_while(|x: char| x.is_digit(10));
			let a_alpha = a_alnum.consume_front_while(|x: char| x.is_alphabetic());
			let b_alpha = b_alnum.consume_front_while(|x: char| x.is_alphabetic());

			// Parse the numeric part.
			let a_num = if a_num.is_empty() { -1 } else { a_num.parse::<i32>().unwrap() };
			let b_num = if b_num.is_empty() { -1 } else { b_num.parse::<i32>().unwrap() };

			// If the numeric part is different, we're done.
			let num_ordering = a_num.cmp(&b_num);
			if num_ordering != Ordering::Equal { return num_ordering };

			// If the alphabetical part is different, we're done.
			// Note that an empty alphabetical part is "newer" than a non-empty alphabetical part.
			let alpha_ordering = (a_alpha.is_empty(), a_alpha).cmp(&(b_alpha.is_empty(), b_alpha));
			if alpha_ordering != Ordering::Equal { return alpha_ordering };
		}

		// Drop the non-alphanumeric separator.
		// If only one has a separator, it's a newer version.
		let a_sep = a.consume_front_while(|x: char| !x.is_alphanumeric());
		let b_sep = b.consume_front_while(|x: char| !x.is_alphanumeric());
		let ordering = (!a_sep.is_empty()).cmp(&!b_sep.is_empty());
		if ordering != Ordering::Equal { return ordering }
	}

	// If we get here the versions are equal.
	Ordering::Equal
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_consume_epoch() {
		{
			let mut a = "3:a";
			assert_eq!(consume_epoch(&mut a), Some(3));
			assert_eq!(a, "a");
		}
		{
			let mut a = "3:a:b";
			assert_eq!(consume_epoch(&mut a), Some(3));
			assert_eq!(a, "a:b");
		}
		{
			let mut a = "31:a";
			assert_eq!(consume_epoch(&mut a), Some(31));
			assert_eq!(a, "a");
		}
		{
			let mut a = "a";
			assert_eq!(consume_epoch(&mut a), None);
			assert_eq!(a, "a");
		}
		{
			let mut a = "abc";
			assert_eq!(consume_epoch(&mut a), None);
			assert_eq!(a, "abc");
		}
		{
			let mut a = "3a1:a";
			assert_eq!(consume_epoch(&mut a), None);
			assert_eq!(a, "3a1:a");
		}
	}

	#[test]
	fn test_consume_pkgrel() {
		{
			let mut a = "1-2";
			assert_eq!(consume_pkgrel(&mut a), Some("2"));
			assert_eq!(a, "1");
		}
		{
			let mut a = "1-2-3";
			assert_eq!(consume_pkgrel(&mut a), Some("3"));
			assert_eq!(a, "1-2");
		}
		{
			let mut a = "1.2abc-3.4def";
			assert_eq!(consume_pkgrel(&mut a), Some("3.4def"));
			assert_eq!(a, "1.2abc");
		}
		{
			let mut a = "123";
			assert_eq!(consume_pkgrel(&mut a), None);
			assert_eq!(a, "123");
		}
	}

	fn assert_compare_version_string(a: &str, b: &str, ordering: Ordering) {
		assert_eq!(compare_version_string(a, b), ordering, "comparing {:?} to {:?}", a, b);
		assert_eq!(compare_version_string(b, a), ordering.reverse(), "comparing {:?} to {:?}", a, b);
		assert_eq!(VersionString(a).cmp(&VersionString(b)), ordering, "comparing {:?} to {:?}", b, a);
		assert_eq!(VersionString(b).cmp(&VersionString(a)), ordering.reverse(), "comparing {:?} to {:?}", b, a);
	}

	#[test]
	fn test_compare_version_string() {
		// Simple cases.
		assert_compare_version_string("",  "",  Ordering::Equal);
		assert_compare_version_string("1", "0", Ordering::Greater);
		assert_compare_version_string("0", "1", Ordering::Less);

		// Examples from the man page for alphanumeric comparisons.
		assert_compare_version_string("1.0a"   , "1.0b"   , Ordering::Less);
		assert_compare_version_string("1.0b"   , "1.0beta", Ordering::Less);
		assert_compare_version_string("1.0beta", "1.0p"   , Ordering::Less);
		assert_compare_version_string("1.0p"   , "1.0pre" , Ordering::Less);
		assert_compare_version_string("1.0pre" , "1.0rc"  , Ordering::Less);
		assert_compare_version_string("1.0rc"  , "1.0"    , Ordering::Less);
		assert_compare_version_string("1.0"    , "1.0.a"  , Ordering::Less);
		assert_compare_version_string("1.0.a"  , "1.0.1"  , Ordering::Less);

		// Examples from the man page for numeric comparisons.
		assert_compare_version_string("1"    , "1.0"  , Ordering::Less);
		assert_compare_version_string("1.0"  , "1.1"  , Ordering::Less);
		assert_compare_version_string("1.1"  , "1.1.1", Ordering::Less);
		assert_compare_version_string("1.1.1", "1.2"  , Ordering::Less);
		assert_compare_version_string("1.2"  , "2.0"  , Ordering::Less);
		assert_compare_version_string("2.0"  , "3.0.0", Ordering::Less);

		// Extra numeric component makes the version greater.
		assert_compare_version_string("1.0rc", "1.0rc1", Ordering::Less);

		// Extra alphabetical component makes the version less.
		assert_compare_version_string("1a2b", "1a2", Ordering::Less);

		// Extra seperator makes the version greater.
		assert_compare_version_string("1", "1.", Ordering::Less);

		// Consecutive separators are folded into one separator.
		assert_compare_version_string("1." , "1..",  Ordering::Equal);
		assert_compare_version_string("1.2", "1..2", Ordering::Equal);

		// Empty components are newer than alphabetical components.
		assert_compare_version_string("1..a" , "1."    , Ordering::Less);
		assert_compare_version_string("1..a" , "1.."   , Ordering::Less);
		// Empty components are less than numeric components.
		assert_compare_version_string("1.."  , "1..1"  , Ordering::Less);
	}

	fn assert_compare_package_version(a: &str, b: &str, ordering: Ordering) {
		assert_eq!(compare_package_version(a, b), ordering, "comparing {:?} to {:?}", a, b);
		assert_eq!(compare_package_version(b, a), ordering.reverse(), "comparing {:?} to {:?}", b, a);
	}

	#[test]
	fn test_compare_package_version() {
		// Test simple cases.
		assert_compare_package_version("",  "" , Ordering::Equal);
		assert_compare_package_version("1", "0", Ordering::Greater);
		assert_compare_package_version("0", "1", Ordering::Less);

		// Test that pkgrel decides if pkgver is equal.
		assert_compare_package_version("0-1", "0",   Ordering::Greater);
		assert_compare_package_version("0-1", "0-0", Ordering::Greater);
		assert_compare_package_version("1-0", "0-1", Ordering::Greater);

		// Test that epoch is implicitly 0.
		assert_compare_package_version("0:",  "" , Ordering::Equal);
		assert_compare_package_version("0:1", "0", Ordering::Greater);
		assert_compare_package_version("0:0", "1", Ordering::Less);

		// Test that epoch trumps pkgver.
		assert_compare_package_version("1:1", "0", Ordering::Greater);
		assert_compare_package_version("1:0", "0", Ordering::Greater);
		assert_compare_package_version("1:0", "1", Ordering::Greater);

		// Test that epoch trumps pkgrel.
		assert_compare_package_version("0-1", "1:0",   Ordering::Less);
		assert_compare_package_version("0-1", "1:0-0", Ordering::Less);
		assert_compare_package_version("1-0", "1:0-1", Ordering::Less);
	}
}
