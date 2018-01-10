use std;
use std::borrow::Cow;
use std::collections::Bound::{Included, Unbounded};

trait EndPtr {
	type Type;

	fn start_ptr(&self) -> *const Self::Type;
	fn end_ptr(&self)   -> *const Self::Type;

	fn contains(&self, other: &Self) -> bool {
		other.start_ptr() >= self.start_ptr() && other.end_ptr() <= self.end_ptr()
	}

	fn contained_by(&self, other: &Self) -> bool {
		other.contains(self)
	}
}

impl<T: AsRef<[u8]>> EndPtr for T {
	type Type = u8;
	fn start_ptr(&self) -> *const u8 { AsRef::<[u8]>::as_ref(self).as_ptr() }
	fn end_ptr(&self)   -> *const u8 { unsafe { self.as_ref().as_ptr().add(AsRef::<[u8]>::as_ref(self).len()) } }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Source {
	File(std::path::PathBuf),
	ExpandedFrom(*const str),
	Other,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Entry<'a> {
	data: Cow<'a, str>,
	source: Source,
}

impl<'a> Entry<'a> {
	pub fn key(&self) -> *const u8 { self.start_ptr() }
}

impl<'a> AsRef<[u8]> for Entry<'a> {
	fn as_ref(&self) -> &[u8] {
		self.data.as_ref().as_ref()
	}
}

#[derive(Default)]
pub struct StringTracker<'a> {
	map: std::collections::BTreeMap<*const u8, Entry<'a>>
}

impl<'a> StringTracker<'a> {
	pub fn new() -> Self { Self::default() }

	pub fn has_overlap<S: ?Sized + AsRef<[u8]>>(&self, data: &S) -> bool {
		let data = data.as_ref();
		// Last element with start < data.end_ptr()
		let conflict = self.map.range(..data.end_ptr()).next_back();
		if let Some((_key, conflict)) = conflict {
			// If conflict doesn't end before data starts, it's a conflict.
			// Though end is one-past the end, so end == start is also okay.
			conflict.end_ptr() > data.start_ptr()
		} else {
			false
		}
	}

	pub fn insert(&mut self, entry: Entry<'a>) -> Result<(), ()> {
		if self.has_overlap(&entry.data.as_ref()) { return Err(()) }
		self.map.insert(entry.key(), entry);
		Ok(())
	}

	pub fn insert_borrowed<S: ?Sized + AsRef<str>>(&mut self, data: &'a S, source: Source) -> Result<(), ()> {
		self.insert(Entry{data: Cow::Borrowed(data.as_ref().clone()), source})
	}

	pub fn insert_owned<S: Into<String>>(&mut self, data: S, source: Source) -> Result<(), ()> {
		self.insert(Entry{data: Cow::Owned(data.into()), source})
	}

	pub fn get(&self, data: &str) -> Option<&Entry<'a>> {
		// Get the last element where start_ptr <= data.start_ptr
		let entry = self.map.range((Unbounded, Included(data.start_ptr()))).next_back();
		if let Some((_key, entry)) = entry {
			if entry.end_ptr() <= entry.end_ptr() {
				return Some(entry)
			}
		}
		None
	}

	pub fn get_source(&self, data: &str) -> Option<&Source> {
		self.get(data).map(|x| &x.source)
	}

	pub fn get_whole_str(&self, data: &str) -> Option<&str> {
		self.get(data).map(|x| x.data.as_ref())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_overlap() {
		let mut pool = StringTracker::default();
		let data = "aap noot mies";
		assert_eq!(pool.has_overlap(data), false);
		assert_eq!(pool.has_overlap(&data[3..8]), false);
		assert_eq!(pool.insert_borrowed::<str>(data, Source::Other), Ok(()));
		assert_eq!(pool.has_overlap(data), true);
		assert_eq!(pool.has_overlap(&data[3..8]), true);
		assert_eq!(pool.insert_borrowed::<str>(data,        Source::Other), Err(()));
		assert_eq!(pool.insert_borrowed::<str>(&data[3..4], Source::Other), Err(()));
	}

}
