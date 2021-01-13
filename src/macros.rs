#[macro_export]
#[rustfmt::skip]
macro_rules! impl_ord_requisites {
	($($template:tt),*; $type:ty) => (
		impl<$($template)*> PartialOrd for $type { fn partial_cmp(&self, other: &$type) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }}
		impl<$($template)*> PartialEq  for $type { fn          eq(&self, other: &$type) -> bool { self.cmp(other) == std::cmp::Ordering::Equal }}
		impl<$($template)*> Eq         for $type {}
	);
	($type:ty) => (impl_ord_requisites!('a; $type);)
}
