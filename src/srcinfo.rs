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
use std::error;
use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::io::Error as IoError;
use std::io::Read;
use std::path::{Path,PathBuf};

extern crate walkdir;
use self::walkdir::{DirEntry, WalkDir};

use error::ParseError;
use package::Package;
use package::VersionConstraint;
use parse::parse_depends;
use parse::parse_provides;
use util::ConsumableStr;
use version::Version;
use version::VersionBuf;

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
/// Empty lines are yielded as `None`.
/// Data lines are yielded as `Some((&str, &str))`.
pub fn iterate_info<'a>(blob: &'a str) -> impl Iterator<Item = Result<Option<(&'a str, &'a str)>, ParseError>> {
	blob.split('\n').map(move |line| {
		let line = line.trim();
		if line.is_empty() {
			return Ok(None);
		}
		line.partition('=').map(|(key, _, value)| Some((key.trim(), value.trim())))
		.ok_or_else(|| ParseError::for_token(blob, line, "expected 'key = value' or empty line"))
	})
}

fn set_once<T>(option: &mut Option<T>, value: T) -> Option<()> {
	if option.is_some() {
		None
	} else {
		*option = Some(value);
		Some(())
	}
}

fn set_once_err<T>(option: &mut Option<T>, value: T, blob: &str, key: &str) -> Result<(), ParseError> {
	set_once(option, value).ok_or_else(|| ParseError::for_token(blob, key, format!("duplicate key: {}", key)))
}

fn insert_err<V, IV: Into<V>>(map: &mut Map<String,V>, map_name: &str, blob: &str, entry: (&str, IV)) -> Result<(), ParseError> {
	let (key, value) = entry;
	match map.entry(key.into()) {
		Entry::Vacant(x)   => { x.insert(value.into()); Ok(()) },
		Entry::Occupied(_) => { Err(ParseError::for_token(blob, key, format!("duplicate {}: {}", map_name, key))) }
	}
}

pub fn parse_srcinfo(blob: &str) -> Result<Package, ParseError> {
	let mut name:    Option<String>     = Default::default();
	let mut version: Option<VersionBuf> = Default::default();

	let mut url:           Option<String> = Default::default();
	let mut description:   Option<String> = Default::default();
	let mut licenses:      Vec<String>    = Default::default();

	let mut groups:        Vec<String> = Default::default();
	let mut backup:        Vec<String> = Default::default();

	let mut provides:      Map<String, Option<VersionBuf>> = Default::default();
	let mut conflicts:     Map<String, Option<VersionConstraint>> = Default::default();
	let mut replaces:      Map<String, Option<VersionConstraint>> = Default::default();

	let mut depends:       Map<String, Option<VersionConstraint>> = Default::default();
	let mut make_depends:  Map<String, Option<VersionConstraint>> = Default::default();
	let mut check_depends: Map<String, Option<VersionConstraint>> = Default::default();

	for entry in iterate_info(blob) {
		if let Some((key, value)) = entry? {
			if false {}
			else if key == "pkgname"     { set_once_err(&mut name, value.into(), blob, key)? }
			else if key == "version"     { set_once_err(&mut version, Version::from_str(value).into(), blob, key)? }
			else if key == "url"         { set_once_err(&mut url, value.into(), blob, key)? }
			else if key == "description" { set_once_err(&mut description, value.into(), blob, key)? }


			else if key == "licenses"      { licenses.push(value.into()); }
			else if key == "groups"        { groups.push(value.into()); }
			else if key == "backup"        { backup.push(value.into()); }

			else if key == "provides"      { insert_err(&mut provides, "provides", blob, parse_provides(value))? }
			else if key == "conflicts"     { insert_err(&mut conflicts, "conflicts", blob, parse_depends(value))? }

			else if key == "replaces"      { insert_err(&mut replaces,      "replaces",      blob, parse_depends(value))? }
			else if key == "depends"       { insert_err(&mut depends,       "depends",       blob, parse_depends(value))? }
			else if key == "make_depends"  { insert_err(&mut make_depends,  "make_depends",  blob, parse_depends(value))? }
			else if key == "check_depends" { insert_err(&mut check_depends, "check_depends", blob, parse_depends(value))? }
		}
	}

	Ok(Package {
		name:    name.ok_or_else(|| ParseError::whole_blob(blob, "no pkgname found"))?,
		version: version.ok_or_else(|| ParseError::whole_blob(blob, "no pkgver found"))?,
		url, description, licenses,
		groups, backup,
		provides, conflicts, replaces,
		depends, make_depends, check_depends,
	})
}

/// Find all .SRCINFO files in a given directory.
pub fn find_srcinfo(directory: &Path) -> impl Iterator<Item = Result<DirEntry, walkdir::Error>> {
	WalkDir::new(directory).into_iter().filter(|x|
		match x {
			&Err(_)        => true,
			&Ok(ref entry) => entry.file_type().is_file() && entry.path().file_name() == Some(OsStr::new(".SRCINFO")),
		}
	)
}

pub fn read_srcinfo_db(directory: &Path) -> impl Iterator<Item = Result<(String, Package), ReadDbError>> {
	find_srcinfo(directory).map(|entry| {
		entry.map_err(|err| ReadDbError::WalkError(err)).and_then(|entry| -> Result<(String, Package), ReadDbError> {
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
			Ok(Some(("a", "b"))),
			Ok(Some(("c", "d"))),
		])
	}

	#[test]
	fn spaces_are_stripped() {
		assert_seq!(iterate_info(" a   =    b  "), [Ok(Some(("a", "b")))])
	}

	#[test]
	fn empty_lines_are_none() {
		let blob = ["  ", "a=b", "", "c=d", ""].join("\n");
		assert_seq!(iterate_info(&blob), [
			Ok(None),
			Ok(Some(("a", "b"))),
			Ok(None),
			Ok(Some(("c", "d"))),
			Ok(None),
		])
	}

	#[test]
	fn garbage_gives_error() {
		let blob = ["ab", "a = b"].join("\n");
		let mut iterator = iterate_info(&blob);
		assert!(iterator.next().unwrap().is_err());
		assert_eq!(iterator.next(), Some(Ok(Some(("a", "b")))));
		assert_eq!(iterator.next(), None);
	}

}
