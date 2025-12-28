use std::ops::{AddAssign, MulAssign};

use derive_more::Display;
use serde::{
    Deserialize,
    de::{IntoDeserializer, MapAccess, SeqAccess, Visitor},
};

type Result<T> = std::result::Result<T, ParserError>;

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Host {
    #[serde(rename = "Host")]
    pub name: String,
    #[serde(rename = "HostName")]
    pub host_name: String,
    #[serde(rename = "User")]
    pub user: String,
    #[serde(rename = "IdentityFile")]
    pub identity_file: String,
    #[serde(rename = "Port", default = "default_port")]
    pub port: u16,
}

const fn default_port() -> u16 {
    22
}

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
    input: &'de str,
    // Stores the host name found in the "Host <name>" line to be injected into the map
    pending_host: Option<String>,
}

impl<'de> Deserializer<'de> {
    pub fn from_str(input: &'de str) -> Self {
        Deserializer {
            input,
            pending_host: None,
        }
    }
}

pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
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

    fn advance(&mut self) -> Result<char> {
        let ch = self.peek_char()?;
        self.input = &self.input[ch.len_utf8()..];
        Ok(ch)
    }

    fn skip_whitespace(&mut self) {
        let to_skip = self
            .input
            .chars()
            .take_while(|ch| ch.is_whitespace())
            .count();
        self.input = &self.input[to_skip..];

        // Skip comments
        while self.input.starts_with('#') {
            let to_eol = self.input.chars().take_while(|ch| *ch != '\n').count();
            self.input = &self.input[to_eol..];

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

    fn peek_identifier(&mut self) -> Result<Identifier> {
        let mut iter = self.input.chars().peekable();

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

impl<'de> serde::Deserializer<'de> for & mut Deserializer<'de> {
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

    fn deserialize_tuple<V>(
        self,
        _len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
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

        // Check if this struct starts with the "Host" keyword
        match self.peek_identifier() {
            Ok(Identifier::Host) => {
                self.parse_identifier()?; // Consume "Host"
                self.skip_whitespace();
                let host_name = self.parse_string()?; // Parse the alias (e.g., "mc_server")
                self.skip_whitespace();

                // Store the name to be injected when the map is visited
                self.pending_host = Some(host_name);

                let host = visitor.visit_map(WhitespaceSeparated::new(self))?;
                Ok(host)
            }
            Ok(_) => {
                // If it's not a "Host" block, just deserialize it as a map (or error)
                // For this parser, we primarily expect "Host" blocks.
                Err(ParserError::UnexpectedToken)
            }
            Err(ParserError::Eof) => Err(ParserError::Eof),
            Err(e) => Err(e),
        }
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
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

    // Stub implementations for remaining traits
    fn deserialize_i8<V>(self, _: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_i16<V>(self, _: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_i32<V>(self, _: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_i64<V>(self, _: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_u8<V>(self, _: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_f32<V>(self, _: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_f64<V>(self, _: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_char<V>(self, _: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_u32<V>(self, _: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_bytes<V>(self, _: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_byte_buf<V>(self, _: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_option<V>(self, _: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_u64<V>(self, _: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_bool<V>(self, _: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_i128<V>(self, _: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_u128<V>(self, _: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_unit<V>(self, _: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_enum<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        _: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_unit_struct<V>(
        self,
        _: &'static str,
        _: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_ignored_any<V>(self, _: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
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

        match self.de.peek_identifier() {
            Ok(Identifier::Host) => seed.deserialize(&mut *self.de).map(Some),
            Ok(_) => Err(ParserError::UnexpectedToken),
            Err(ParserError::Eof) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

struct WhitespaceSeparated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> WhitespaceSeparated<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de }
    }
}

impl<'a, 'de> MapAccess<'de> for WhitespaceSeparated<'a, 'de> {
    type Error = ParserError;

    fn next_key_seed<K>(&mut self, seed: K) -> std::result::Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        // If we have a pending host name (from the "Host" line), inject it into the map
        if self.de.pending_host.is_some() {
            // The Host struct has a field renamed to "Host", so we inject that key
            return seed.deserialize("Host".into_deserializer()).map(Some);
        }

        self.de.skip_whitespace();

        if self.de.input.is_empty() {
            return Ok(None);
        }

        // If we encounter another "Host" identifier, the current host block is finished
        if let Ok(Identifier::Host) = self.de.peek_identifier() {
            return Ok(None);
        }

        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        // If we have a pending host value, return it and clear the buffer
        if let Some(host_name) = self.de.pending_host.take() {
            return seed.deserialize(host_name.into_deserializer());
        }

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
        let host: Host = from_str(test_str.trim()).unwrap();
        assert_eq!(host.name, "mc_server");
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
        assert_eq!(hosts.0.len(), 2);

        let h1 = &hosts.0[0];
        assert_eq!(h1.name, "mc_server");
        assert_eq!(h1.host_name, "141.148.218.223");
        assert_eq!(h1.user, "opc");

        let h2 = &hosts.0[1];
        assert_eq!(h2.name, "git_server");
        assert_eq!(h2.host_name, "github.com");
        assert_eq!(h2.user, "git");
        assert_eq!(h2.port, 2222);
    }

    #[test]
    fn test_de_tokens_host() {
        // Note: The tokens reflect the internal view where "Host" becomes a map key
        let host = Host {
            name: "mc_server".to_string(),
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
                    len: 5,
                },
                Token::Str("Host"),
                Token::Str("mc_server"),
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
