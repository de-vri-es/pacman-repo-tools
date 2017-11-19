use std::str::pattern::{Pattern,ReverseSearcher,SearchStep,Searcher};


pub trait ConsumableStr<'a> : Copy {
	fn consume_front_n(self, n: usize) -> (Option<&'a str>, &'a str);
	fn consume_back_n(self, n: usize) -> (Option<&'a str>, &'a str);

	fn consume_front<P: Pattern<'a>>(self, pattern: P) -> (Option<&'a str>, &'a str);
	fn consume_back<P: Pattern<'a>>(self, pattern: P) -> (Option<&'a str>, &'a str)
		where P::Searcher: ReverseSearcher<'a>;

	fn  partition<P: Pattern<'a>>(self, pattern: P) -> Option<(&'a str, &'a str, &'a str)>;
	fn rpartition<P: Pattern<'a>>(self, pattern: P) -> Option<(&'a str, &'a str, &'a str)>
		where P::Searcher: ReverseSearcher<'a>;
}

impl<'a> ConsumableStr<'a> for &'a str {
	fn consume_front_n(self, n: usize) -> (Option<&'a str>, &'a str) {
		if self.len() < n { (None, self) }
		else { (Some(&self[0..n]), &self[n..]) }
	}

	fn consume_back_n(self, n: usize) -> (Option<&'a str>, &'a str) {
		if self.len() < n {
			(None, self)
		} else {
			let index = self.len() - 1 - n;
			(Some(&self[index..]), &self[..index])
		}
	}

	fn consume_front<P: Pattern<'a>> (self, pattern: P) -> (Option<&'a str>, &'a str) {
		if let SearchStep::Match(_, i) = pattern.into_searcher(self).next() {
			(Some(&self[..i]), &self[i..])
		} else {
			(None, self)
		}
	}

	fn consume_back<P: Pattern<'a>> (self, pattern: P) -> (Option<&'a str>, &'a str)
		where P::Searcher: ReverseSearcher<'a>
	{
		if let SearchStep::Match(i, _) = pattern.into_searcher(self).next_back() {
			(Some(&self[i..]), &self[i..])
		} else {
			(None, self)
		}
	}

	fn partition<P: Pattern<'a>>(self, pattern: P) -> Option<(&'a str, &'a str, &'a str)> {
		pattern.into_searcher(self).next_match().map(|(begin, end)| {
			(&self[..begin], &self[begin..end], &self[end..])
		})
	}

	fn rpartition<P: Pattern<'a>>(self, pattern: P) -> Option<(&'a str, &'a str, &'a str)>
		where P::Searcher: ReverseSearcher<'a>
	{
		pattern.into_searcher(self).next_match_back().map(|(begin, end)| {
			(&self[..begin], &self[begin..end], &self[end..])
		})
	}
}

#[cfg(test)]
mod tests {
	use super::ConsumableStr;

	#[test]
	fn partition() {
		assert_eq!("abc"        .partition('='), None);
		assert_eq!("abc=def"    .partition('='), Some(("abc", "=", "def")));
		assert_eq!("abc=def=ghi".partition('='), Some(("abc", "=", "def=ghi")));

		assert_eq!("a=".partition('='), Some(("a", "=", "")));
		assert_eq!("=b".partition('='), Some(("",  "=", "b")));
		assert_eq!("=" .partition('='), Some(("",  "=", "")));
	}

	#[test]
	fn rpartition() {
		assert_eq!("abc"        .rpartition('='), None);
		assert_eq!("abc=def"    .rpartition('='), Some(("abc",     "=", "def")));
		assert_eq!("abc=def=ghi".rpartition('='), Some(("abc=def", "=", "ghi")));

		assert_eq!("a=".rpartition('='), Some(("a", "=", "")));
		assert_eq!("=b".rpartition('='), Some(("",  "=", "b")));
		assert_eq!("=" .rpartition('='), Some(("",  "=", "")));
	}
}
