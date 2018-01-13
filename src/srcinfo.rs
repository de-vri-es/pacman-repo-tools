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

use std;
use std::collections::BTreeMap as Map;
use std::collections::btree_map::Entry as Entry;
use std::ffi::OsStr;
use std::io::Error as IoError;
use std::path::{Path,PathBuf};

extern crate walkdir;
use self::walkdir::{DirEntry, WalkDir};

use error::ParseError;
use package::{Package, PartialPackage};
use parse::{parse_depends, parse_provides};
use util::{ConsumableStr, DefaultOption};

use slice_tracker::{SliceTracker, SourceLocation, FileSliceTracker};

type SourceTracker<'a> = SliceTracker<'a, str, SourceLocation<'a, str>>;
type Result<'a, T>   = std::result::Result<T, ParseError<'a>>;
type DbResult<'a, T> = std::result::Result<T, ReadDbError<'a>>;

#[derive(Debug)]
pub enum ReadDbError<'a> {
	WalkError(walkdir::Error),
	IoError(PathBuf, IoError),
	ParseError(PathBuf, ParseError<'a>),
}

impl<'a> ReadDbError<'a> {
	pub fn inner(&self) -> &std::error::Error {
		match self {
			&ReadDbError::WalkError(ref err) => err,
			&ReadDbError::IoError(_, ref err) => err,
			&ReadDbError::ParseError(_, ref err) => err,
		}
	}
}

impl<'a> std::fmt::Display for ReadDbError<'a> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { std::fmt::Display::fmt(self.inner(), f) }
}

impl<'a> std::error::Error for ReadDbError<'a> {
	fn description(&self) -> &str                       { self.inner().description() }
	fn cause(&self)       -> Option<&std::error::Error> { self.inner().cause() }
}

/// Iterate over key,value pairs in an INFO blob.
///
/// INFO blobs consist of 'key = value' lines.
/// All whitespace around keys or values is removed.
///
/// Empty lines are discarded.
pub fn iterate_info<'a>(blob: &'a str) -> impl Iterator<Item = Result<'a, (&'a str, &'a str)>> {
	blob.split('\n').filter_map(move |line| {
		let line = line.trim();
		if line.is_empty() {
			return None;
		}
		let result = line.partition('=').map(|(key, _, value)| (key.trim(), value.trim()));
		let result = result.ok_or_else(|| ParseError::with_token(line, "expected 'key = value' or empty line"));
		Some(result)
	})
}

fn set_once_err<'a, T>(option: &mut Option<T>, value: T, key: &'a str) -> Result<'a, ()> {
	if option.is_some() {
		Err(ParseError::with_token(key, format!("duplicate key: {}", key)))
	} else {
		*option = Some(value);
		Ok(())
	}
}

fn insert_err<'a, V: Eq>(map: &mut Map<&'a str, V>, map_name: &str, entry: (&'a str, impl Into<V>)) -> Result<'a, ()> {
	let (key, value) = entry;
	match map.entry(key.into()) {
		Entry::Vacant(x)   => { x.insert(value.into()); Ok(()) },
		Entry::Occupied(x) => {
			if x.get() == &value.into() {
				Ok(())
			} else {
				Err(ParseError::with_token(key, format!("duplicate {} with different value: {}", map_name, key)))
			}
		}
	}
}

pub struct PackageIterator<'a, DataIterator: Iterator<Item = Result<'a, (&'a str, &'a str)>>> {
	data_iterator: std::iter::Peekable<DataIterator>,
	base: PartialPackage<'a>,
	base_done: bool,
}

fn parse_data_line<'a, I>(data_iterator: &mut std::iter::Peekable<I>, package: &mut PartialPackage<'a>) -> Result<'a, bool>
	where I: 'a + Iterator<Item = Result<'a, (&'a str, &'a str)>>
{
	let (key, value) = match data_iterator.peek() {
		None         => return Ok(true),
		Some(&ref x) => (*x).clone()?,
	};

	match key {
		"pkgname"     => return Ok(true),
		"epoch"       => set_once_err(&mut package.epoch,  value.parse().map_err(|x| ParseError::with_token(value, format!("invalid {}: {}", key, x)))?, key)?,
		"pkgver"      => set_once_err(&mut package.pkgver, value, key)?,
		"pkgrel"      => set_once_err(&mut package.pkgrel, value, key)?,
		"url"         => set_once_err(&mut package.url, value.into(), key)?,
		"description" => set_once_err(&mut package.description, value.into(), key)?,

		"licenses"      => package.licenses.get_or_default().push(value.into()),
		"groups"        => package.groups.get_or_default().push(value.into()),
		"backup"        => package.backup.get_or_default().push(value.into()),

		"provides"      => insert_err(package.provides.get_or_default(),  key, parse_provides(value))?,
		"conflicts"     => insert_err(package.conflicts.get_or_default(), key, parse_depends(value))?,
		"replaces"      => insert_err(package.replaces.get_or_default(),  key, parse_depends(value))?,

		"depends"       => insert_err(package.depends.get_or_default(),       key, parse_depends(value))?,
		"opt_depends"   => insert_err(package.opt_depends.get_or_default(),   key, parse_depends(value))?,
		"make_depends"  => insert_err(package.make_depends.get_or_default(),  key, parse_depends(value))?,
		"check_depends" => insert_err(package.check_depends.get_or_default(), key, parse_depends(value))?,
		_               => {}, // ignore unknown keys
	}

	data_iterator.next();
	Ok(false)
}

fn parse_data_lines<'a, I>(data_iterator: &mut std::iter::Peekable<I>, mut package: PartialPackage<'a>) -> Result<'a, PartialPackage<'a>>
	where I: 'a + Iterator<Item = Result<'a, (&'a str, &'a str)>>
{
	while !parse_data_line(data_iterator, &mut package)? {}
	Ok(package)
}

fn parse_base<'a, I>(data_iterator: &mut std::iter::Peekable<I>) -> Result<'a, PartialPackage<'a>>
	where I: 'a + Iterator<Item = Result<'a, (&'a str, &'a str)>>
{
	let base = parse_data_lines(data_iterator, PartialPackage::default())?;
	if base.pkgver.is_none() {
		Err(ParseError::no_token("missing pkgver in pkgbase"))
	} else if base.pkgrel.is_none() {
		Err(ParseError::no_token("missing pkgrel in pkgbase"))
	} else {
		Ok(base)
	}
}

fn parse_package<'a, I>(data_iterator: &mut std::iter::Peekable<I>, base: &PartialPackage<'a>) -> Option<Result<'a, PartialPackage<'a>>>
	where I: 'a + Iterator<Item = Result<'a, (&'a str, &'a str)>>
{
	let (key, pkgname) = match data_iterator.next() {
		None         => return None,
		Some(Err(x)) => return Some(Err(x)),
		Some(Ok(x))  => x,
	};

	if key != "pkgname" { panic!("logic error: next item in iterator had to be pkgname"); }

	let mut package = PartialPackage::default();
	package.pkgname = Some(pkgname);
	package.epoch   = base.epoch;
	package.pkgver  = base.pkgver;
	package.pkgrel  = base.pkgrel;

	let mut package = match parse_data_lines(data_iterator, package) {
		Err(x) => return Some(Err(x)),
		Ok(x)  => x,
	};

	package.add_base(base);
	Some(Ok(package))
}

impl<'a, DataIterator> PackageIterator<'a, DataIterator>
	where DataIterator: 'a + Iterator<Item = Result<'a, (&'a str, &'a str)>>
{
	pub fn new(data_iterator: DataIterator) -> Self {
		PackageIterator {
			data_iterator: data_iterator.peekable(),
			base: PartialPackage::default(),
			base_done: false,
		}
	}

}

impl<'a, DataIterator> Iterator for PackageIterator<'a, DataIterator>
	where DataIterator: 'a + Iterator<Item = Result<'a, (&'a str, &'a str)>>
{
	type Item = Result<'a, Package<'a>>;

	fn next(&mut self) -> Option<Result<'a, Package<'a>>> {
		// Make sure the pkgbase is parsed.
		if !std::mem::replace(&mut self.base_done, true) {
			self.base = match parse_base(&mut self.data_iterator) {
				Err(x) => return Some(Err(x)),
				Ok(x)  => x,
			}
		}

		let package = parse_package(&mut self.data_iterator, &self.base)?;
		let package = package.and_then(|x| x.into_package().map_err(ParseError::no_token));
		Some(package)
	}
}

pub fn parse_srcinfo_blob<'a>(blob: &'a str) -> PackageIterator<'a, impl Iterator<Item = Result<'a, (&'a str, &'a str)>>> {
	PackageIterator::new(iterate_info(blob))
}

/// Find all .SRCINFO files in a given directory.
pub fn walk_srcinfo_files<P: ?Sized + AsRef<Path>>(directory: &P) -> impl Iterator<Item = walkdir::Result<DirEntry>> {
	WalkDir::new(directory).into_iter().filter(|x|
		if let &Ok(ref entry) = x {
			entry.file_type().is_file() && entry.path().file_name() == Some(OsStr::new(".SRCINFO"))
		} else {
			true
		}
	)
}

pub fn parse_srcinfo_dir<'a, P>(tracker: &'a SourceTracker, directory: &P) -> DbResult<'a, Map<&'a str, Package<'a>>> where P: ?Sized + AsRef<Path> {
	let mut result = Map::default();
	for entry in walk_srcinfo_files(directory) {
		let entry    = entry.map_err(ReadDbError::WalkError)?;
		let path     = entry.path();
		let data     = tracker.insert_file(path).map_err(|x| ReadDbError::IoError(path.into(), x))?;
		for package in parse_srcinfo_blob(&data) {
			match package {
				Err(x) => return Err(ReadDbError::ParseError(path.into(), x)),
				Ok(package) => match result.entry(package.pkgname) {
					Entry::Occupied(_) => return Err(ReadDbError::ParseError(path.into(), ParseError::no_token(format!("duplicate package name: {}", package.pkgname)))),
					Entry::Vacant(x)   => x.insert(package),
				}
			};
		}
	}
	Ok(result)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_simple() {
		let blob = ["a=b", "c=d"].join("\n");
		assert_seq!(iterate_info(&blob), [
			Ok(("a", "b")),
			Ok(("c", "d")),
		])
	}

	#[test]
	fn spaces_are_stripped() {
		assert_seq!(iterate_info(" a   =    b  "), [Ok(("a", "b"))])
	}

	#[test]
	fn garbage_gives_error() {
		let blob = ["ab", "a = b"].join("\n");
		let mut iterator = iterate_info(&blob);
		assert!(iterator.next().unwrap().is_err());
		assert_eq!(iterator.next(), Some(Ok(("a", "b"))));
		assert_eq!(iterator.next(), None);
	}

}
