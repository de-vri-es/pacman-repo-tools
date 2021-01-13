use crate::parse::{partition, rpartition};

pub fn consume_epoch(version: &mut &str) -> Option<i32> {
	let (epoch, rest) = partition(version, ':')?;
	let epoch = if epoch.is_empty() { 0 } else { epoch.parse().ok()? };
	*version = rest;
	Some(epoch)
}

pub fn consume_pkgrel<'a>(version: &mut &'a str) -> Option<&'a str> {
	let (rest, pkgrel) = rpartition(version, '-')?;
	*version = rest;
	Some(pkgrel)
}

#[cfg(test)]
mod test {
	use super::*;
	use assert2::assert;

	#[test]
	fn test_consume_epoch() {
		{
			let mut a = "3:a";
			assert!(consume_epoch(&mut a) == Some(3));
			assert!(a == "a");
		}
		{
			let mut a = "3:a:b";
			assert!(consume_epoch(&mut a) == Some(3));
			assert!(a == "a:b");
		}
		{
			let mut a = "31:a";
			assert!(consume_epoch(&mut a) == Some(31));
			assert!(a == "a");
		}
		{
			let mut a = "a";
			assert!(consume_epoch(&mut a) == None);
			assert!(a == "a");
		}
		{
			let mut a = "abc";
			assert!(consume_epoch(&mut a) == None);
			assert!(a == "abc");
		}
		{
			let mut a = "3a1:a";
			assert!(consume_epoch(&mut a) == None);
			assert!(a == "3a1:a");
		}
		{
			let mut a = ":1.3.2";
			assert!(consume_epoch(&mut a) == Some(0));
			assert!(a == "1.3.2");
		}
	}

	#[test]
	fn test_consume_pkgrel() {
		{
			let mut a = "1-2";
			assert!(consume_pkgrel(&mut a) == Some("2"));
			assert!(a == "1");
		}
		{
			let mut a = "1-2-3";
			assert!(consume_pkgrel(&mut a) == Some("3"));
			assert!(a == "1-2");
		}
		{
			let mut a = "1.2abc-3.4def";
			assert!(consume_pkgrel(&mut a) == Some("3.4def"));
			assert!(a == "1.2abc");
		}
		{
			let mut a = "123";
			assert!(consume_pkgrel(&mut a) == None);
			assert!(a == "123");
		}
	}
}
