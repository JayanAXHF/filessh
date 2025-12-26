use std::ops::{AddAssign, MulAssign};

use derive_more::Display;
use serde::{
    Deserialize,
    de::{MapAccess, SeqAccess, Visitor},
};

type Result<T> = std::result::Result<T, ParserError>;

const fn default_port() -> u16 {
    22
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Host {
    #[serde(rename = "HostName")]
    pub host_name: String,
    #[serde(rename = "User")]
    pub user: String,
    #[serde(rename = "IdentityFile")]
    pub identity_file: String,
    #[serde(rename = "Port", default = "default_port")]
    pub port: u16,
}

// Changed to a tuple struct to seamlessly support deserializing a sequence of Hosts
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Hosts(pub Vec<Host>);

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
        // Allow trailing whitespace
        let trimmed = deserializer.input.trim();
        if trimmed.is_empty() {
            Ok(t)
        } else {
            Err(ParserError::TrailingCharacters)
        }
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

    fn skip_whitespace(&mut self) {
        // Skip whitespace
        let to_skip = self
            .input
            .chars()
            .take_while(|ch| ch.is_whitespace())
            .count();
        self.input = &self.input[to_skip..];

        // Skip lines starting with # (comments)
        while self.input.starts_with('#') {
            let to_eol = self.input.chars().take_while(|ch| *ch != '\n').count();
            self.input = &self.input[to_eol..];

            // Remove the newline character itself if present
            if self.input.starts_with('\n') {
                self.input = &self.input[1..];
            }

            let to_skip = self
                .input
                .chars()
                .take_while(|ch| ch.is_whitespace())
                .count();
            self.input = &self.input[to_skip..];
        }
    }

    // Peeks at the next identifier without consuming it
    fn peek_identifier(&mut self) -> Result<Identifier> {
        let mut iter = self.input.chars().peekable();

        // Skip whitespace
        while let Some(&ch) = iter.peek() {
            if ch.is_whitespace() {
                iter.next();
            } else {
                break;
            }
        }

        let mut word = String::new();
        while let Some(&ch) = iter.peek() {
            if !ch.is_whitespace() {
                word.push(ch);
                iter.next();
            } else {
                break;
            }
        }

        Identifier::try_from(word)
    }

    fn parse_identifier(&mut self) -> Result<Identifier> {
        self.skip_whitespace();
        let mut identifier = String::new();

        while let Ok(ch) = self.peek_char() {
            if !ch.is_whitespace() {
                identifier.push(ch);
                self.advance()?;
            } else {
                break;
            }
        }
        Identifier::try_from(identifier)
    }

    fn parse_string(&mut self) -> Result<String> {
        self.skip_whitespace();
        let mut string = String::new();
        while let Ok(ch) = self.peek_char() {
            if ch.is_whitespace() {
                break;
            }
            string.push(ch);
            self.advance()?;
        }
        Ok(string)
    }

    fn parse_unsigned<T>(&mut self) -> Result<T>
    where
        T: AddAssign<T> + MulAssign<T> + From<u8>,
    {
        self.skip_whitespace();
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

    fn deserialize_seq<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_seq(HostsSeqAccess::new(self))
    }

    fn deserialize_map<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_map(WhitespaceSeparated::new(self))
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
        self.deserialize_seq(visitor)
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
        // Check if we are deserializing a Host or Hosts struct
        // Note: Since we changed Hosts to a tuple struct, deserialize_struct might not be called for it
        // depending on serde internals, but if it were a named struct it would.
        // Here we specifically handle the "Host" keyword parsing.

        // For the top-level file parsing into Hosts (tuple struct), deserialize_seq is usually called.
        // This method is primarily for parsing a single "Host" block.

        let trimmed = self.input.trim_start();
        self.input = trimmed;

        // Peek to ensure we are at a Host block
        match self.peek_identifier() {
            Ok(Identifier::Host) => {
                self.parse_identifier()?; // "Host"
                self.advance()?; // space
                self.parse_string()?; // alias (discarded)
                self.advance()?; // newline

                let host = visitor.visit_map(WhitespaceSeparated::new(self))?;
                Ok(host)
            }
            Ok(_) => Err(ParserError::UnexpectedToken),
            Err(ParserError::Eof) => Err(ParserError::Eof),
            Err(e) => Err(e),
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
        self.deserialize_seq(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }
}

struct HostsSeqAccess<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> HostsSeqAccess<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de }
    }
}

impl<'a, 'de> SeqAccess<'de> for HostsSeqAccess<'a, 'de> {
    type Error = ParserError;

    fn next_element_seed<T>(
        &mut self,
        seed: T,
    ) -> std::result::Result<Option<T::Value>, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        self.de.skip_whitespace();

        if self.de.input.is_empty() {
            return Ok(None);
        }

        // Peek to see if we are starting a Host block
        match self.de.peek_identifier() {
            Ok(Identifier::Host) => {
                // Deserialize this Host
                seed.deserialize(&mut *self.de).map(Some)
            }
            Ok(_) => {
                // Found a token that isn't "Host" at the sequence level
                // Depending on strictness, we could error or ignore.
                // For SSH config, we expect Host blocks.
                // Let's return an error as this is likely malformed.
                Err(ParserError::UnexpectedToken)
            }
            Err(ParserError::Eof) => Ok(None),
            Err(e) => Err(e),
        }
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
        // This implementation is kept for completeness but might not be used for the primary logic
        // which relies on MapAccess for Hosts.
        if self.de.peek_char()? == '\n' {
            return Ok(None);
        }
        if !self.first && self.de.advance()? != ' ' {
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
        self.de.skip_whitespace();

        if self.de.input.is_empty() {
            return Ok(None);
        }

        // Check if the next token is "Host", indicating the start of a new host block
        // If so, the current map (Host) is finished.
        if let Ok(Identifier::Host) = self.de.peek_identifier() {
            return Ok(None);
        }

        // Reset first flag is not really applicable here as this is map access,
        // but we ensure we don't consume newlines as separators unless necessary.
        // The current format is Key Value \n Key Value

        // Note: We don't use self.first here because map keys in SSH config
        // don't have a leading separator like the first element of a line usually does in other formats.
        // The first call should succeed immediately if the input pointer is at a Key.

        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        self.de.skip_whitespace();
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
        assert_eq!(host.host_name, "141.148.218.223");
        assert_eq!(host.user, "opc");
        assert_eq!(host.port, 22);
    }

    #[test]
    fn test_deserialize_hosts_multiple() {
        let test_str = "Host mc_server
    HostName 141.148.218.223
    User opc
    Port 22
    IdentityFile ~/Downloads/ssh-key-2024-06-13.key

Host git_server
    HostName github.com
    User git
    Port 2222
    IdentityFile ~/.ssh/id_rsa";

        let hosts: Hosts = from_str(test_str).unwrap();
        println!("{:?}", hosts);
        assert_eq!(hosts.0.len(), 2);

        let h1 = &hosts.0[0];
        assert_eq!(h1.host_name, "141.148.218.223");
        assert_eq!(h1.user, "opc");

        let h2 = &hosts.0[1];
        assert_eq!(h2.host_name, "github.com");
        assert_eq!(h2.user, "git");
        assert_eq!(h2.port, 2222);
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
