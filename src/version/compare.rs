use std::cmp::Ordering;

use util::ConsumableStr;

use super::parse::split_parts;

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

pub fn compare_package_version(a: &str, b: &str) -> Ordering {
	split_parts(a).cmp(&split_parts(b))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn assert_compare_version_string(a: &str, b: &str, ordering: Ordering) {
		assert_eq!(compare_version_string(a, b), ordering, "comparing {:?} to {:?}", a, b);
		assert_eq!(compare_version_string(b, a), ordering.reverse(), "comparing {:?} to {:?}", a, b);
		//assert_eq!(VersionStr(a).cmp(&VersionStr(b)), ordering, "comparing {:?} to {:?}", b, a);
		//assert_eq!(VersionStr(b).cmp(&VersionStr(a)), ordering.reverse(), "comparing {:?} to {:?}", b, a);
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
