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
use std::fs::File;
use std::io::{Error as IoError, Read};
use std::path::{Path,PathBuf};

extern crate walkdir;
use self::walkdir::{DirEntry, WalkDir};

use error::ParseError;
use package::{Package, PartialPackage};
use parse::{parse_depends, parse_provides};
use util::{ConsumableStr, DefaultOption};
use version::{Version};

type Result<T>   = std::result::Result<T, ParseError>;
type DbResult<T> = std::result::Result<T, ReadDbError>;

#[derive(Debug)]
pub enum ReadDbError {
	WalkError(walkdir::Error),
	IoError(PathBuf, IoError),
	ParseError(PathBuf, ParseError),
}

impl ReadDbError {
	pub fn inner(&self) -> &std::error::Error {
		match self {
			&ReadDbError::WalkError(ref err) => err,
			&ReadDbError::IoError(_, ref err) => err,
			&ReadDbError::ParseError(_, ref err) => err,
		}
	}
}

impl std::fmt::Display for ReadDbError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { std::fmt::Display::fmt(self.inner(), f) }
}

impl std::error::Error for ReadDbError {
	fn description(&self) -> &str                       { self.inner().description() }
	fn cause(&self)       -> Option<&std::error::Error> { self.inner().cause() }
}

/// Iterate over key,value pairs in an INFO blob.
///
/// INFO blobs consist of 'key = value' lines.
/// All whitespace around keys or values is removed.
///
/// Empty lines are discarded.
pub fn iterate_info<'a>(blob: &'a str) -> impl Iterator<Item = Result<(&'a str, &'a str)>> {
	blob.split('\n').filter_map(move |line| {
		let line = line.trim();
		if line.is_empty() {
			return None;
		}
		let result = line.partition('=').map(|(key, _, value)| (key.trim(), value.trim()));
		let result = result.ok_or_else(|| ParseError::for_token(blob, line, "expected 'key = value' or empty line"));
		Some(result)
	})
}

fn set_once_err<T>(option: &mut Option<T>, value: T, key: &str) -> Result<()> {
	if option.is_some() {
		Err(ParseError{message: format!("duplicate key: {}", key), token_start: 0, token_end: 0})
	} else {
		*option = Some(value);
		Ok(())
	}
}

fn insert_err<V: Eq, IV: Into<V>>(map: &mut Map<String,V>, map_name: &str, entry: (&str, IV)) -> Result<()> {
	let (key, value) = entry;
	match map.entry(key.into()) {
		Entry::Vacant(x)   => { x.insert(value.into()); Ok(()) },
		Entry::Occupied(x) => {
			if x.get() == &value.into() {
				Ok(())
			} else {
				Err(ParseError{message: format!("duplicate {} with different value: {}", map_name, key), token_start: 0, token_end: 0})
			}
		}
	}
}

pub struct PackageIterator<DataIterator: Iterator> {
	data_iterator: std::iter::Peekable<DataIterator>,
	base: PartialPackage,
	base_done: bool,
}

fn parse_data_line<'a, I>(data_iterator: &mut std::iter::Peekable<I>, package: &mut PartialPackage) -> Result<bool>
	where I: Iterator<Item = Result<(&'a str, &'a str)>>
{
	let (key, value) = match data_iterator.peek() {
		None         => return Ok(true),
		Some(&ref x) => (*x).clone()?,
	};

	match key {
		"pkgname"     => return Ok(true),
		"pkgver"      => set_once_err(&mut package.version, Version::from_str(value).into(), key)?,
		"url"         => set_once_err(&mut package.url, value.into(), key)?,
		"description" => set_once_err(&mut package.description, value.into(), key)?,

		"licenses"      => package.licenses.get_or_default().push(value.into()),
		"groups"        => package.groups.get_or_default().push(value.into()),
		"backup"        => package.backup.get_or_default().push(value.into()),

		"provides"      => insert_err(package.provides.get_or_default(),  "provides",  parse_provides(value))?,
		"conflicts"     => insert_err(package.conflicts.get_or_default(), "conflicts", parse_depends(value))?,
		"replaces"      => insert_err(package.replaces.get_or_default(),  "replaces",  parse_depends(value))?,

		"depends"       => insert_err(package.depends.get_or_default(),       "depends",       parse_depends(value))?,
		"opt_depends"   => insert_err(package.opt_depends.get_or_default(),   "opt_depends",   parse_depends(value))?,
		"make_depends"  => insert_err(package.make_depends.get_or_default(),  "make_depends",  parse_depends(value))?,
		"check_depends" => insert_err(package.check_depends.get_or_default(), "check_depends", parse_depends(value))?,
		_               => {}, // ignore unknown keys
	}

	data_iterator.next();
	Ok(false)
}

fn parse_data_lines<'a, I>(data_iterator: &mut std::iter::Peekable<I>, mut package: PartialPackage) -> Result<PartialPackage>
	where I: Iterator<Item = Result<(&'a str, &'a str)>>
{
	while !parse_data_line(data_iterator, &mut package)? {}
	Ok(package)
}

impl<'a, DataIterator> PackageIterator<DataIterator>
	where DataIterator: Iterator<Item = Result<(&'a str, &'a str)>>
{
	pub fn new(data_iterator: DataIterator) -> Self {
		PackageIterator {
			data_iterator: data_iterator.peekable(),
			base: PartialPackage::default(),
			base_done: false,
		}
	}

	fn parse_base(&mut self) -> Result<()> {
		while !parse_data_line(&mut self.data_iterator, &mut self.base)? {}
		if self.base.version.is_none() {
			Err(ParseError::no_token("missing pkgver in base"))
		} else {
			Ok(())
		}
	}

	fn parse_package(&mut self) -> Option<Result<PartialPackage>> {
		let (key, name) = match self.data_iterator.next() {
			None         => return None,
			Some(Err(x)) => return Some(Err(x)),
			Some(Ok(x))  => x,
		};

		if key != "pkgname" { panic!("logic error: next item in iterator had to be pkgname"); }

		let mut package = PartialPackage::default();
		package.name    = Some(String::from(name));
		package.version = self.base.version.clone();

		Some(parse_data_lines(&mut self.data_iterator, package))
	}

}

impl<'a, DataIterator> Iterator for PackageIterator<DataIterator>
	where DataIterator: Iterator<Item = Result<(&'a str, &'a str)>>
{
	type Item = Result<Package>;

	fn next(&mut self) -> Option<Result<Package>> {
		// Make sure the pkgbase is parsed.
		if !std::mem::replace(&mut self.base_done, true) {
			if let Err(x) = self.parse_base() { return Some(Err(x)) }
		}

		Some(match self.parse_package()? {
			Err(x) => Err(x),
			Ok(mut package) => {
				package.add_base(&self.base);
				package.into_package().map_err(ParseError::no_token)
			},
		})
	}
}

fn read_text_file<P: ?Sized + AsRef<Path>>(path: &P) -> std::io::Result<String> {
	let mut file = File::open(path)?;
	let mut data = String::new();
	file.read_to_string(&mut data)?;
	return Ok(data);
}

pub fn parse_srcinfo_blob<'a>(blob: &'a str) -> PackageIterator<impl Iterator<Item = Result<(&'a str, &'a str)>>> {
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

pub fn parse_srcinfo_dir<P: ?Sized + AsRef<Path>>(directory: &P) -> DbResult<Map<String, Package>> {
	let mut result = Map::default();
	for entry in walk_srcinfo_files(directory) {
		let entry    = entry.map_err(ReadDbError::WalkError)?;
		let path     = entry.path();
		let data     = read_text_file(path).map_err(|x| ReadDbError::IoError(path.into(), x))?;
		for package in parse_srcinfo_blob(&data) {
			match package {
				Err(x) => return Err(ReadDbError::ParseError(path.into(), x)),
				Ok(package) => match result.entry(package.name.clone()) {
					Entry::Occupied(_) => return Err(ReadDbError::ParseError(path.into(), ParseError::no_token(format!("duplicate package name: {}", &package.name)))),
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
