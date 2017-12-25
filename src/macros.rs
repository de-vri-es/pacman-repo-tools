macro_rules! impl_ord {
	($($template:tt),*; $type:ty; $_self:ident, $other:ident => $body:block) => (
		impl<$($template)*> Ord for $type { fn cmp(&$_self, $other: &$type) -> std::cmp::Ordering $body }
		impl<$($template)*> PartialOrd for $type { fn partial_cmp(&$_self, other: &$type) -> Option<std::cmp::Ordering> { Some($_self.cmp(other)) }}
		impl<$($template)*> PartialEq  for $type { fn          eq(&$_self, other: &$type) -> bool { $_self.cmp(other) == std::cmp::Ordering::Equal }}
		impl<$($template)*> Eq         for $type {}
	);
	($type:ty; $_self:ident, $other:ident => $body:block) => (impl_ord!('a; $type; $_self, $other => $body););
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

	use std;

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
