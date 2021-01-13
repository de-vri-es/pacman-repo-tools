use std::cmp::Ordering;

use super::Version;

fn consume_while<'a, F>(input: &mut &'a str, mut condition: F) -> &'a str
where
	F: FnMut(char) -> bool,
{
	let i = input.find(|c| !condition(c)).unwrap_or(input.len());
	let (result, remainder) = input.split_at(i);
	*input = remainder;
	result
}

pub fn compare_version_string(a: &str, b: &str) -> Ordering {
	let mut a = a;
	let mut b = b;

	// Loop over the alphanumeric parts.
	while !a.is_empty() || !b.is_empty() {
		// Get the first alphanumeric component.
		let mut a_alnum = consume_while(&mut a, |x| x.is_alphanumeric());
		let mut b_alnum = consume_while(&mut b, |x| x.is_alphanumeric());

		// Loop over the numeric and alphabetical parts.
		while !a_alnum.is_empty() || !b_alnum.is_empty() {
			let a_num   = consume_while(&mut a_alnum, |x| x.is_digit(10));
			let b_num   = consume_while(&mut b_alnum, |x| x.is_digit(10));
			let a_alpha = consume_while(&mut a_alnum, |x| x.is_alphabetic());
			let b_alpha = consume_while(&mut b_alnum, |x| x.is_alphabetic());

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
		let a_sep = consume_while(&mut a, |x| !x.is_alphanumeric());
		let b_sep = consume_while(&mut b, |x| !x.is_alphanumeric());
		let ordering = (!a_sep.is_empty()).cmp(&!b_sep.is_empty());
		if ordering != Ordering::Equal { return ordering }
	}

	// If we get here the versions are equal.
	Ordering::Equal
}

pub fn compare_package_version(a: &str, b: &str) -> Ordering {
	Version::from_str(a).cmp(&b.into())
}

#[cfg(test)]
mod test {
	use super::*;
	use assert2::assert;

	#[track_caller]
	fn assert_compare_version_string(a: &str, b: &str, ordering: Ordering) {
		assert!(compare_version_string(a, b) == ordering);
		assert!(compare_version_string(b, a) == ordering.reverse());
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

	#[track_caller]
	fn assert_compare_package_version(a: &str, b: &str, ordering: Ordering) {
		assert!(compare_package_version(a, b) == ordering);
		assert!(compare_package_version(b, a) == ordering.reverse());
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
