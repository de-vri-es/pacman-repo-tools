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
use std::collections::btree_map::Entry;
use std::collections::BTreeMap as Map;
use std::ffi::OsStr;
use std::io::Error as IoError;
use std::path::{Path, PathBuf};

use walkdir::{DirEntry, WalkDir};

use crate::error::ParseError;
use crate::package::{Package, PartialPackage};
use crate::parse::{parse_depends, parse_provides, partition};

use slice_tracker::{FileTracker, SliceTracker, Source};

type SourceTracker<'a> = SliceTracker<String, Source<str>>;
type ParseResult<'a, T> = std::result::Result<T, ParseError<'a>>;
type DbResult<'a, T> = std::result::Result<T, ReadDbError<'a>>;

/// An error that can occur while reading a folder structure with .SRCINFO files.
#[derive(Debug)]
pub enum ReadDbError<'a> {
	/// An error occured while crawling the filesystem.
	WalkError(walkdir::Error),

	/// An error occured while reading a .SRCINFO file.
	IoError(PathBuf, IoError),

	/// An error occured while parsing a .SRCINFO file.
	ParseError(PathBuf, ParseError<'a>),
}

impl<'a> std::fmt::Display for ReadDbError<'a> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			ReadDbError::WalkError(e) => write!(f, "{}", e),
			ReadDbError::IoError(_, e) => write!(f, "{}", e),
			ReadDbError::ParseError(_, e) => write!(f, "{}", e),
		}
	}
}

/// Iterate over key,value pairs in an INFO blob.
///
/// INFO blobs consist of 'key = value' lines.
/// All whitespace around keys or values is removed.
///
/// Empty lines are discarded.
pub fn iterate_info<'a>(blob: &'a str) -> impl Iterator<Item = ParseResult<'a, (&'a str, &'a str)>> {
	blob.split('\n').filter_map(move |line| {
		let line = line.trim();
		if line.is_empty() {
			return None;
		}
		let (key, value) = match partition(line, '=') {
			Some(x) => x,
			None => return Some(Err(ParseError::with_token(line, "expected 'key = value' or empty line"))),
		};
		Some(Ok((key.trim(), value.trim())))
	})
}

/// Set the value of an Option, or give an error if it is already set.
fn set_once<'a, T>(option: &mut Option<T>, value: T, key: &'a str) -> ParseResult<'a, ()> {
	if option.is_some() {
		Err(ParseError::with_token(key, format!("duplicate key: {}", key)))
	} else {
		*option = Some(value);
		Ok(())
	}
}

/// Insert a value into a map, but give an error if a conflicting entry already exists.
///
/// If the key already exists but the value is the same as the one being inserted,
/// the insertion is ignored and no error is returned.
fn insert_map<'a, V: Eq>(map: &mut Map<&'a str, V>, map_name: &str, entry: (&'a str, impl Into<V>)) -> ParseResult<'a, ()> {
	let (key, value) = entry;
	match map.entry(key.into()) {
		Entry::Vacant(x) => {
			x.insert(value.into());
			Ok(())
		},
		Entry::Occupied(x) => {
			if x.get() == &value.into() {
				Ok(())
			} else {
				Err(ParseError::with_token(
					key,
					format!("duplicate {} with different value: {}", map_name, key),
				))
			}
		},
	}
}

/// Parse one line of data from a .SRCINFO file.
///
/// This function returns true once the current package is done.
/// Data belonging to the next package is not consumed from the iterator.
fn parse_data_line<'a, I>(data_iterator: &mut std::iter::Peekable<I>, package: &mut PartialPackage<'a>) -> ParseResult<'a, bool>
where
	I: 'a + Iterator<Item = ParseResult<'a, (&'a str, &'a str)>>,
{
	let (key, value) = match data_iterator.peek() {
		None => return Ok(true),
		Some(&ref x) => (*x).clone()?,
	};

	match key {
		"pkgname" => return Ok(true),
		"epoch" => {
			let value = value
				.parse()
				.map_err(|x| ParseError::with_token(value, format!("invalid {}: {}", key, x)))?;
			set_once(&mut package.epoch, value, key)?;
		},
		"pkgver" => set_once(&mut package.pkgver, value, key)?,
		"pkgrel" => set_once(&mut package.pkgrel, value, key)?,
		"url" => set_once(&mut package.url, value.into(), key)?,
		"pkgdesc" => set_once(&mut package.description, value.into(), key)?,

		"license" => package.licenses.get_or_insert(Default::default()).push(value.into()),
		"groups" => package.groups.get_or_insert(Default::default()).push(value.into()),
		"backup" => package.backup.get_or_insert(Default::default()).push(value.into()),

		"provides" => insert_map(package.provides.get_or_insert(Default::default()), key, parse_provides(value))?,
		"conflicts" => insert_map(package.conflicts.get_or_insert(Default::default()), key, parse_depends(value))?,
		"replaces" => insert_map(package.replaces.get_or_insert(Default::default()), key, parse_depends(value))?,

		"depends" => insert_map(package.depends.get_or_insert(Default::default()), key, parse_depends(value))?,
		"optdepends" => insert_map(package.opt_depends.get_or_insert(Default::default()), key, parse_depends(value))?,
		"makedepends" => insert_map(package.make_depends.get_or_insert(Default::default()), key, parse_depends(value))?,
		"checkdepends" => insert_map(package.check_depends.get_or_insert(Default::default()), key, parse_depends(value))?,
		_ => (), // ignore unknown keys
	}

	data_iterator.next();
	Ok(false)
}

/// Parse all data belonging to one package.
fn parse_data_lines<'a, I>(data_iterator: &mut std::iter::Peekable<I>, mut package: PartialPackage<'a>) -> ParseResult<'a, PartialPackage<'a>>
where
	I: 'a + Iterator<Item = ParseResult<'a, (&'a str, &'a str)>>,
{
	while !parse_data_line(data_iterator, &mut package)? {}
	Ok(package)
}

/// Parse all pkgbase data not belonging to any specific package.
fn parse_base<'a, I>(data_iterator: &mut std::iter::Peekable<I>) -> ParseResult<'a, PartialPackage<'a>>
where
	I: 'a + Iterator<Item = ParseResult<'a, (&'a str, &'a str)>>,
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

/// Parse the next package from an iterator over the lines of a .SRCINFO file.
fn parse_package<'a, I>(data_iterator: &mut std::iter::Peekable<I>, base: &PartialPackage<'a>) -> Option<ParseResult<'a, PartialPackage<'a>>>
where
	I: 'a + Iterator<Item = ParseResult<'a, (&'a str, &'a str)>>,
{
	let (key, pkgname) = match data_iterator.next() {
		None => return None,
		Some(Err(x)) => return Some(Err(x)),
		Some(Ok(x)) => x,
	};

	if key != "pkgname" {
		panic!("logic error: next item in iterator had to be pkgname");
	}

	let mut package = PartialPackage::default();
	package.pkgname = Some(pkgname);
	package.epoch = base.epoch;
	package.pkgver = base.pkgver;
	package.pkgrel = base.pkgrel;

	let mut package = match parse_data_lines(data_iterator, package) {
		Err(x) => return Some(Err(x)),
		Ok(x) => x,
	};

	package.add_base(base);
	Some(Ok(package))
}

/// Iterator over all packages in a .SRCINFO file.
struct PackageIterator<'a, DataIterator: Iterator<Item = ParseResult<'a, (&'a str, &'a str)>>> {
	data_iterator: std::iter::Peekable<DataIterator>,
	base: PartialPackage<'a>,
	base_done: bool,
}

impl<'a, DataIterator> PackageIterator<'a, DataIterator>
where
	DataIterator: 'a + Iterator<Item = ParseResult<'a, (&'a str, &'a str)>>,
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
where
	DataIterator: 'a + Iterator<Item = ParseResult<'a, (&'a str, &'a str)>>,
{
	type Item = ParseResult<'a, Package<'a>>;

	fn next(&mut self) -> Option<ParseResult<'a, Package<'a>>> {
		// Make sure the pkgbase is parsed.
		if !std::mem::replace(&mut self.base_done, true) {
			self.base = match parse_base(&mut self.data_iterator) {
				Err(x) => return Some(Err(x)),
				Ok(x) => x,
			}
		}

		let package = parse_package(&mut self.data_iterator, &self.base)?;
		let package = package.and_then(|x| x.into_package().map_err(ParseError::no_token));
		Some(package)
	}
}

/// Parse all packages from a .SRCINFO file.
pub fn parse_srcinfo_blob<'a>(blob: &'a str) -> impl Iterator<Item = ParseResult<'a, Package<'a>>> {
	PackageIterator::new(iterate_info(blob))
}

/// Find all .SRCINFO files in a given directory.
pub fn walk_srcinfo_files<P: ?Sized + AsRef<Path>>(directory: &P) -> impl Iterator<Item = walkdir::Result<DirEntry>> {
	WalkDir::new(directory).into_iter().filter(|x| {
		if let &Ok(ref entry) = x {
			entry.file_type().is_file() && entry.path().file_name() == Some(OsStr::new(".SRCINFO"))
		} else {
			true
		}
	})
}

/// Parse all .SRCINFO files under the given directory.
///
/// This will recursively look in subdirectories.
pub fn parse_srcinfo_dir<'a, P>(tracker: &'a SourceTracker, directory: &P) -> DbResult<'a, Map<&'a str, (PathBuf, Package<'a>)>>
where
	P: ?Sized + AsRef<Path>,
{
	let mut result = Map::default();
	for entry in walk_srcinfo_files(directory) {
		let entry = entry.map_err(ReadDbError::WalkError)?;
		let path = entry.path();
		let data = tracker.insert_file(path).map_err(|x| ReadDbError::IoError(path.into(), x))?;
		for package in parse_srcinfo_blob(&data) {
			match package {
				Err(x) => return Err(ReadDbError::ParseError(path.into(), x)),
				Ok(package) => match result.entry(package.pkgname) {
					Entry::Occupied(_) => {
						return Err(ReadDbError::ParseError(
							path.into(),
							ParseError::no_token(format!("duplicate package name: {}", package.pkgname)),
						))
					},
					Entry::Vacant(x) => x.insert((path.into(), package)),
				},
			};
		}
	}
	Ok(result)
}

#[cfg(test)]
mod test {
	use super::*;
	use assert2::assert;

	fn iterate_info_vec(blob: &str) -> ParseResult<Vec<(&str, &str)>> {
		iterate_info(blob).collect()
	}

	#[test]
	fn test_simple() {
		let blob = ["a=b", "c=d"].join("\n");
		assert!(iterate_info_vec(&blob) == Ok(vec![("a", "b"), ("c", "d"),]))
	}

	#[test]
	fn spaces_are_stripped() {
		assert!(iterate_info_vec(" a   =    b  ") == Ok(vec![("a", "b")]))
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
