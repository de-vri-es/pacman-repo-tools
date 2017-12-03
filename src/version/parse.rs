use util::ConsumableStr;

use super::types::VersionParts;

fn consume_epoch(v: &mut &str) -> Option<i32> {
	let mut a: &str = v;
	let epoch = a.consume_front_while(|x: char| x.is_digit(10));
	if a.consume_front_n(1) == Some(":") {
		*v = a;
		Some(if epoch.is_empty() {0} else {epoch.parse().unwrap()})
	} else {
		None
	}
}

fn consume_pkgrel<'a>(v: &mut &'a str) -> Option<&'a str> {
	v.rpartition('-').map(|(rest, _, pkgrel)| {
		*v = rest;
		pkgrel
	})
}

pub fn split_parts(version: &str) -> VersionParts {
	let mut version = version;
	let epoch  = consume_epoch(&mut version).unwrap_or(0);
	let pkgrel = consume_pkgrel(&mut version).map(|x| x.into());
	let pkgver = version.into();
	VersionParts{epoch, pkgver, pkgrel}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_consume_epoch() {
		{
			let mut a = "3:a";
			assert_eq!(consume_epoch(&mut a), Some(3));
			assert_eq!(a, "a");
		}
		{
			let mut a = "3:a:b";
			assert_eq!(consume_epoch(&mut a), Some(3));
			assert_eq!(a, "a:b");
		}
		{
			let mut a = "31:a";
			assert_eq!(consume_epoch(&mut a), Some(31));
			assert_eq!(a, "a");
		}
		{
			let mut a = "a";
			assert_eq!(consume_epoch(&mut a), None);
			assert_eq!(a, "a");
		}
		{
			let mut a = "abc";
			assert_eq!(consume_epoch(&mut a), None);
			assert_eq!(a, "abc");
		}
		{
			let mut a = "3a1:a";
			assert_eq!(consume_epoch(&mut a), None);
			assert_eq!(a, "3a1:a");
		}
	}

	#[test]
	fn test_consume_pkgrel() {
		{
			let mut a = "1-2";
			assert_eq!(consume_pkgrel(&mut a), Some("2"));
			assert_eq!(a, "1");
		}
		{
			let mut a = "1-2-3";
			assert_eq!(consume_pkgrel(&mut a), Some("3"));
			assert_eq!(a, "1-2");
		}
		{
			let mut a = "1.2abc-3.4def";
			assert_eq!(consume_pkgrel(&mut a), Some("3.4def"));
			assert_eq!(a, "1.2abc");
		}
		{
			let mut a = "123";
			assert_eq!(consume_pkgrel(&mut a), None);
			assert_eq!(a, "123");
		}
	}
}
