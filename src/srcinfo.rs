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

use std::collections::BTreeMap as Map;
use std::collections::btree_map::Entry as Entry;
use std::mem;
use std::error;
use std::ffi::OsStr;
use std::fmt;
use std::result;
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

type Result<T> = result::Result<T, ParseError>;

#[derive(Debug)]
pub enum ReadDbError {
	WalkError(walkdir::Error),
	IoError(PathBuf, IoError),
	ParseError(PathBuf, String, ParseError),
}

impl ReadDbError {
	pub fn inner(&self) -> &error::Error {
		match self {
			&ReadDbError::WalkError(ref err) => err,
			&ReadDbError::IoError(_, ref err) => err,
			&ReadDbError::ParseError(_, _, ref err) => err,
		}
	}
}

impl fmt::Display for ReadDbError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { fmt::Display::fmt(self.inner(), f) }
}

impl error::Error for ReadDbError {
	fn description(&self) -> &str                  { self.inner().description() }
	fn cause(&self)       -> Option<&error::Error> { self.inner().cause() }
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

pub struct PackageIterator<'a, DataIterator> {
	data_iterator: DataIterator,
	base_done: bool,
	base: PartialPackage,
	next_name: Option<&'a str>,
}

impl<'a, DataIterator> PackageIterator<'a, DataIterator>
	where DataIterator: Iterator<Item = Result<(&'a str, &'a str)>>
{
	pub fn new(data_iterator: DataIterator) -> Self {
		PackageIterator {
			data_iterator,
			base_done: false,
			base: PartialPackage::default(),
			next_name: None,
		}
	}

	fn parse_base(&mut self) -> Result<(PartialPackage, &'a str)> {
		let (package, next_name) = self.parse_package()?;
		if let Some(next_name) = next_name {
			Ok((package, next_name))
		} else {
			Err(ParseError{message: String::from("missing pkgname"), token_start: 0, token_end: 0})
		}
	}

	fn parse_package(&mut self) -> Result<(PartialPackage, Option<&'a str>)> {
		let mut package = PartialPackage::default();

		for entry in &mut self.data_iterator {
			let (key, value) = entry?;
			match key {
				"pkgname"     => return Ok((package, Some(value))),
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
		}

		Ok((package, None))
	}
}

impl<'a, DataIterator> Iterator for PackageIterator<'a, DataIterator>
	where DataIterator: Iterator<Item = Result<(&'a str, &'a str)>>
{
	type Item = Result<Package>;

	fn next(&mut self) -> Option<Result<Package>> {
		// Make sure the pkgbase is parsed.
		if !mem::replace(&mut self.base_done, true) {
			match self.parse_base() {
				Ok((pkgbase, next_name)) => {
					self.base      = pkgbase;
					self.next_name = Some(next_name);
				},
				Err(err) => return Some(Err(err)),
			}
		}

		// Parse the next package.
		if let Some(pkgname) = self.next_name {
			match self.parse_package() {
				Ok((mut package, next_name)) => {
					package.name = Some(String::from(pkgname));
					package.add_base(&self.base);
					self.next_name = next_name;
					Some(package.into_package().map_err(|msg| ParseError{message: msg, token_start: 0, token_end: 0}))
				},
				Err(err) => Some(Err(err)),
			}
		} else {
			None
		}
	}
}

pub fn parse_srcinfo(blob: &str) -> Result<Package> {
	let mut package = PartialPackage::default();

	for entry in iterate_info(blob) {
		let (key, value) = entry?;
		match key {
			"pkgname"     => set_once_err(&mut package.name, value.into(), key)?,
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
			_               => {},
		}
	}

	package.into_package().map_err(|e| ParseError::whole_blob(blob, e))
}

/// Find all .SRCINFO files in a given directory.
pub fn find_srcinfo<P: ?Sized + AsRef<Path>>(directory: &P) -> impl Iterator<Item = result::Result<DirEntry, walkdir::Error>> {
	WalkDir::new(directory).into_iter().filter(|x|
		if let &Ok(ref entry) = x {
			entry.file_type().is_file() && entry.path().file_name() == Some(OsStr::new(".SRCINFO"))
		} else {
			true
		}
	)
}

pub fn read_srcinfo_db<P: ?Sized + AsRef<Path>>(directory: &P) -> impl Iterator<Item = result::Result<(String, Package), ReadDbError>> {
	find_srcinfo(directory).map(|entry| {
		entry.map_err(|err| ReadDbError::WalkError(err)).and_then(|entry| -> result::Result<(String, Package), ReadDbError> {
			let mut file = File::open(entry.path()).map_err(|x| ReadDbError::IoError(entry.path().to_path_buf(), x))?;
			let mut data = String::new();
			file.read_to_string(&mut data).map_err(|x| ReadDbError::IoError(entry.path().to_path_buf(), x))?;
			let package = parse_srcinfo(&data).map_err(|x| ReadDbError::ParseError(entry.path().to_path_buf(), data, x))?;
			Ok((package.name.clone(), package))
		})
	})
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
