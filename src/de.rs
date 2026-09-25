use nom::IResult;
use serde::de::{EnumAccess, IntoDeserializer, VariantAccess};
use serde::Deserialize;

use crate::error::{Error, ErrorCode, Result};
use crate::parser::*;

/// A container for deserializing Rust values from SJSON.
pub struct Deserializer<'de> {
    input: Span<'de>,
    is_top_level: bool,
}

impl<'de> Deserializer<'de> {
    #![allow(clippy::should_implement_trait)]
    pub(crate) fn from_str(input: &'de str) -> Self {
        Self {
            input: Span::from(input),
            is_top_level: true,
        }
    }

    fn parse(&mut self, f: &dyn Fn(Span) -> IResult<Span, Token>) -> Result<Token> {
        f(self.input)
            .map(|(span, token)| {
                self.input = span;
                token
            })
            .map_err(|err| self.error(ErrorCode::Message(err.to_string())))
    }

    fn next_token(&mut self) -> Result<Token> {
        match parse_next_token(self.input) {
            Ok((span, token)) => {
                self.input = span;
                Ok(token)
            }
            Err(err) => Err(self.error(ErrorCode::Message(err.to_string()))),
        }
    }

    fn peek_token(&mut self) -> Result<Token> {
        match parse_next_token(self.input) {
            Ok((_, token)) => Ok(token),
            Err(err) => Err(self.error(ErrorCode::Message(err.to_string()))),
        }
    }

    fn error(&self, code: ErrorCode) -> Error {
        Error::new(
            code,
            self.input.location_line(),
            self.input.get_utf8_column(),
            Some(self.input.fragment().to_string()),
        )
    }

    fn error_with_token(&self, code: ErrorCode, token: Token) -> Error {
        Error::with_token(
            code,
            self.input.location_line(),
            self.input.get_utf8_column(),
            Some(self.input.fragment().to_string()),
            token,
        )
    }
}

/// Deserializes an SJSON string to a Rust value.
#[inline]
pub fn from_str<'a, T>(input: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut de = Deserializer::from_str(input);
    let t = T::deserialize(&mut de)?;
    if de.input.is_empty() || parse_trailing_characters(de.input).is_ok() {
        Ok(t)
    } else {
        Err(de.error(ErrorCode::TrailingCharacters))
    }
}

impl<'de> serde::de::Deserializer<'de> for &mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.is_top_level {
            return Err(self.error(ErrorCode::ExpectedTopLevelObject));
        }

        match self.peek_token()? {
            Token::Boolean(_) => self.deserialize_bool(visitor),
            Token::Float(_) => self.deserialize_f64(visitor),
            Token::Integer(_) => self.deserialize_i64(visitor),
            Token::Null => self.deserialize_unit(visitor),
            Token::String(_) => self.deserialize_str(visitor),
            Token::ArrayStart => self.deserialize_seq(visitor),
            Token::ObjectStart => self.deserialize_map(visitor),
            token => Err(self.error_with_token(ErrorCode::ExpectedValue, token)),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.is_top_level {
            return Err(self.error(ErrorCode::ExpectedTopLevelObject));
        }

        if let Ok(Token::Boolean(val)) = self.parse(&parse_bool) {
            visitor.visit_bool(val)
        } else {
            Err(self.error(ErrorCode::ExpectedBoolean))
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.is_top_level {
            return Err(self.error(ErrorCode::ExpectedTopLevelObject));
        }

        if let Ok(Token::Integer(val)) = self.parse(&parse_integer) {
            visitor.visit_i64(val)
        } else {
            Err(self.error(ErrorCode::ExpectedInteger))
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.is_top_level {
            return Err(self.error(ErrorCode::ExpectedTopLevelObject));
        }

        if let Ok(Token::Float(val)) = self.parse(&parse_float) {
            visitor.visit_f64(val)
        } else {
            Err(self.error(ErrorCode::ExpectedFloat))
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.is_top_level {
            return Err(self.error(ErrorCode::ExpectedTopLevelObject));
        }

        if let Ok(Token::String(val)) = self.parse(&parse_string) {
            visitor.visit_str(&val)
        } else {
            Err(self.error(ErrorCode::ExpectedString))
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.is_top_level {
            return Err(self.error(ErrorCode::ExpectedTopLevelObject));
        }

        if self.peek_token()? == Token::Null {
            let _ = self.next_token()?;
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.is_top_level {
            return Err(self.error(ErrorCode::ExpectedTopLevelObject));
        }

        if let Ok(Token::Null) = self.parse(&parse_null) {
            visitor.visit_unit()
        } else {
            Err(self.error(ErrorCode::ExpectedNull))
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.is_top_level {
            return Err(self.error(ErrorCode::ExpectedTopLevelObject));
        }

        if self.next_token()? != Token::ArrayStart {
            return Err(self.error(ErrorCode::ExpectedArray));
        }

        let value = visitor.visit_seq(Separated::new(self))?;

        if self.next_token()? == Token::ArrayEnd {
            Ok(value)
        } else {
            Err(self.error(ErrorCode::ExpectedArrayEnd))
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.is_top_level {
            self.is_top_level = false;

            visitor.visit_map(Separated::new(self))
        } else {
            if self.next_token()? != Token::ObjectStart {
                return Err(self.error(ErrorCode::ExpectedMap));
            }

            let value = visitor.visit_map(Separated::new(self))?;
            if self.next_token()? == Token::ObjectEnd {
                Ok(value)
            } else {
                Err(self.error(ErrorCode::ExpectedMapEnd))
            }
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.next_token()? {
            Token::String(val) => visitor.visit_enum(val.into_deserializer()),
            Token::ObjectStart => {
                let value = visitor.visit_enum(Enum::new(self))?;

                if self.next_token()? == Token::ObjectEnd {
                    Ok(value)
                } else {
                    Err(self.error(ErrorCode::ExpectedMapEnd))
                }
            }
            _ => Err(self.error(ErrorCode::ExpectedEnum)),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        // A key is either a bare word or a quoted string. A derived struct reads
        // its field names through here, and the toolchain's own files quote the
        // keys of some tables (`"macros": [...]`), so both spellings are a key.
        if let Ok(Token::String(val)) = self.parse(&parse_identifier) {
            visitor.visit_str(&val)
        } else if let Ok(Token::String(val)) = self.parse(&parse_string) {
            visitor.visit_str(&val)
        } else {
            Err(self.error(ErrorCode::ExpectedString))
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct Separated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    first: bool,
}

impl<'a, 'de: 'a> Separated<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de, first: true }
    }
}

impl<'de> serde::de::SeqAccess<'de> for Separated<'_, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        if self.de.peek_token()? == Token::ArrayEnd {
            return Ok(None);
        }

        if !self.first && self.de.parse(&parse_separator)? != Token::Separator {
            return Err(self.de.error(ErrorCode::ExpectedArraySeparator));
        }

        self.first = false;

        // TODO: Shouldn't I check that this is a valid value?
        seed.deserialize(&mut *self.de).map(Some)
    }
}

impl<'de> serde::de::MapAccess<'de> for Separated<'_, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        if matches!(self.de.peek_token()?, Token::ObjectEnd | Token::Eof) {
            return Ok(None);
        }

        if !self.first && self.de.parse(&parse_separator)? != Token::Separator {
            return Err(self.de.error(ErrorCode::ExpectedMapSeparator));
        }

        self.first = false;

        // TODO: Shouldn't I check that this is a valid identifier?
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        if self.de.next_token()? != Token::Equals {
            return Err(self.de.error(ErrorCode::ExpectedMapEquals));
        }

        // TODO: Shouldn't I check that this is a valid value?
        seed.deserialize(&mut *self.de)
    }
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de }
    }
}

impl<'de> EnumAccess<'de> for Enum<'_, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let val = seed.deserialize(&mut *self.de)?;

        if self.de.next_token()? == Token::Equals {
            Ok((val, self))
        } else {
            Err(self.de.error(ErrorCode::ExpectedMapEquals))
        }
    }
}

impl<'de> VariantAccess<'de> for Enum<'_, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Err(self.de.error(ErrorCode::ExpectedString))
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        serde::Deserializer::deserialize_seq(self.de, visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        serde::Deserializer::deserialize_map(self.de, visitor)
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::error::{Error, ErrorCode};
    use crate::from_str;

    macro_rules! assert_value_ok {
        ($type:ty, $json:expr) => {
            assert_value_ok!($type, Default::default(), $json)
        };
        ($type:ty, $expected:expr, $json:expr) => {{
            #[derive(Debug, serde::Deserialize, PartialEq)]
            struct Value {
                value: $type,
            }

            let expected = Value { value: $expected };

            let json = format!("value = {}", $json);
            let actual = from_str::<Value>(&json).unwrap();
            assert_eq!(actual, expected);
        }};
    }

    macro_rules! assert_ok {
        ($type:ty, $expected:expr, $json:expr) => {{
            let actual = from_str::<$type>($json).unwrap();
            assert_eq!(actual, $expected);
        }};
    }

    macro_rules! assert_value_err {
        ($type:ty, $expected:expr, $json:expr) => {{
            #[derive(Debug, serde::Deserialize, PartialEq)]
            struct Value {
                value: $type,
            }

            let json = format!("value = {}", $json);
            let actual = from_str::<Value>(&json);
            assert_eq!(actual, Err($expected));
        }};
    }

    #[test]
    fn deserialize_null() {
        assert_value_ok!((), "null");

        let err = Error::new(ErrorCode::ExpectedNull, 1, 8, Some(" foo".to_string()));
        assert_value_err!((), err, "foo");
    }

    #[test]
    fn deserialize_bool() {
        assert_value_ok!(bool, true, "true");
        assert_value_ok!(bool, false, "false");

        let err = Error::new(ErrorCode::ExpectedBoolean, 1, 8, Some(" foo".to_string()));
        assert_value_err!(bool, err, "foo");
    }

    #[test]
    fn deserialize_integer() {
        assert_value_ok!(i64, 0, "0");
        assert_value_ok!(i64, -1, "-1");
        assert_value_ok!(i64, i64::MAX, i64::MAX.to_string());
        assert_value_ok!(i64, i64::MIN, i64::MIN.to_string());

        assert_value_ok!(i8, 0, "0");
        assert_value_ok!(i8, 102, "102");
        assert_value_ok!(i8, -102, "-102");
        assert_value_ok!(u8, 102, "102");
        assert_value_ok!(i16, 256, "256");

        let err = Error::new(ErrorCode::ExpectedInteger, 1, 8, Some(" foo".to_string()));
        assert_value_err!(i64, err, "foo");
    }

    #[test]
    fn deserialize_float() {
        assert_value_ok!(f64, 0.0, "0");
        assert_value_ok!(f64, 0.0, "0.0");
        assert_value_ok!(f64, -1.0, "-1");
        assert_value_ok!(f64, -1.0, "-1.0");
        assert_value_ok!(f64, f64::MAX, f64::MAX.to_string());
        assert_value_ok!(f64, f64::MIN, f64::MIN.to_string());
    }

    #[test]
    fn deserialize_vec() {
        assert_value_ok!(Vec<u64>, vec![1, 2, 3], "[1, 2, 3]");
        assert_value_ok!(
            Vec<u64>,
            vec![1, 2, 3],
            "\
[
    1
    2
    3
]"
        );
    }

    #[test]
    fn deserialize_enum() {
        #[derive(Debug, serde::Deserialize, PartialEq)]
        enum Animal {
            Mouse,
            Dog { name: String },
            Cat(u64),
        }

        assert_value_ok!(Animal, Animal::Mouse, "Mouse");
        assert_value_ok!(Animal, Animal::Cat(9), "{ Cat = 9 }");
        assert_value_ok!(
            Animal,
            Animal::Dog {
                name: String::from("Buddy")
            },
            "{ Dog = { name = Buddy }}"
        );
    }

    // Checks the example from
    // https://help.autodesk.com/view/Stingray/ENU/?guid=__stingray_help_managing_content_sjson_html
    #[test]
    fn deserialize_stingray_example() {
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct Win32Settings {
            query_performance_counter_affinity_mask: u64,
        }

        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct Settings {
            boot_script: String,
            console_port: u16,
            win32: Win32Settings,
            render_config: PathBuf,
        }

        let expected = Settings {
            boot_script: String::from("boot"),
            console_port: 14030,
            win32: Win32Settings {
                query_performance_counter_affinity_mask: 0,
            },
            render_config: PathBuf::from("core/rendering/renderer"),
        };

        let json = r#"
// The script that should be started when the application runs.
boot_script = "boot"

// The port on which the console server runs.
console_port = 14030

// Settings for the win32 platform
win32 = {
    /* Sets the affinity mask for
       QueryPerformanceCounter() */
    query_performance_counter_affinity_mask = 0
}

render_config = "core/rendering/renderer"
"#;

        assert_ok!(Settings, expected, json);
    }

    #[test]
    fn deserialize_missing_top_level_struct() {
        let json = "0";
        let err = Error::new(
            ErrorCode::ExpectedTopLevelObject,
            1,
            1,
            Some(json.to_string()),
        );
        let actual = from_str::<i64>(json);
        assert_eq!(actual, Err(err));

        let json = "1.23";
        let err = Error::new(
            ErrorCode::ExpectedTopLevelObject,
            1,
            1,
            Some(json.to_string()),
        );
        let actual = from_str::<f64>(json);
        assert_eq!(actual, Err(err));

        let json = "true";
        let err = Error::new(
            ErrorCode::ExpectedTopLevelObject,
            1,
            1,
            Some(json.to_string()),
        );
        let actual = from_str::<bool>(json);
        assert_eq!(actual, Err(err));

        let json = "null";
        let err = Error::new(
            ErrorCode::ExpectedTopLevelObject,
            1,
            1,
            Some(json.to_string()),
        );
        let actual = from_str::<()>(json);
        assert_eq!(actual, Err(err));
    }

    #[test]
    fn deserialize_array() {
        #[derive(Debug, Default, serde::Deserialize, PartialEq)]
        struct Data {
            array: Vec<String>,
        }

        let expected = Data {
            array: vec![String::from("foo")],
        };

        let sjson = r#"
array = [
    "foo"
]
"#;
        assert_ok!(Data, expected, sjson);
    }

    // Regression test for #1 (https://git.sclu1034.dev/lucas/serde_sjson/issues/1)
    #[test]
    fn deserialize_dtmt_config() {
        #[derive(Debug, Default, serde::Deserialize, PartialEq)]
        struct DtmtConfig {
            name: String,
            #[serde(default)]
            description: String,
            version: Option<String>,
        }

        let sjson = r#"
name = "test-mod"
description = "A dummy project to test things with"
version = "0.1.0"

packages = [
    "packages/test-mod"
]
"#;

        let expected = DtmtConfig {
            name: String::from("test-mod"),
            description: String::from("A dummy project to test things with"),
            version: Some(String::from("0.1.0")),
        };

        assert_ok!(DtmtConfig, expected, sjson);
    }
}
