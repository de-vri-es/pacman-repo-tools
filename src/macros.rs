#[macro_export]
macro_rules! impl_ord_requisites {
	($($template:tt),*; $type:ty) => (
		impl<$($template)*> PartialOrd for $type { fn partial_cmp(&self, other: &$type) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }}
		impl<$($template)*> PartialEq  for $type { fn          eq(&self, other: &$type) -> bool { self.cmp(other) == std::cmp::Ordering::Equal }}
		impl<$($template)*> Eq         for $type {}
	);
	($type:ty) => (impl_ord_requisites!('a; $type);)
}

#[macro_export]
macro_rules! return_not_equal {
	($a:expr) => {
		match $a {
			std::cmp::Ordering::Equal   => (),
			std::cmp::Ordering::Less    => return std::cmp::Ordering::Less,
			std::cmp::Ordering::Greater => return std::cmp::Ordering::Greater,
		}
	};
}

#[macro_export]
macro_rules! assert_seq {
	($iterator:expr, [$($value:expr),*]) => {{
		let mut it = $iterator;
		$(assert_eq!(it.next(), Some($value));)*
		assert_eq!(it.next(), None);
	}};
	($iterator:expr, [$($value:expr,)*]) => {{
		assert_seq!($iterator, [$($value),*]);
	}};
}

#[cfg(test)]
mod tests {
	#[test]
	fn assert_seq_identifier() {
		let iterator = "aap noot mies".split(" ");
		assert_seq!(iterator, ["aap", "noot", "mies"]);
	}

	#[test]
	fn assert_seq_comma_delimited() {
		let iterator = "aap noot mies".split(" ");
		assert_seq!(iterator, ["aap", "noot", "mies",]);
	}

	#[test]
	fn assert_seq_expression() {
		assert_seq!("aap noot mies".split(" "), ["aap", "noot", "mies"]);
	}

	#[test]
	#[should_panic]
	fn assert_seq_unequal() {
		let iterator = "aap noot mies".split(" ");
		assert_seq!(iterator, ["aap", "noot", "wim"]);
	}

	#[test]
	#[should_panic]
	fn assert_seq_too_short() {
		let iterator = "aap noot mies".split(" ");
		assert_seq!(iterator, ["aap", "noot"]);
	}

	#[test]
	#[should_panic]
	fn assert_seq_too_long() {
		let iterator = "aap noot mies".split(" ");
		assert_seq!(iterator, ["aap", "noot", "mies", "wim"]);
	}
}
