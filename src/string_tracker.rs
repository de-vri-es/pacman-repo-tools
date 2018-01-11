use std;
use std::borrow::Cow;
use std::collections::Bound::{Excluded, Included, Unbounded};
use std::collections::btree_map::Entry as BTreeMapEntry;
use std::fs::File;
use std::io::Read;
use std::path::{Path,PathBuf};
use std::str::from_utf8_unchecked;

trait PointerRange {
	type Type;

	fn start_ptr(&self) -> *const Self::Type;
	fn end_ptr(&self)   -> *const Self::Type;
}

impl<T: AsRef<[u8]>> PointerRange for T {
	type Type = u8;
	fn start_ptr(&self) -> *const u8 { self.as_ref().as_ptr() }
	fn end_ptr(&self)   -> *const u8 { unsafe { self.as_ref().as_ptr().add(self.as_ref().len()) } }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Source<'a, 'path> {
	Other,
	ExpandedFrom(&'a str),
	File(&'path Path),
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum SourceStorage<'a> {
	Other,
	ExpandedFrom(&'a str),
	File(PathBuf),
}

impl<'a, 'path> Source<'a, 'path> {
	fn to_storage(self) -> SourceStorage<'a> {
		match self {
			Source::Other                => SourceStorage::Other,
			Source::ExpandedFrom(string) => SourceStorage::ExpandedFrom(string),
			Source::File(path)           => SourceStorage::File(path.to_owned()),
		}
	}
}

impl<'a> SourceStorage<'a> {
	fn to_source<'b>(&'b self) -> Source<'a, 'b> {
		match self {
			&SourceStorage::Other                    => Source::Other,
			&SourceStorage::ExpandedFrom(ref string) => Source::ExpandedFrom(string),
			&SourceStorage::File(ref path)           => Source::File(path),
		}
	}
}

pub struct Entry<'a> {
	data: Cow<'a, [u8]>,
	source: SourceStorage<'a>,
}

/// Read a file into a string.
fn read_text_file<P: ?Sized + AsRef<Path>>(path: &P) -> std::io::Result<String> {
	let mut file = File::open(path)?;
	let mut data = String::new();
	file.read_to_string(&mut data)?;
	return Ok(data);
}

/// Tracker for strings with metadata.
///
/// The tracker can take ownership or store references if their lifetime is long enough.
/// Each string added to the tracker has some source information attached to it.
/// This information can later be retrieved from the tracker with a (partial) &str.
///
/// The tracker can not track empty strings,
/// and it can not look up information for empty string slices.
#[derive(Default)]
pub struct StringTracker<'a> {
	map: std::cell::UnsafeCell<std::collections::BTreeMap<*const u8, Entry<'a>>>
}

impl<'a> StringTracker<'a> {
	pub fn new() -> Self { Self::default() }

	/// Get the map from the UnsafeCell.
	fn map(&self) -> &std::collections::BTreeMap<*const u8, Entry<'a>> {
		unsafe { &*self.map.get() }
	}

	/// Get the map from the UnsafeCell as mutable map.
	fn map_mut(&self) -> &mut std::collections::BTreeMap<*const u8, Entry<'a>> {
		unsafe { &mut *self.map.get() }
	}

	/// Find the first entry with start_ptr <= the given bound.
	fn first_entry_at_or_before(&self, bound: *const u8) -> Option<&Entry<'a>> {
		let (_key, value) = self.map().range((Unbounded, Included(bound))).next_back()?;
		Some(&value)
	}

	/// Find the first entry with start_ptr < the given bound.
	fn first_entry_before(&self, bound: *const u8) -> Option<&Entry<'a>> {
		let (_key, value) = self.map().range((Unbounded, Excluded(bound))).next_back()?;
		Some(&value)
	}

	/// Check if the given data has overlap with anything in the string tracker.
	fn has_overlap<S: ?Sized + AsRef<[u8]>>(&self, data: &S) -> bool {
		let data = data.as_ref();

		// Empty slices can't overlap with anything, even if their start pointer is tracked.
		if data.is_empty() { return false }

		// Last element with start < data.end_ptr()
		let conflict = match self.first_entry_before(data.end_ptr()) {
			None        => return false,
			Some(entry) => entry,
		};

		// If conflict doesn't end before data starts, it's a conflict.
		// Though end is one-past the end, so end == start is also okay.
		conflict.data.end_ptr() > data.start_ptr()
	}

	/// Get the entry tracking a string.
	fn get_entry(&self, data: &str) -> Option<&Entry> {
		// Empty strings aren't tracked.
		// They can't be distuingished from str_a[end..end] or str_b[0..0],
		// if str_a and str_b directly follow eachother in memory.
		if data.is_empty() { return None }

		// Get the last element where start_ptr <= data.start_ptr
		let entry = self.first_entry_at_or_before(data.start_ptr())?;
		if data.end_ptr() <= entry.data.end_ptr() {
			Some(entry)
		} else {
			None
		}
	}

	/// Insert data with source information without checking if the data is already present.
	unsafe fn insert_unsafe<'path>(&self, data: Cow<'a, [u8]>, source: SourceStorage<'a>) -> &[u8] {
		// Insert the data itself.
		match self.map_mut().entry(data.start_ptr()) {
			BTreeMapEntry::Vacant(x)   => x.insert(Entry{data, source}).data.as_ref(),
			BTreeMapEntry::Occupied(_) => unreachable!(),
		}
	}

	/// Like insert, but convert the Source to SourceStorage only after all checks are done.
	fn insert_with_source<'path>(&self, data: Cow<'a, [u8]>, source: Source<'a, 'path>) -> Result<&[u8], ()> {
		// Reject empty data or data that is already (partially) tracked.
		if data.is_empty() || self.has_overlap(data.as_ref()) { return Err(()) }
		Ok(unsafe { self.insert_unsafe(data, source.to_storage()) })
	}

	/// Insert a borrowed reference in the tracker.
	///
	/// Fails if the string is empty or if it is already tracked.
	pub fn insert_borrow<'path, S: ?Sized + AsRef<str>>(&self, data: &'a S, source: Source<'a, 'path>) -> Result<&str, ()> {
		let slice = self.insert_with_source(Cow::Borrowed(data.as_ref().as_bytes()), source)?;
		Ok(unsafe { from_utf8_unchecked(slice) })
	}

	/// Move a string into the tracker.
	///
	/// Fails if the string is empty.
	pub fn insert_move<'path, S: Into<String>>(&self, data: S, source: Source<'a, 'path>) -> Result<&str, ()> {
		// New string can't be in the map yet, but empty string can not be inserted.
		let slice = self.insert_with_source(Cow::Owned(data.into().into_bytes()), source)?;
		Ok(unsafe { from_utf8_unchecked(slice) })
	}

	/// Read a file and insert it into the tracker.
	///
	/// Fails if reading the file fails, or if the file is empty.
	pub fn insert_file<P: Into<PathBuf>>(&self, path: P) -> std::io::Result<&str> {
		let path = path.into();
		let data = read_text_file(&path)?;
		if data.is_empty() {
			Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "file is empty"))
		} else {
			Ok(unsafe { from_utf8_unchecked(self.insert_unsafe(Cow::Owned(data.into_bytes()), SourceStorage::File(path))) })
		}
	}

	/// Check if a string slice is tracked.
	pub fn is_tracked(&self, data: &str) -> bool {
		self.get_entry(data).is_some()
	}

	/// Get the whole tracked slice and source information for a string slice.
	pub fn get(&self, data: &str) -> Option<(&str, Source)> {
		self.get_entry(data).map(|entry| {
			let string = unsafe { from_utf8_unchecked(&entry.data) };
			(string, entry.source.to_source())
		})
	}

	/// Get the source information for a string slice.
	pub fn get_source(&self, data: &str) -> Option<Source> {
		self.get_entry(data).map(|entry| entry.source.to_source())
	}

	/// Get the whole tracked slice for a string slice.
	pub fn get_whole_slice(&self, data: &str) -> Option<&str> {
		self.get_entry(data).map(|entry| unsafe { from_utf8_unchecked(&entry.data) })
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_insert_borrow() {
		let pool = StringTracker::default();
		let data = "aap noot mies";
		let len  = data.len();
		assert_eq!(pool.is_tracked(data), false);

		// Cant insert empty string slices.
		assert!(pool.insert_borrow("",          Source::Other).is_err());
		assert!(pool.insert_borrow(&data[3..3], Source::Other).is_err());

		// Can insert non-empty str only once.
		let tracked = pool.insert_borrow(data, Source::Other).unwrap();
		assert!(pool.insert_borrow(data, Source::Other).is_err());
		assert!(pool.is_tracked(data));

		// is_tracked says no to empty sub-slices.
		assert!(!pool.is_tracked(&data[0..0]));
		assert!(!pool.is_tracked(&data[1..1]));
		assert!(!pool.is_tracked(&data[len..len]));

		// non-empty sub-slices give the whole slice back.
		assert!(std::ptr::eq(data, tracked));
		assert!(std::ptr::eq(data, pool.get_whole_slice(data).unwrap()));
		assert!(std::ptr::eq(data, pool.get_whole_slice(&data[0..1]).unwrap()));
		assert!(std::ptr::eq(data, pool.get_whole_slice(&data[4..8]).unwrap()));
		assert!(std::ptr::eq(data, pool.get_whole_slice(&data[len-1..len]).unwrap()));
		assert!(std::ptr::eq(data, pool.get_whole_slice(&data[..]).unwrap()));
	}

	#[test]
	fn test_insert_part() {
		let pool = StringTracker::default();
		let data = "aap noot mies";
		let noot = &data[4..8];
		assert_eq!(noot, "noot");


		// Adding the subslice to the pool doesn't make the whole str tracked.
		let tracked = pool.insert_borrow(noot, Source::Other).unwrap();
		assert!(pool.is_tracked(noot));
		assert!(pool.is_tracked(&data[4..8]));
		assert!(!pool.is_tracked(data));
		assert!(!pool.is_tracked(&data[ ..4]));
		assert!(!pool.is_tracked(&data[8.. ]));

		// But we can't track the whole slice anymore now.
		assert!(pool.insert_borrow(data, Source::Other).is_err());

		// Subslices from the original str in the right range give the whole tracked subslice.
		assert!(std::ptr::eq(noot, tracked));
		assert!(std::ptr::eq(noot, pool.get_whole_slice(noot).unwrap()));
		assert!(std::ptr::eq(noot, pool.get_whole_slice(&data[4..8]).unwrap()));
		assert!(std::ptr::eq(noot, pool.get_whole_slice(&data[4..7]).unwrap()));
		assert!(std::ptr::eq(noot, pool.get_whole_slice(&data[5..8]).unwrap()));
		assert!(std::ptr::eq(noot, pool.get_whole_slice(&data[5..7]).unwrap()));
	}

	#[test]
	fn test_insert_move() {
		let pool = StringTracker::default();

		// Can't insert empty strings.
		assert!(pool.insert_move("",            Source::Other).is_err());
		assert!(pool.insert_move(String::new(), Source::Other).is_err());

		let data: &str = pool.insert_move("aap noot mies", Source::Other).unwrap();
		let len = data.len();
		assert!(pool.is_tracked(data), true);
		assert!(!pool.is_tracked(&data[0..0]));
		assert!(!pool.is_tracked(&data[5..5]));
		assert!(!pool.is_tracked(&data[len..len]));
		assert!(!pool.is_tracked("aap"));

		assert!(std::ptr::eq(data, pool.get_whole_slice(data).unwrap()));
		assert!(std::ptr::eq(data, pool.get_whole_slice(&data[0..1]).unwrap()));
		assert!(std::ptr::eq(data, pool.get_whole_slice(&data[4..8]).unwrap()));
		assert!(std::ptr::eq(data, pool.get_whole_slice(&data[len-1..len]).unwrap()));
		assert!(std::ptr::eq(data, pool.get_whole_slice(&data[..]).unwrap()));
	}
}
