use std::io::{BufRead, BufReader};
use std::path::Path;
use serde::de;
use serde::de::Error as _;

pub struct Deserializer<R> {
	reader: R,
	source: Option<String>,
	line: u32,
	peek_buffer: Option<String>,
}

impl<R: BufRead> Deserializer<R> {
	pub fn new(reader: R, source: Option<String>) -> Self {
		Self {
			reader,
			source,
			line: 0,
			peek_buffer: None,
		}
	}
}

impl<'a> Deserializer<std::io::Cursor<&'a str>> {
	pub fn from_str(data: &'a str, source: Option<String>) -> Self {
		Self::new(std::io::Cursor::new(data), source)
	}
}

impl<'a> Deserializer<std::io::Cursor<&'a [u8]>> {
	pub fn from_bytes(data: &'a [u8], source: Option<String>) -> Self {
		Self::new(std::io::Cursor::new(data), source)
	}
}

impl Deserializer<BufReader<std::fs::File>> {
	pub fn from_file(path: impl AsRef<Path>) -> std::io::Result<Self> {
		let path = path.as_ref();
		let file = std::fs::File::open(path)?;
		let source = path.display().to_string();
		Ok(Self::new(BufReader::new(file), Some(source)))
	}
}

pub fn from_str<'a, T: serde::de::DeserializeOwned>(data: &'a str) -> Result<T, Error> {
	let mut deserializer = Deserializer::from_str(data, None);
	T::deserialize(&mut deserializer)
}

pub fn from_bytes<'a, T: serde::de::DeserializeOwned>(data: &'a [u8]) -> Result<T, Error> {
	let mut deserializer = Deserializer::from_bytes(data, None);
	T::deserialize(&mut deserializer)
}

pub fn from_file<'a, T: serde::de::DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, Error> {
	let path = path.as_ref();
	let mut deserializer = Deserializer::from_file(path)
		.map_err(|e| Error {
			source: Some(path.display().to_string()),
			line: None,
			message: format!("failed to open file for reading: {}", e),
		})?;
	T::deserialize(&mut deserializer)
}

#[derive(Debug)]
pub struct Error {
	source: Option<String>,
	line: Option<u32>,
	message: String,
}

impl<R: BufRead> Deserializer<R> {
	fn error(&self, msg: impl ToString) -> Error {
		Error {
			source: self.source.clone(),
			line: Some(self.line),
			message: msg.to_string(),
		}
	}

	fn read_error(&self, error: std::io::Error) -> Error {
		Error {
			source: self.source.clone(),
			line: None,
			message: error.to_string(),
		}
	}

	fn extend_error<T>(&self, result: Result<T, Error>) -> Result<T, Error> {
		result.map_err(|e| Error {
			source: e.source.or_else(|| self.source.clone()),
			line: e.line.or_else(|| Some(self.line)),
			message: e.message,
		})
	}

	fn peek_line(&mut self) -> Result<Option<&str>, Error> {
		if self.peek_buffer.is_none() {
			if let Some(line) = self.read_line()? {
				self.line -= 1;
				self.peek_buffer = Some(line);
			}
		}
		Ok(self.peek_buffer.as_deref())
	}

	fn read_line(&mut self) -> Result<Option<String>, Error> {
		if let Some(line) = self.peek_buffer.take() {
			self.line += 1;
			return Ok(Some(line))
		}

		loop {
			// Try to read a line.
			let mut line = String::new();
			let size = self.reader.read_line(&mut line).map_err(|e| self.read_error(e))?;

			// If we got one, strip line endings and return it.
			if size == 0 {
				return Ok(None)
			} else {
				if line.ends_with('\n') {
					line.pop();
				}
				if line.ends_with('\r') {
					line.pop();
				}
				self.line += 1;
				if line.is_empty() {
					continue;
				}
				return Ok(Some(line));
			}
		}
	}

	/// Like [`Self::read_line`], but return an error if the input is exhausted.
	fn read_expected_line(&mut self) -> Result<String, Error> {
		self.read_line()?.ok_or_else(|| self.error("unexpected end of file"))
	}

	/// Read a line and parse a [`std::str::FromStr`] value from it.
	fn read_value<T: std::str::FromStr>(&mut self, type_name: &str) -> Result<T, Error> {
		let line = self.read_expected_line()?;
		let value = line.parse().map_err(|_| self.error(format_args!("invalid value, expected {}", type_name)))?;
		Ok(value)
	}

	/// Read a line and parse a key from it.
	///
	/// A line with a key has the format '%KEY%`.
	/// If the read line does not match the format,
	/// an error is returned.
	fn read_key(&mut self) -> Result<Option<String>, Error> {
		let mut line = match self.read_line()? {
			None => return Ok(None),
			Some(x) => x,
		};

		if line.starts_with('%') && line.ends_with('%') {
			line.pop();
			line.remove(0);
			Ok(Some(line))
		} else {
			Err(self.error("expected \"%NAME%\""))
		}
	}
}

fn unexpected_top_level_type(name: &str) -> Error {
	Error::custom(format_args!("the top level type must be a struct for the ALPM database format, but it is {}", name))
}

impl<'de, R: BufRead> de::Deserializer<'de> for &'_ mut Deserializer<R> {
	type Error = Error;

	fn deserialize_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		visitor.visit_map(self)
	}

	fn deserialize_ignored_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		visitor.visit_map(self)
	}

	fn deserialize_bool<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("a boolean"))
	}

	fn deserialize_u8<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("a u8"))
	}

	fn deserialize_u16<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("a u16"))
	}

	fn deserialize_u32<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("a u32"))
	}

	fn deserialize_u64<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("a u64"))
	}

	fn deserialize_i8<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("an i8"))
	}

	fn deserialize_i16<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("an i16"))
	}

	fn deserialize_i32<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("an i32"))
	}

	fn deserialize_i64<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("an i64"))
	}

	fn deserialize_f32<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("an f32"))
	}

	fn deserialize_f64<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("an f64"))
	}

	fn deserialize_char<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("a character"))
	}

	fn deserialize_str<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("a string"))
	}

	fn deserialize_string<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("a string"))
	}

	fn deserialize_bytes<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("a byte string"))
	}

	fn deserialize_byte_buf<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("a byte string"))
	}

	fn deserialize_option<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("an optional value"))
	}

	fn deserialize_newtype_struct<V: de::Visitor<'de>>(self, _name: &str, visitor: V) -> Result<V::Value, Self::Error> {
		// ALPM database files don't do newtype structs, just parse the inner value directly.
		// If it's not a struct, we'll still give an error.
		visitor.visit_newtype_struct(self)
	}

	fn deserialize_tuple<V: de::Visitor<'de>>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("a tuple"))
	}

	fn deserialize_tuple_struct<V: de::Visitor<'de>>(self, _name: &str, _len: usize, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("a tuple struct"))
	}

	fn deserialize_unit<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("a unit value"))
	}

	fn deserialize_unit_struct<V: de::Visitor<'de>>(self, _name: &str, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("a unit struct"))
	}

	fn deserialize_map<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		visitor.visit_map(self)
	}

	fn deserialize_enum<V: de::Visitor<'de>>(self, _name: &str, _variants: &[&str], _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("an enum"))
	}

	fn deserialize_seq<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("a list or sequence"))
	}

	fn deserialize_struct<V: de::Visitor<'de>>(self, _name: &str, _fields: &[&str], visitor: V) -> Result<V::Value, Self::Error> {
		visitor.visit_map(self)
	}

	fn deserialize_identifier<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(unexpected_top_level_type("an identifier"))
	}
}

impl<'de, R: BufRead> de::MapAccess<'de> for Deserializer<R> {
	type Error = Error;

	fn next_key_seed<K: de::DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error> {
		if self.peek_line()?.is_some() {
			let deserializer = FieldDeserializer {
				parent: self,
				in_sequence: false,
			};
			Ok(Some(seed.deserialize(deserializer)?))
		} else {
			Ok(None)
		}
	}

	fn next_value_seed<K: de::DeserializeSeed<'de>>(&mut self, seed: K) -> Result<K::Value, Self::Error> {
		let deserializer = FieldDeserializer {
			parent: self,
			in_sequence: false,
		};
		seed.deserialize(deserializer)
	}
}

/// Deserializer that can only deserialize unstructered values.
struct FieldDeserializer<'a, R> {
	parent: &'a mut Deserializer<R>,
	in_sequence: bool,
}

impl<'de, R: BufRead> de::Deserializer<'de> for FieldDeserializer<'_, R> {
	type Error = Error;

	fn deserialize_any<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(Error::custom("ALPM database field values are not self describing, so deserialize_any is not supported"))
	}

	fn deserialize_ignored_any<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(Error::custom("ALPM database field values are not self describing, so deserialize_ignored_any is not supported"))
	}

	fn deserialize_bool<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		let value = self.parent.read_value("bool")?;
		self.parent.extend_error(visitor.visit_bool(value))
	}

	fn deserialize_u8<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		let value = self.parent.read_value("u8")?;
		self.parent.extend_error(visitor.visit_u8(value))
	}

	fn deserialize_u16<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		let value = self.parent.read_value("u16")?;
		self.parent.extend_error(visitor.visit_u16(value))
	}

	fn deserialize_u32<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		let value = self.parent.read_value("u32")?;
		self.parent.extend_error(visitor.visit_u32(value))
	}

	fn deserialize_u64<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		let value = self.parent.read_value("u64")?;
		self.parent.extend_error(visitor.visit_u64(value))
	}

	fn deserialize_i8<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		let value = self.parent.read_value("i8")?;
		self.parent.extend_error(visitor.visit_i8(value))
	}

	fn deserialize_i16<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		let value = self.parent.read_value("i16")?;
		self.parent.extend_error(visitor.visit_i16(value))
	}

	fn deserialize_i32<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		let value = self.parent.read_value("i32")?;
		self.parent.extend_error(visitor.visit_i32(value))
	}

	fn deserialize_i64<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		let value = self.parent.read_value("i64")?;
		self.parent.extend_error(visitor.visit_i64(value))
	}

	fn deserialize_f32<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		let value = self.parent.read_value("f32")?;
		self.parent.extend_error(visitor.visit_f32(value))
	}

	fn deserialize_f64<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		let value = self.parent.read_value("f64")?;
		self.parent.extend_error(visitor.visit_f64(value))
	}

	fn deserialize_char<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		let line = self.parent.read_expected_line()?;
		let mut chars = line.chars();
		let value = chars.next().unwrap();

		if chars.next().is_some() {
			Err(self.parent.error("invalid value, expected char"))
		} else {
			self.parent.extend_error(visitor.visit_char(value))
		}
	}

	fn deserialize_str<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		let value = self.parent.read_value("string")?;
		self.parent.extend_error(visitor.visit_string(value))
	}

	fn deserialize_string<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		let value = self.parent.read_value("string")?;
		self.parent.extend_error(visitor.visit_string(value))
	}

	fn deserialize_bytes<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(Error::custom("unsupported data type: bytes"))
	}

	fn deserialize_byte_buf<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(Error::custom("unsupported data type: bytes"))
	}

	fn deserialize_option<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		// None values are not present in the input,
		// so if we get to this point there is a value.
		visitor.visit_some(self)
	}

	fn deserialize_newtype_struct<V: de::Visitor<'de>>(self, _name: &str, visitor: V) -> Result<V::Value, Self::Error> {
		// ALPM database files don't describe their types, so just parse the inner value directly.
		visitor.visit_newtype_struct(self)
	}

	fn deserialize_tuple<V: de::Visitor<'de>>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(Error::custom("ALPM database format does not support tuples"))
	}

	fn deserialize_tuple_struct<V: de::Visitor<'de>>(self, _name: &str, _len: usize, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(Error::custom("ALPM database format does not support tuple structs"))
	}

	fn deserialize_unit<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(Error::custom("ALPM database format does not support unit values"))
	}

	fn deserialize_unit_struct<V: de::Visitor<'de>>(self, _name: &str, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(Error::custom("ALPM database format does not support unit structs"))
	}

	fn deserialize_map<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
		Err(Error::custom("ALPM database format does not support maps"))
	}

	fn deserialize_enum<V: de::Visitor<'de>>(self, name: &str, variants: &[&str], visitor: V) -> Result<V::Value, Self::Error> {
		// We only support value-less enums, so we simply check fo
		let line: String = self.parent.read_value(name)?;
		for &variant in variants {
			if line == variant {
				return visitor.visit_str(variant);
			}
		}
		Err(self.parent.error(format_args!("invalud enum variant for {}: {}", name, line)))
	}

	fn deserialize_seq<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		if self.in_sequence {
			Err(Error::custom("ALPM database format does not support nested lists"))
		} else {
			visitor.visit_seq(self)
		}
	}

	fn deserialize_struct<V: de::Visitor<'de>>(self, _name: &str, _fields: &[&str], _visitor: V) -> Result<V::Value, Self::Error> {
		Err(Error::custom("ALPM database format does not support nested structs"))
	}

	fn deserialize_identifier<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		let key = self.parent.read_key()?;
		if let Some(key) = key {
			self.parent.extend_error(visitor.visit_string(key))
		} else {
			Err(self.parent.error("expected \"%NAME%\""))
		}
	}
}

impl<'de, R: BufRead> de::SeqAccess<'de> for FieldDeserializer<'_, R> {
	type Error = Error;

	fn next_element_seed<T: de::DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> {
		let line = match self.parent.peek_line()? {
			None => return Ok(None),
			Some(x) => x,
		};
		if line.starts_with('%') && line.ends_with('%') {
			Ok(None)
		} else {
			let deserializer = FieldDeserializer {
				parent: &mut self.parent,
				in_sequence: true,
			};
			Ok(Some(seed.deserialize(deserializer)?))
		}
	}
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match (&self.source, self.line) {
			(Some(source), Some(line)) => write!(f, "{}:{}: {}", source, line, self.message),
			(Some(source), None) => write!(f, "{}: {}", source, self.message),
			_ => f.write_str(&self.message),
		}
	}
}

impl de::Error for Error {
	fn custom<T: ToString>(msg: T) -> Self {
		Self {
			source: None,
			line: None,
			message: msg.to_string(),
		}
	}
}


#[cfg(test)]
mod test {
	use super::*;
	use assert2::{assert, let_assert};
	use serde::Deserialize;

	#[test]
	#[rustfmt::skip]
	fn simple() {
		#[derive(Debug, Eq, PartialEq, Deserialize)]
		#[serde(rename_all = "UPPERCASE")]
		struct Test {
			foo: Vec<String>,
			bar: Vec<i32>,
			baz: bool,
		}
		let blob = [
			"%FOO%",
			"aap",
			"noot",
			"mies",
			"",
			"%BAR%",
			"10",
			"-5",
			"+8",
			"",
			"%BAZ%",
			"true",
		].join("\n");

		let_assert!(Ok(parsed) = from_str::<Test>(&blob));
		assert!(parsed == Test {
			foo: vec!["aap".into(), "noot".into(), "mies".into()],
			bar: vec![10, -5, 8],
			baz: true,
		});
	}
}
