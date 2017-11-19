use std::cmp::Ordering;

use util::ConsumableStr;

struct VersionString<'a>(&'a str);

impl<'a> From<&'a str> for VersionString<'a> { fn from(s: &'a str) -> VersionString<'a> {VersionString(s)} }
impl<'a> Into<&'a str> for VersionString<'a> { fn into(self) -> &'a str {self.0} }

impl<'a> PartialOrd for VersionString<'a> { fn partial_cmp(&self, other: &VersionString<'a>) -> Option<Ordering> { Some(self.cmp(other)) } }
impl<'a> Ord        for VersionString<'a> { fn cmp(&self, other: &VersionString<'a>) -> Ordering { compare_version_string(self.0, other.0) } }
impl<'a> PartialEq  for VersionString<'a> { fn eq(&self, other: &VersionString<'a>) -> bool { self.cmp(other) == Ordering::Equal } }
impl<'a> Eq         for VersionString<'a> {}

#[derive(PartialEq,PartialOrd,Eq,Ord)]
struct VersionParts<'a> {
	epoch: Option<&'a str>,
	pkgver: VersionString<'a>,
	pkgrel: Option<VersionString<'a>>,
}

fn split_epoch(v: &str) -> (Option<&str>, &str) {
	v.partition(':').map_or_else(|| (None, v), |(epoch, _, rest)| (Some(epoch), rest))
}

fn split_pkgrel(v: &str) -> (&str, Option<&str>) {
	v.rpartition('-').map_or_else(|| (v, None), |(rest, _, pkgrel)| (rest, Some(pkgrel)))
}

fn split_parts(version: &str) -> VersionParts {
	let (epoch,  version) = split_epoch(version);
	let (pkgver, pkgrel)  = split_pkgrel(version);
	let pkgver = pkgver.into();
	let pkgrel = pkgrel.map(|x| x.into());
	VersionParts{epoch, pkgver, pkgrel}
}

pub fn compare(a: &str, b: &str) -> Ordering {
	split_parts(a).cmp(&split_parts(b))
}

pub fn compare_version_string(a: &str, b: &str) -> Ordering {
	while !a.is_empty() && !b.is_empty() {

	}

	return a.cmp(b);
}
