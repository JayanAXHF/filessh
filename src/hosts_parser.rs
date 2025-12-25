use std::{
    collections::VecDeque,
    ops::{AddAssign, MulAssign},
};

use derive_more::Display;
use serde::{
    Deserialize,
    de::{IntoDeserializer, Visitor},
};

type Result<T> = std::result::Result<T, ParserError>;

#[derive(Debug, Deserialize)]
pub struct Host {
    #[serde(rename = "HostName")]
    pub host_name: String,
    #[serde(rename = "Port")]
    pub port: u16,
    #[serde(rename = "User")]
    pub user: String,
    #[serde(rename = "IdentityFile")]
    pub identity_file: String,
}

struct HostMapAccess {
    entries: VecDeque<(String, String)>,
}

use serde::de::DeserializeSeed;

impl<'de> serde::de::MapAccess<'de> for HostMapAccess {
    type Error = ParserError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if let Some((key, _)) = self.entries.front() {
            // 🔑 THIS IS THE FIX
            seed.deserialize(key.as_str().into_deserializer()).map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        let (_, value) = self.entries.pop_front().unwrap();
        seed.deserialize(value.as_str().into_deserializer())
    }
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
        self.input.chars().next().ok_or(ParserError::Eof)
    }

    // Consume the first character in the input.
    fn advance(&mut self) -> Result<char> {
        let ch = self.peek_char()?;
        self.input = &self.input[ch.len_utf8()..];
        Ok(ch)
    }

    fn parse_identifier(&mut self) -> Result<Identifier> {
        let mut identifier = String::new();
        let to_skip = self
            .input
            .chars()
            .take_while(|ch| ch.is_whitespace())
            .count();
        self.input = &self.input[to_skip..];
        while let Ok(ch) = self.peek_char() {
            if ch.is_alphanumeric() {
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
        let mut string = String::new();
        while let Ok(ch) = self.peek_char() {
            if ch == '\n' {
                self.advance()?;
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
        //   let to_skip = self
        //       .input
        //       .chars()
        //       .take_while(|ch| ch.is_whitespace())
        //       .count();
        //   self.input = &self.input[to_skip..];
        //   let identifier = self.parse_identifier()?;
        //   match identifier {
        //       Identifier::Host => {
        //           self.advance()?;
        //           self.parse_string()?;
        //           let Some(line_end) = self.input.find('\n') else {
        //               return Err(ParserError::Eof);
        //           };
        //           self.input = &self.input[line_end + 1..];
        //           loop {
        //               while let Ok(ch) = self.peek_char() {
        //                   if ch.is_whitespace() {
        //                       self.advance()?;
        //                   } else {
        //                       break;
        //                   }
        //               }
        //               let Some(look_ahead) = self.input.find(char::is_whitespace) else {
        //                   break;
        //               };
        //               let next_identifier = &self.input[..look_ahead];
        //               match Identifier::try_from(next_identifier.to_string())? {
        //                   Identifier::Host => {
        //                       break;
        //                   }
        //                   Identifier::Port => {
        //                       self.parse_identifier()?;
        //                       self.advance()?;
        //                       self.parse_unsigned::<u16>()?;
        //                       let Some(line_end) = self.input.find('\n') else {
        //                           continue;
        //                       };

        //                       self.input = &self.input[line_end + 1..];
        //                   }
        //                   _ => {
        //                       self.parse_identifier()?;
        //                       self.advance()?;
        //                       self.parse_string()?;
        //                       let Some(line_end) = self.input.find('\n') else {
        //                           continue;
        //                       };
        //                       self.input = &self.input[line_end + 1..];
        //                   }
        //               }
        //           }
        //       }
        //       _ => {
        //           println!("Unexpected token: {:?}", identifier);
        //           return Err(ParserError::UnexpectedToken);
        //       }
        //   }
        //   visitor.visit_unit()
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

    fn deserialize_u16<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u16(self.parse_unsigned()?)
    }

    fn deserialize_seq<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_map<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        use std::collections::VecDeque;

        let mut entries = VecDeque::new();

        // Expect: Host <alias>
        let ident = self.parse_identifier()?;
        if !matches!(ident, Identifier::Host) {
            return Err(ParserError::UnexpectedToken);
        }

        self.advance()?; // space
        self.parse_string()?; // alias (ignored)

        // consume newline
        if let Some(i) = self.input.find('\n') {
            self.input = &self.input[i + 1..];
        }

        loop {
            self.input = self.input.trim_start();
            if self.input.is_empty() {
                break;
            }

            let ident_end = self
                .input
                .find(char::is_whitespace)
                .ok_or(ParserError::UnexpectedToken)?;

            let key = &self.input[..ident_end];

            if key == "Host" {
                break;
            }

            self.parse_identifier()?; // key
            self.advance()?; // space
            let value = self.parse_string()?;

            entries.push_back((key.to_string(), value.trim().to_string()));

            if let Some(i) = self.input.find('\n') {
                self.input = &self.input[i + 1..];
            } else {
                break;
            }
        }

        visitor.visit_map(HostMapAccess { entries })
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
        self.deserialize_map(visitor)
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
        unimplemented!()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_host() {
        let test_str = "Host mc_server
	HostName 141.148.218.223
	User opc
        Port 22
	IdentityFile ~/Downloads/ssh-key-2024-06-13.key";
        let host: Host = from_str(test_str.trim()).unwrap();
        assert_eq!(host.host_name, "141.148.218.223");
    }
}
