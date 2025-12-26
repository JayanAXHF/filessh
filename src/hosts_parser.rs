use std::{
    collections::VecDeque,
    ops::{AddAssign, MulAssign},
};

use derive_more::Display;
use serde::{
    Deserialize,
    de::{IntoDeserializer, MapAccess, Visitor},
};

type Result<T> = std::result::Result<T, ParserError>;

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Host {
    #[serde(rename = "HostName")]
    pub host_name: String,
    #[serde(rename = "User")]
    pub user: String,
    #[serde(rename = "IdentityFile")]
    pub identity_file: String,
    #[serde(rename = "Port")]
    pub port: u16,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Hosts {
    #[serde(flatten)]
    pub hosts: Vec<Host>,
}

struct HostMapAccess {
    entries: VecDeque<(String, String)>,
}

#[derive(Debug)]
enum Identifier {
    Host,
    HostName,
    Port,
    User,
    IdentityFile,
}

impl serde::de::Error for ParserError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        ParserError::Message(msg.to_string())
    }
}

impl TryFrom<String> for Identifier {
    type Error = ParserError;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        match value.as_str() {
            "Host" => Ok(Identifier::Host),
            "HostName" => Ok(Identifier::HostName),
            "Port" => Ok(Identifier::Port),
            "User" => Ok(Identifier::User),
            "IdentityFile" => Ok(Identifier::IdentityFile),
            _ => Err(ParserError::UnexpectedToken),
        }
    }
}

#[derive(thiserror::Error, Debug, Display)]
pub enum ParserError {
    TrailingCharacters,
    Eof,
    ExpectedInteger,
    UnexpectedToken,

    Message(String),
}

pub struct Deserializer<'de> {
    // This string starts with the input data and characters are truncated off
    // the beginning as data is parsed.
    input: &'de str,
}

impl<'de> Deserializer<'de> {
    // By convention, `Deserializer` constructors are named like `from_xyz`.
    // That way basic use cases are satisfied by something like
    // `serde_json::from_str(...)` while advanced use cases that require a
    // deserializer can make one with `serde_json::Deserializer::from_str(...)`.
    pub fn from_str(input: &'de str) -> Self {
        Deserializer { input }
    }
}

// By convention, the public API of a Serde deserializer is one or more
// `from_xyz` methods such as `from_str`, `from_bytes`, or `from_reader`
// depending on what Rust types the deserializer is able to consume as input.
//
// This basic deserializer supports only `from_str`.
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        Err(ParserError::TrailingCharacters)
    }
}

impl<'de> Deserializer<'de> {
    fn peek_char(&mut self) -> Result<char> {
        self.input.chars().next().ok_or_else(|| {
            dbg!(self.input);
            ParserError::Eof
        })
    }

    // Consume the first character in the input.
    fn advance(&mut self) -> Result<char> {
        let ch = self.peek_char()?;
        dbg!("advance");
        self.input = &self.input[ch.len_utf8()..];
        Ok(ch)
    }

    fn parse_identifier(&mut self) -> Result<Identifier> {
        let mut identifier = String::new();
        let to_skip = self
            .input
            .chars()
            .take_while(|ch| matches!(ch, ' ' | '\t'))
            .count();
        self.input = &self.input[to_skip..];
        println!("{}", self.input);
        while let Ok(ch) = self.peek_char() {
            dbg!("{}", ch);
            if !ch.is_whitespace() {
                identifier.push(ch);
                self.advance()?;
            } else {
                println!("{}", ch);
                println!("{}", self.input);
                break;
            }
        }
        println!("identifier: {}", identifier);
        Identifier::try_from(identifier)
    }

    fn parse_string(&mut self) -> Result<String> {
        dbg!("parse_string");
        let mut string = String::new();
        while let Ok(ch) = self.peek_char() {
            if ch.is_whitespace() {
                break;
            }
            string.push(ch);
            self.advance()?;
        }
        println!("string: {}", string);
        println!("input:\n{}", self.input);
        Ok(string)
    }

    fn parse_unsigned<T>(&mut self) -> Result<T>
    where
        T: AddAssign<T> + MulAssign<T> + From<u8>,
    {
        let mut int = match self.advance()? {
            ch @ '0'..='9' => T::from(ch as u8 - b'0'),
            _ => {
                dbg!(&self.input);
                return Err(ParserError::ExpectedInteger);
            }
        };
        loop {
            match self.input.chars().next() {
                Some(ch @ '0'..='9') => {
                    self.input = &self.input[1..];
                    int *= T::from(10);
                    int += T::from(ch as u8 - b'0');
                }
                _ => {
                    return Ok(int);
                }
            }
        }
    }
}

impl<'de, 'a> serde::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = ParserError;
    fn deserialize_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }
    fn deserialize_string<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_string(self.parse_string()?)
    }

    fn deserialize_str<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u16(self.parse_unsigned()?)
    }

    fn deserialize_seq<V>(self, _visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_map<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        dbg!(&self.input);
        while let Ok(ch) = self.peek_char() {
            if ch.is_whitespace() {
                self.advance()?;
            } else {
                break;
            }
        }
        let value = visitor.visit_map(WhitespaceSeparated::new(self))?;
        Ok(value)
    }

    fn deserialize_i8<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_i16<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_i32<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_i64<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_u8<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    // Float parsing is stupidly hard.
    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    // The `Serializer` implementation on the previous page serialized chars as
    // single-character strings so handle that representation here.
    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Parse a string, check that it is one character, call `visit_char`.
        unimplemented!()
    }
    fn deserialize_u32<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    // Refer to the "Understanding deserializer lifetimes" page for information
    // about the three deserialization flavors of strings in Serde.
    // The `Serializer` implementation on the previous page serialized byte
    // arrays as JSON arrays of bytes. Handle that representation here.
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_option<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_u64<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_bool<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_i128<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_u128<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_unit<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_tuple<V>(
        self,
        len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn is_human_readable(&self) -> bool {
        true
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let trimmed = self.input.trim_start();
        self.input = trimmed;
        let next_identifier_str = self
            .input
            .chars()
            .take_while(|ch| !ch.is_whitespace())
            .collect::<String>();
        let next_identifier = Identifier::try_from(next_identifier_str)?;
        match next_identifier {
            Identifier::Host => {
                self.parse_identifier()?;
                self.advance()?;
                self.parse_string()?;
                self.advance()?;
                dbg!(&self.input);
                let host = visitor.visit_map(WhitespaceSeparated::new(self))?;
                Ok(host)
            }
            _ => Err(ParserError::UnexpectedToken),
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_ignored_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
}

struct WhitespaceSeparated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    first: bool,
}

impl<'a, 'de> WhitespaceSeparated<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de, first: true }
    }
}

impl<'a, 'de> serde::de::SeqAccess<'de> for WhitespaceSeparated<'a, 'de> {
    type Error = ParserError;

    fn next_element_seed<T>(
        &mut self,
        seed: T,
    ) -> std::result::Result<Option<T::Value>, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        if self.de.peek_char()? == '\n' {
            return Ok(None);
        }
        if !self.first && self.de.advance()? != ' ' {
            dbg!("error here");
            return Err(ParserError::UnexpectedToken);
        }

        self.first = false;
        seed.deserialize(&mut *self.de).map(Some)
    }
}

impl<'a, 'de> MapAccess<'de> for WhitespaceSeparated<'a, 'de> {
    type Error = ParserError;
    fn next_key_seed<K>(&mut self, seed: K) -> std::result::Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        dbg!("next_key_seed");
        if self.de.input.is_empty() {
            return Ok(None);
        }
        let ch = self.de.advance()?;
        if !self.first && ch != '\n' {
            dbg!("error here");
            return Err(ParserError::UnexpectedToken);
        }
        self.first = false;
        let trimmed = self.de.input.trim_start();
        self.de.input = trimmed;
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        dbg!("next_value_seed");
        let n_ch = self.de.advance()?;
        if !matches!(n_ch, ' ' | '\t') {
            dbg!("error here: {}");
            return Err(ParserError::UnexpectedToken);
        }
        seed.deserialize(&mut *self.de)
    }
}

#[cfg(test)]
mod tests {
    use serde_test::{Token, assert_de_tokens};

    use super::*;

    #[test]
    fn test_deserialize_host() {
        let test_str = "Host mc_server
	HostName 141.148.218.223
	User opc
        Port 22
	IdentityFile ~/Downloads/ssh-key-2024-06-13.key ";
        println!("starting deserialization");
        let host: Host = from_str(test_str.trim()).unwrap();
        println!("{:?}", host);
    }

    #[test]
    fn test_de_tokens_host() {
        let host = Host {
            host_name: "141.148.218.223".to_string(),
            user: "opc".to_string(),
            identity_file: "~/Downloads/ssh-key-2024-06-13.key".to_string(),
            port: 22,
        };
        assert_de_tokens(
            &host,
            &[
                Token::Struct {
                    name: "Host",
                    len: 4,
                },
                Token::Str("HostName"),
                Token::Str("141.148.218.223"),
                Token::Str("User"),
                Token::Str("opc"),
                Token::Str("IdentityFile"),
                Token::Str("~/Downloads/ssh-key-2024-06-13.key"),
                Token::Str("Port"),
                Token::U16(22),
                Token::StructEnd,
            ],
        );
    }
}
