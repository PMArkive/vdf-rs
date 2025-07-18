//! Serialize Rust types to VDF text

use keyvalues_parser::Vdf;
use serde::{ser, Serialize};

use std::io::Write;

use crate::{
    error::{Error, Result},
    tokens::{NaiveToken, NaiveTokenStream},
};

/// The struct for serializing Rust values into VDF text
///
/// This typically doesn't need to be invoked directly when [`to_writer()`] and
/// [`to_writer_with_key()`] can be used instead
#[derive(Default)]
pub struct Serializer {
    tokens: NaiveTokenStream,
}

impl Serializer {
    /// Creates a new VDF serializer
    pub fn new() -> Self {
        Self::default()
    }
}

/// Serialize the `value` into an IO stream of VDF text
///
/// # Errors
///
/// This will return an error if the input can't be represented with valid VDF
pub fn to_writer<W, T>(writer: &mut W, value: &T) -> Result<()>
where
    W: Write,
    T: Serialize,
{
    _to_writer(writer, value, None)
}

/// Serialize the `value` into an IO stream of VDF text with a custom top level VDF key
///
/// # Errors
///
/// This will return an error if the input can't be represented with valid VDF
pub fn to_writer_with_key<W, T>(writer: &mut W, value: &T, key: &str) -> Result<()>
where
    W: Write,
    T: Serialize,
{
    _to_writer(writer, value, Some(key))
}

// Serialization process goes as follows:
// value: &T
// -> NaiveTokenStream
// -> Vdf (fails on invalid VDF structure like nested sequences)
// -> Formatted
// Which is a bit of a long-winded process just to serialize some text, but it comes with
// validation (NaiveTokenStream -> Vdf) and reuses portions from the parser (Vdf -> Formatted)
fn _to_writer<W, T>(writer: &mut W, value: &T, maybe_key: Option<&str>) -> Result<()>
where
    W: Write,
    T: Serialize,
{
    let mut serializer = Serializer::new();
    value.serialize(&mut serializer)?;

    if let Some(key) = maybe_key {
        match serializer.tokens.first() {
            // Replace the old key
            Some(NaiveToken::Str(_old_key)) => {
                serializer.tokens[0] = NaiveToken::Str(key.to_owned());
            }
            // Push on the key
            Some(_) => serializer.tokens.insert(0, NaiveToken::Str(key.to_owned())),
            None => {}
        }
    }

    let vdf = Vdf::try_from(&serializer.tokens)?;
    write!(writer, "{vdf}")?;

    Ok(())
}

/// Attempts to serialize some input to VDF text
///
/// # Errors
///
/// This will return an error if the input can't be represented with valid VDF
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let mut buffer = Vec::new();
    to_writer(&mut buffer, value)?;
    let s = String::from_utf8(buffer).expect("Input was all valid UTF-8");

    Ok(s)
}

/// Attempts to serialize some input to VDF text with a custom top level VDF key
///
/// # Errors
///
/// This will return an error if the input can't be represented with valid VDF
pub fn to_string_with_key<T>(value: &T, key: &str) -> Result<String>
where
    T: Serialize,
{
    let mut buffer = Vec::new();
    to_writer_with_key(&mut buffer, value, key)?;
    let s = String::from_utf8(buffer).expect("Input was all valid UTF-8");

    Ok(s)
}

macro_rules! forward_serialize_as_str {
    ( $( ( $method:ident, $ty:ty ) ),* $(,)? ) => {
        $(
            fn $method(self, v: $ty) -> Result<()> {
                self.serialize_str(&v.to_string())
            }
        )*
    }
}

impl ser::Serializer for &mut Serializer {
    type Ok = ();

    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    forward_serialize_as_str!(
        (serialize_i8, i8),
        (serialize_i16, i16),
        (serialize_i32, i32),
        (serialize_i64, i64),
        (serialize_i128, i128),
        (serialize_u8, u8),
        (serialize_u16, u16),
        (serialize_u32, u32),
        (serialize_u64, u64),
        (serialize_u128, u128),
        (serialize_char, char),
    );

    fn serialize_str(self, v: &str) -> Result<()> {
        self.tokens.push(NaiveToken::str(v));
        Ok(())
    }

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.serialize_i8(v as i8)
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        if v.is_finite() {
            self.serialize_str(&v.to_string())
        } else {
            Err(Error::NonFiniteFloat(v))
        }
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        // TODO: include this and empty vecs and nested Option<Vec> in potential pitfalls
        // TODO: look into this more, might be the other way around if the wiki is wrong
        // Note: I believe floats in VDF are considered f32 so even when you use an f64 it will get
        // converted to an f32 when serialized
        self.serialize_f32(v as f32)
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        Err(Error::Unsupported("Bytes"))
    }

    fn serialize_none(self) -> Result<()> {
        self.tokens.push(NaiveToken::Null);
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        // Just serializes the contained value
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        Err(Error::Unsupported("Unit Type"))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Err(Error::Unsupported("Unit Struct"))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        // Just pass the variant name for unit variant enums
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        // Just a wrapper over the contained value
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Unsupported("Enum Newtype Variant"))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.tokens.push(NaiveToken::SeqBegin);
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        self.tokens.push(NaiveToken::ObjBegin);
        Ok(self)
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        // The top level key is the name of the struct
        if self.tokens.is_empty() {
            self.serialize_str(name)?;
        }

        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(self)
    }
}

impl ser::SerializeSeq for &mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.tokens.push(NaiveToken::SeqEnd);
        Ok(())
    }
}

impl ser::SerializeTuple for &mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.tokens.push(NaiveToken::SeqEnd);
        Ok(())
    }
}

impl ser::SerializeTupleStruct for &mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.tokens.push(NaiveToken::SeqEnd);
        Ok(())
    }
}

impl ser::SerializeTupleVariant for &mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Unsupported("Enum Tuple Variant"))
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported("Enum Tuple Variant"))
    }
}

impl ser::SerializeMap for &mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.tokens.push(NaiveToken::ObjEnd);
        Ok(())
    }
}

impl ser::SerializeStruct for &mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(&mut **self)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.tokens.push(NaiveToken::ObjEnd);
        Ok(())
    }
}

impl ser::SerializeStructVariant for &mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Unsupported("Enum Struct Variant"))
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported("Enum Struct Variant"))
    }
}
