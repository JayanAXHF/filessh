use std::ops::{AddAssign, MulAssign};

use derive_more::Display;
use serde::{
    Deserialize,
    de::{IntoDeserializer, MapAccess, SeqAccess, Visitor},
};

type Result<T> = std::result::Result<T, ParserError>;

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Host {
    /// The patterns the `Host` line listed, separated by whitespace. `ssh`
    /// accepts more than one per block, as in `Host git github.com`.
    #[serde(rename = "Host")]
    pub name: String,
    /// Everything below is optional in `ssh_config(5)`: a block may set only
    /// the keywords it needs, and `Host *` blocks routinely set none of these.
    #[serde(rename = "HostName")]
    pub host_name: Option<String>,
    #[serde(rename = "User")]
    pub user: Option<String>,
    #[serde(rename = "IdentityFile")]
    pub identity_file: Option<String>,
    #[serde(rename = "Port")]
    pub port: Option<u16>,
}

impl Host {
    /// Whether this block applies to `alias`. A `Host` line carries one or more
    /// patterns, in which `*` and `?` are wildcards, and a `!` prefix excludes.
    pub fn matches(&self, alias: &str) -> bool {
        let mut matched = false;
        for pattern in self.name.split_whitespace() {
            match pattern.strip_prefix('!') {
                Some(excluded) => {
                    if pattern_matches(excluded, alias) {
                        return false;
                    }
                }
                None => matched |= pattern_matches(pattern, alias),
            }
        }
        matched
    }
}

fn pattern_matches(pattern: &str, alias: &str) -> bool {
    if pattern.contains(['*', '?', '[']) {
        glob::Pattern::new(pattern).is_ok_and(|pattern| pattern.matches(alias))
    } else {
        pattern == alias
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Hosts(pub Vec<Host>);

impl Hosts {
    /// The settings to connect to `alias` with: every block whose patterns match
    /// it, in the order the file lists them, with the first value obtained for
    /// each keyword winning, as in `ssh_config(5)`. A `Host *` block therefore
    /// supplies whatever the blocks above it left out.
    ///
    /// `None` when no block matches at all. Note that a config with a `Host *`
    /// block always matches, which is also how `ssh` behaves: the alias is then
    /// the host name.
    pub fn settings_for(&self, alias: &str) -> Option<Host> {
        let mut matching = self.0.iter().filter(|host| host.matches(alias)).peekable();
        matching.peek()?;

        let mut settings = Host {
            name: alias.to_owned(),
            host_name: None,
            user: None,
            identity_file: None,
            port: None,
        };
        for host in matching {
            settings.host_name = settings.host_name.or_else(|| host.host_name.clone());
            settings.user = settings.user.or_else(|| host.user.clone());
            settings.identity_file = settings
                .identity_file
                .or_else(|| host.identity_file.clone());
            settings.port = settings.port.or(host.port);
        }
        Some(settings)
    }
}

#[derive(Debug, Clone, Copy)]
enum Identifier {
    Host,
    HostName,
    Port,
    User,
    IdentityFile,
}

impl Identifier {
    /// The spelling [`Host`] renames its fields to, so that every accepted
    /// casing of a keyword reaches the same field.
    const fn field_name(self) -> &'static str {
        match self {
            Identifier::Host => "Host",
            Identifier::HostName => "HostName",
            Identifier::Port => "Port",
            Identifier::User => "User",
            Identifier::IdentityFile => "IdentityFile",
        }
    }
}

impl serde::de::Error for ParserError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        ParserError::Message(msg.to_string())
    }
}

impl TryFrom<String> for Identifier {
    type Error = ParserError;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        // Keywords are case-insensitive in ssh_config(5).
        match value.to_ascii_lowercase().as_str() {
            "host" => Ok(Identifier::Host),
            "hostname" => Ok(Identifier::HostName),
            "port" => Ok(Identifier::Port),
            "user" => Ok(Identifier::User),
            "identityfile" => Ok(Identifier::IdentityFile),
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

    /// Advances past every character up to the first one `keep` accepts.
    /// Indexes by byte offset, so a comment or value containing non-ASCII text
    /// cannot land the input in the middle of a character.
    fn take_until(&mut self, keep: impl Fn(char) -> bool) -> &'de str {
        let end = self
            .input
            .char_indices()
            .find(|(_, ch)| keep(*ch))
            .map_or(self.input.len(), |(idx, _)| idx);
        let (taken, rest) = self.input.split_at(end);
        self.input = rest;
        taken
    }

    /// Consumes the rest of the line, including the newline that ends it.
    fn skip_line(&mut self) {
        self.take_until(|ch| ch == '\n');
        if self.input.starts_with('\n') {
            self.input = &self.input[1..];
        }
    }

    fn skip_whitespace(&mut self) {
        loop {
            self.take_until(|ch| !ch.is_whitespace());
            if !self.input.starts_with('#') {
                return;
            }
            self.skip_line();
        }
    }

    /// Consumes one keyword, plus the `=` separator if the line uses one:
    /// `Port 22`, `Port=22` and `Port = 22` are all the same to `ssh`.
    /// Returns the canonical spelling for keywords it knows, and the word as
    /// written for the rest, which lets serde ignore them.
    fn parse_keyword(&mut self) -> String {
        self.skip_whitespace();
        let word = self.take_until(|ch| ch.is_whitespace() || ch == '=');

        let separator = self.input.trim_start_matches([' ', '\t']);
        if let Some(rest) = separator.strip_prefix('=') {
            self.input = rest;
        }

        Identifier::try_from(word.to_owned())
            .map_or_else(|_| word.to_owned(), |id| id.field_name().to_owned())
    }

    /// The keyword starting at the current position, without consuming it.
    fn peek_identifier(&self) -> Result<Identifier> {
        let mut probe = Deserializer {
            input: self.input,
            pending_host: None,
        };
        probe.skip_whitespace();
        if probe.input.is_empty() {
            return Err(ParserError::Eof);
        }
        Identifier::try_from(
            probe
                .take_until(|ch| ch.is_whitespace() || ch == '=')
                .to_owned(),
        )
    }

    /// Consumes the rest of the line as a value, dropping any trailing comment.
    fn parse_rest_of_line(&mut self) -> String {
        let line = self.take_until(|ch| ch == '\n');
        line.split('#').next().unwrap_or(line).trim().to_owned()
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

impl<'de> serde::Deserializer<'de> for &mut Deserializer<'de> {
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
        self.skip_whitespace();

        // Check if this struct starts with the "Host" keyword
        match self.peek_identifier() {
            Ok(Identifier::Host) => {
                self.parse_keyword(); // Consume "Host"
                // A `Host` line may list several patterns, as in
                // `Host git github.com`; keep all of them.
                let patterns = self.parse_rest_of_line();

                // Store the name to be injected when the map is visited
                self.pending_host = Some(patterns);

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
    /// A keyword that appears always has a value; a keyword that is absent
    /// never reaches the deserializer at all, and serde leaves the field
    /// `None`. So there is nothing to look at here but the value itself.
    fn deserialize_option<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
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
    /// Reached for every keyword the [`Host`] struct does not name, of which a
    /// real config has many: `PreferredAuthentications`, `ForwardAgent`, and so
    /// on. The argument of such a keyword runs to the end of the line, so drop
    /// the line and carry on with the next keyword.
    fn deserialize_ignored_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.skip_line();
        visitor.visit_unit()
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
        loop {
            self.de.skip_whitespace();

            if self.de.input.is_empty() {
                return Ok(None);
            }

            match self.de.peek_identifier() {
                Ok(Identifier::Host) => return seed.deserialize(&mut *self.de).map(Some),
                // A directive that belongs to no `Host` block, which is how
                // configs open: `Include`, `AddKeysToAgent`, `ServerAliveInterval`.
                // They apply to every host, and this parser reads per-host
                // settings only, so pass over them.
                Ok(_) | Err(ParserError::UnexpectedToken) => self.de.skip_line(),
                Err(ParserError::Eof) => return Ok(None),
                Err(e) => return Err(e),
            }
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

        // Hand serde the canonical spelling so that any casing of a keyword
        // reaches the right field, and an unknown one is ignored by name.
        seed.deserialize(self.de.parse_keyword().into_deserializer())
            .map(Some)
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
        assert_eq!(host.host_name.as_deref(), Some("141.148.218.223"));
        assert_eq!(host.user.as_deref(), Some("opc"));
        assert_eq!(host.port, Some(22));
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
        assert_eq!(h1.host_name.as_deref(), Some("141.148.218.223"));
        assert_eq!(h1.user.as_deref(), Some("opc"));

        let h2 = &hosts.0[1];
        assert_eq!(h2.name, "git_server");
        assert_eq!(h2.host_name.as_deref(), Some("github.com"));
        assert_eq!(h2.user.as_deref(), Some("git"));
        assert_eq!(h2.port, Some(2222));
    }

    /// A config in the shape people actually keep, exercising every tolerance
    /// below at once.
    const REALISTIC_CONFIG: &str = "\
# Defaults for every host
Include ~/.ssh/config.d/*
AddKeysToAgent yes

Host mc_server
	HostName 141.148.218.223
	User opc
	Port 22
	IdentityFile ~/Downloads/ssh-key.key
	PreferredAuthentications publickey
	ForwardAgent no

Host git_server
	hostname github.com
	user git
	Port=2222
	IdentityFile ~/.ssh/id_rsa
";

    #[test]
    fn unknown_keywords_are_ignored() {
        let host: Host = from_str(
            "Host mc_server
	HostName 141.148.218.223
	PreferredAuthentications publickey
	User opc
	IdentityFile ~/.ssh/id_rsa",
        )
        .unwrap();
        assert_eq!(host.host_name.as_deref(), Some("141.148.218.223"));
        assert_eq!(host.user.as_deref(), Some("opc"));
    }

    #[test]
    fn keywords_are_case_insensitive() {
        let host: Host = from_str(
            "Host mc_server
	hostname 141.148.218.223
	USER opc
	IdentityFile ~/.ssh/id_rsa
	port 2222",
        )
        .unwrap();
        assert_eq!(host.host_name.as_deref(), Some("141.148.218.223"));
        assert_eq!(host.user.as_deref(), Some("opc"));
        assert_eq!(host.port, Some(2222));
    }

    #[test]
    fn keywords_accept_an_equals_separator() {
        let host: Host = from_str(
            "Host mc_server
	HostName=141.148.218.223
	User = opc
	IdentityFile ~/.ssh/id_rsa
	Port=2222",
        )
        .unwrap();
        assert_eq!(host.host_name.as_deref(), Some("141.148.218.223"));
        assert_eq!(host.user.as_deref(), Some("opc"));
        assert_eq!(host.port, Some(2222));
    }

    #[test]
    fn directives_before_the_first_host_are_ignored() {
        let hosts: Hosts = from_str(
            "Include ~/.ssh/config.d/*
AddKeysToAgent yes

Host mc_server
	HostName 141.148.218.223
	User opc
	IdentityFile ~/.ssh/id_rsa",
        )
        .unwrap();
        assert_eq!(hosts.0.len(), 1);
        assert_eq!(hosts.0[0].name, "mc_server");
    }

    #[test]
    fn wildcard_block_without_per_host_keywords_parses() {
        let hosts: Hosts = from_str(
            "Host *
	ServerAliveInterval 60
	AddKeysToAgent yes

Host mc_server
	HostName 141.148.218.223",
        )
        .unwrap();
        assert_eq!(hosts.0.len(), 2);
        assert_eq!(hosts.0[0].name, "*");
        assert_eq!(hosts.0[0].host_name, None);
        assert_eq!(hosts.0[0].user, None);
        assert_eq!(hosts.0[0].identity_file, None);
        assert_eq!(hosts.0[0].port, None);
        // `*` matches everything, which is the point of such a block.
        assert!(hosts.0[0].matches("mc_server"));
    }

    #[test]
    fn a_host_line_may_list_several_patterns() {
        let host: Host = from_str(
            "Host git github.com gh
	HostName github.com
	User git",
        )
        .unwrap();
        assert!(host.matches("git"));
        assert!(host.matches("github.com"));
        assert!(host.matches("gh"));
        assert!(!host.matches("gitlab.com"));
    }

    #[test]
    fn comments_may_contain_non_ascii() {
        // Byte offsets and character counts part ways here, and slicing the
        // input by the wrong one panics.
        let host: Host = from_str(
            "# ✨ the résumé server ✨
Host mc_server	# inline ✨ comment
	HostName 141.148.218.223",
        )
        .unwrap();
        assert_eq!(host.name, "mc_server");
        assert_eq!(host.host_name.as_deref(), Some("141.148.218.223"));
    }

    #[test]
    fn a_wildcard_block_supplies_what_others_leave_out() {
        let hosts: Hosts = from_str(
            "Host web
	HostName web.example.com

Host *
	User deploy
	IdentityFile ~/.ssh/id_ed25519
	Port 2222",
        )
        .unwrap();

        let web = hosts.settings_for("web").unwrap();
        assert_eq!(web.host_name.as_deref(), Some("web.example.com"));
        assert_eq!(web.user.as_deref(), Some("deploy"));
        assert_eq!(web.identity_file.as_deref(), Some("~/.ssh/id_ed25519"));
        assert_eq!(web.port, Some(2222));
    }

    #[test]
    fn the_first_value_obtained_wins() {
        let hosts: Hosts = from_str(
            "Host web
	User specific

Host *
	User general",
        )
        .unwrap();
        assert_eq!(
            hosts.settings_for("web").unwrap().user.as_deref(),
            Some("specific")
        );
    }

    #[test]
    fn patterns_match_wildcards_and_exclusions() {
        let hosts: Hosts = from_str(
            "Host *.example.com !secret.example.com
	User deploy",
        )
        .unwrap();
        assert_eq!(
            hosts
                .settings_for("web.example.com")
                .unwrap()
                .user
                .as_deref(),
            Some("deploy")
        );
        assert!(hosts.settings_for("secret.example.com").is_none());
        assert!(hosts.settings_for("elsewhere.net").is_none());
    }

    #[test]
    fn an_unmatched_alias_has_no_settings() {
        let hosts: Hosts = from_str(
            "Host web
	HostName web.example.com",
        )
        .unwrap();
        assert!(hosts.settings_for("other").is_none());
    }

    #[test]
    fn realistic_config_parses() {
        let hosts: Hosts = from_str(REALISTIC_CONFIG).unwrap();
        assert_eq!(hosts.0.len(), 2);

        assert_eq!(hosts.0[0].name, "mc_server");
        assert_eq!(hosts.0[0].host_name.as_deref(), Some("141.148.218.223"));
        assert_eq!(hosts.0[0].user.as_deref(), Some("opc"));
        assert_eq!(hosts.0[0].port, Some(22));

        assert_eq!(hosts.0[1].name, "git_server");
        assert_eq!(hosts.0[1].host_name.as_deref(), Some("github.com"));
        assert_eq!(hosts.0[1].user.as_deref(), Some("git"));
        assert_eq!(hosts.0[1].port, Some(2222));
    }

    #[test]
    fn test_de_tokens_host() {
        // Note: The tokens reflect the internal view where "Host" becomes a map key
        let host = Host {
            name: "mc_server".to_string(),
            host_name: Some("141.148.218.223".to_string()),
            user: Some("opc".to_string()),
            identity_file: Some("~/Downloads/ssh-key-2024-06-13.key".to_string()),
            port: Some(22),
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
                Token::Some,
                Token::Str("141.148.218.223"),
                Token::Str("User"),
                Token::Some,
                Token::Str("opc"),
                Token::Str("IdentityFile"),
                Token::Some,
                Token::Str("~/Downloads/ssh-key-2024-06-13.key"),
                Token::Str("Port"),
                Token::Some,
                Token::U16(22),
                Token::StructEnd,
            ],
        );
    }
}
