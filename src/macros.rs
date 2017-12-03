macro_rules! impl_ord {
	($($template:tt),*; $type:ty; $_self:ident, $other:ident => $body:block) => (
		impl<$($template)*> Ord for $type { fn cmp(&$_self, $other: &$type) -> std::cmp::Ordering $body }
		impl<$($template)*> PartialOrd for $type { fn partial_cmp(&$_self, other: &$type) -> Option<std::cmp::Ordering> { Some($_self.cmp(other)) }}
		impl<$($template)*> PartialEq  for $type { fn          eq(&$_self, other: &$type) -> bool { $_self.cmp(other) == std::cmp::Ordering::Equal }}
		impl<$($template)*> Eq         for $type {}
	);
	($type:ty; $_self:ident, $other:ident => $body:block) => (impl_ord!('a; $type; $_self, $other => $body););
}
