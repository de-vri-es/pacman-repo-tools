// Copyright (c) 2017, Maarten de Vries
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// * Redistributions of source code must retain the above copyright notice, this
//   list of conditions and the following disclaimer.
//
// * Redistributions in binary form must reproduce the above copyright notice,
//   this list of conditions and the following disclaimer in the documentation
//   and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::str::pattern::{Pattern,ReverseSearcher,Searcher,SearchStep};

pub trait DefaultOption {
	type Item;

	fn get_or_default(&mut self) -> &mut Self::Item;
}


impl<T: Default> DefaultOption for Option<T>
{
	type Item = T;

	fn get_or_default(&mut self) -> &mut T {
		self.get_or_insert_with(|| T::default())
	}
}

pub trait ConsumableStr<'a> : Copy {
	fn consume_front_n(&mut self, n: usize) -> Option<&'a str>;
	fn consume_back_n(&mut self, n: usize) -> Option<&'a str>;

	fn consume_front<P: Pattern<'a>>(&mut self, pattern: P) -> Option<&'a str>;
	fn consume_back<P: Pattern<'a>>(&mut self, pattern: P) -> Option<&'a str>
		where P::Searcher: ReverseSearcher<'a>;

	fn consume_front_while<P: Pattern<'a>>(&mut self, pattern: P) -> &'a str;
	fn consume_back_while<P: Pattern<'a>>(&mut self, pattern: P) -> &'a str
		where P::Searcher: ReverseSearcher<'a>;

	fn consume_front_while_not<P: Pattern<'a>>(&mut self, pattern: P) -> &'a str;
	fn consume_back_while_not<P: Pattern<'a>>(&mut self, pattern: P) -> &'a str
		where P::Searcher: ReverseSearcher<'a>;

	fn  partition<P: Pattern<'a>>(&self, pattern: P) -> Option<(&'a str, &'a str, &'a str)>;
	fn rpartition<P: Pattern<'a>>(&self, pattern: P) -> Option<(&'a str, &'a str, &'a str)>
		where P::Searcher: ReverseSearcher<'a>;
}

impl<'a> ConsumableStr<'a> for &'a str {
	fn consume_front_n(&mut self, n: usize) -> Option<&'a str> {
		if self.len() < n { return None }
		let result = Some(&self[..n]);
		*self = &self[n..];
		result
	}

	fn consume_back_n(&mut self, n: usize) -> Option<&'a str> {
		if self.len() < n { return None }
		let index = self.len() - n;
		let result = Some(&self[index..]);
		*self = &self[..index];
		result
	}

	fn consume_front<P: Pattern<'a>>(&mut self, pattern: P) -> Option<&'a str> {
		if let SearchStep::Match(start, end) = pattern.into_searcher(self).next() {
			let result = Some(&self[start..end]);
			*self = &self[end..];
			result
		} else {
			None
		}
	}

	fn consume_back<P: Pattern<'a>>(&mut self, pattern: P) -> Option<&'a str>
		where P::Searcher: ReverseSearcher<'a>
	{
		if let SearchStep::Match(start, end) = pattern.into_searcher(self).next_back() {
			let result = Some(&self[start..end]);
			*self = &self[..start];
			result
		} else {
			None
		}
	}

	fn consume_front_while<P: Pattern<'a>> (&mut self, pattern: P) -> &'a str {
		let i = pattern.into_searcher(self).next_reject().map(|(i, _)| i).unwrap_or(self.len());
		let (left, right) = self.split_at(i);
		*self = right;
		left
	}

	fn consume_back_while<P: Pattern<'a>> (&mut self, pattern: P) -> &'a str
		where P::Searcher: ReverseSearcher<'a>
	{
		let i = pattern.into_searcher(self).next_reject_back().map(|(_, i)| i).unwrap_or(0);
		let (left, right) = self.split_at(i);
		*self = left;
		right
	}

	fn consume_front_while_not<P: Pattern<'a>> (&mut self, pattern: P) -> &'a str {
		let i = pattern.into_searcher(self).next_match().map(|(i, _)| i).unwrap_or(self.len());
		let (left, right) = self.split_at(i);
		*self = right;
		left
	}

	fn consume_back_while_not<P: Pattern<'a>> (&mut self, pattern: P) -> &'a str
		where P::Searcher: ReverseSearcher<'a>
	{
		let i = pattern.into_searcher(self).next_match_back().map(|(_, i)| i).unwrap_or(0);
		let (left, right) = self.split_at(i);
		*self = left;
		right
	}

	fn partition<P: Pattern<'a>>(&self, pattern: P) -> Option<(&'a str, &'a str, &'a str)> {
		pattern.into_searcher(self).next_match().map(|(begin, end)| {
			(&self[..begin], &self[begin..end], &self[end..])
		})
	}

	fn rpartition<P: Pattern<'a>>(&self, pattern: P) -> Option<(&'a str, &'a str, &'a str)>
		where P::Searcher: ReverseSearcher<'a>
	{
		pattern.into_searcher(self).next_match_back().map(|(begin, end)| {
			(&self[..begin], &self[begin..end], &self[end..])
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_get_or_default() {
		let mut i: Option<i32> = None;
		assert_eq!(i.get_or_default(), &0);
		assert_eq!(i, Some(0));
	}

	#[test]
	fn consume_front_n() {
		let data = String::from("abcdef");
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_front_n(0), Some(""));
			assert_eq!(a, "abcdef");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_front_n(3), Some("abc"));
			assert_eq!(a, "def");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_front_n(6), Some("abcdef"));
			assert_eq!(a, "");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_front_n(7), None);
			assert_eq!(a, "abcdef");
		}
	}

	#[test]
	fn consume_back_n() {
		let data = String::from("abcdef");
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_back_n(0), Some(""));
			assert_eq!(a, "abcdef");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_back_n(3), Some("def"));
			assert_eq!(a, "abc");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_back_n(6), Some("abcdef"));
			assert_eq!(a, "");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_back_n(7), None);
			assert_eq!(a, "abcdef");
		}
	}

	#[test]
	fn consume_front() {
		let data = String::from("abcdef");
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_front("abc"), Some("abc"));
			assert_eq!(a, "def");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_front("def"), None);
			assert_eq!(a, "abcdef");
		}
	}

	#[test]
	fn consume_back() {
		let data = String::from("abcdef");
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_back("def"), Some("def"));
			assert_eq!(a, "abc");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_back("abc"), None);
			assert_eq!(a, "abcdef");
		}
	}

	#[test]
	fn consume_front_while() {
		let data = String::from("aaabbb");
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_front_while('a'), "aaa");
			assert_eq!(a, "bbb");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_front_while(|c| c == 'a'), "aaa");
			assert_eq!(a, "bbb");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_front_while('b'), "");
			assert_eq!(a, "aaabbb");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_front_while(|c| c == 'a' || c == 'b'), "aaabbb");
			assert_eq!(a, "");
		}
	}

	#[test]
	fn consume_back_while() {
		let data = String::from("aaabbb");
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_back_while('b'), "bbb");
			assert_eq!(a, "aaa");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_back_while(|c| c == 'b'), "bbb");
			assert_eq!(a, "aaa");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_back_while('a'), "");
			assert_eq!(a, "aaabbb");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_back_while(|c| c == 'a' || c == 'b'), "aaabbb");
			assert_eq!(a, "");
		}
	}

	#[test]
	fn consume_front_while_not() {
		let data = String::from("aaabbb");
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_front_while_not('b'), "aaa");
			assert_eq!(a, "bbb");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_front_while_not(|c| c == 'b'), "aaa");
			assert_eq!(a, "bbb");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_front_while_not('a'), "");
			assert_eq!(a, "aaabbb");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_front_while_not(|c| c == 'c'), "aaabbb");
			assert_eq!(a, "");
		}
	}

	#[test]
	fn consume_back_while_not() {
		let data = String::from("aaabbb");
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_back_while_not('a'), "bbb");
			assert_eq!(a, "aaa");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_back_while_not(|c| c == 'a'), "bbb");
			assert_eq!(a, "aaa");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_back_while_not('b'), "");
			assert_eq!(a, "aaabbb");
		}
		{
			let mut a: &str = data.as_ref();
			assert_eq!(a.consume_back_while_not(|c| c == 'c'), "aaabbb");
			assert_eq!(a, "");
		}
	}

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
