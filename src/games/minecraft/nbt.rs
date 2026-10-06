//! Java Edition NBT, the format of player data files: big-endian named tags with modified UTF-8
//! strings, gzip-compressed on disk. Writing what was read gives back the same bytes, so edits
//! change nothing but what they touch. Items are shown as SNBT (`Display`).
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use std::{
    fmt::{self, Write as _},
    io::{Read, Write as _},
};

const END: u8 = 0;
const BYTE: u8 = 1;
const SHORT: u8 = 2;
const INT: u8 = 3;
const LONG: u8 = 4;
const FLOAT: u8 = 5;
const DOUBLE: u8 = 6;
const BYTE_ARRAY: u8 = 7;
const STRING: u8 = 8;
const LIST: u8 = 9;
const COMPOUND: u8 = 10;
const INT_ARRAY: u8 = 11;
const LONG_ARRAY: u8 = 12;

/// Nesting limit, the one Minecraft enforces.
const MAX_DEPTH: usize = 512;
/// Decompressed size limit; player data is a few kilobytes to a few megabytes.
const MAX_DECOMPRESSED: u64 = 64 << 20;

#[derive(Clone, Debug, PartialEq)]
pub enum Tag {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(List),
    Compound(Compound),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

/// A list tag: elements of one type, which is kept for empty lists too.
#[derive(Clone, Debug, PartialEq)]
pub struct List {
    kind: u8,
    items: Vec<Tag>,
}

/// A compound tag: named tags in file order.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Compound(Vec<(String, Tag)>);

impl Tag {
    fn kind(&self) -> u8 {
        match self {
            Self::Byte(_) => BYTE,
            Self::Short(_) => SHORT,
            Self::Int(_) => INT,
            Self::Long(_) => LONG,
            Self::Float(_) => FLOAT,
            Self::Double(_) => DOUBLE,
            Self::ByteArray(_) => BYTE_ARRAY,
            Self::String(_) => STRING,
            Self::List(_) => LIST,
            Self::Compound(_) => COMPOUND,
            Self::IntArray(_) => INT_ARRAY,
            Self::LongArray(_) => LONG_ARRAY,
        }
    }

    /// Any integer tag.
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Self::Byte(value) => Some(value.into()),
            Self::Short(value) => Some(value.into()),
            Self::Int(value) => Some(value.into()),
            Self::Long(value) => Some(value),
            _ => None,
        }
    }

    /// Any number tag.
    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Self::Float(value) => Some(value.into()),
            Self::Double(value) => Some(value),
            _ => self.as_i64().map(|value| value as f64),
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&List> {
        match self {
            Self::List(list) => Some(list),
            _ => None,
        }
    }

    pub fn as_compound(&self) -> Option<&Compound> {
        match self {
            Self::Compound(compound) => Some(compound),
            _ => None,
        }
    }
}

impl List {
    pub fn items(&self) -> &[Tag] {
        &self.items
    }

    /// Keeps the elements `keep` accepts; how many were removed. An emptied list gets the end
    /// type, as Minecraft writes empty lists.
    pub fn retain(&mut self, keep: impl FnMut(&Tag) -> bool) -> usize {
        let before = self.items.len();
        self.items.retain(keep);
        if self.items.is_empty() {
            self.kind = END;
        }
        before - self.items.len()
    }
}

/// A list of `items`, which must all have the type of the first (the writer refuses mixed
/// lists).
impl From<Vec<Tag>> for List {
    fn from(items: Vec<Tag>) -> Self {
        Self {
            kind: items.first().map_or(END, Tag::kind),
            items,
        }
    }
}

impl Compound {
    pub fn get(&self, key: &str) -> Option<&Tag> {
        self.0
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, tag)| tag)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Tag> {
        self.0
            .iter_mut()
            .find(|(name, _)| name == key)
            .map(|(_, tag)| tag)
    }

    /// Sets `key`, in place when it exists (keeping the order), else at the end.
    pub fn insert(&mut self, key: &str, tag: Tag) {
        match self.get_mut(key) {
            Some(existing) => *existing = tag,
            None => self.0.push((key.to_string(), tag)),
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<Tag> {
        let index = self.0.iter().position(|(name, _)| name == key)?;
        Some(self.0.remove(index).1)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Tag)> {
        self.0.iter().map(|(name, tag)| (name.as_str(), tag))
    }
}

/// Decompresses a gzip file.
pub fn gunzip(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    GzDecoder::new(data)
        .take(MAX_DECOMPRESSED + 1)
        .read_to_end(&mut output)
        .map_err(|err| format!("not gzip data ({err})"))?;
    if output.len() as u64 > MAX_DECOMPRESSED {
        return Err(format!(
            "more than {} MiB uncompressed",
            MAX_DECOMPRESSED >> 20
        ));
    }
    Ok(output)
}

pub fn gzip(data: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::with_capacity(data.len() / 2), Compression::default());
    encoder
        .write_all(data)
        .expect("writing to memory cannot fail");
    encoder.finish().expect("writing to memory cannot fail")
}

/// The name and compound of a file's root tag (bytes after it are ignored, as Minecraft does).
pub fn read(data: &[u8]) -> Result<(String, Compound), String> {
    let mut reader = Reader { data, position: 0 };
    match reader.u8()? {
        COMPOUND => {}
        kind => return Err(format!("the root tag is not a compound (type {kind})")),
    }
    let name = reader.string()?;
    let root = reader.compound(0)?;
    Ok((name, root))
}

/// A file with a root compound named `name`.
pub fn write(name: &str, root: &Compound) -> Result<Vec<u8>, String> {
    let mut output = vec![COMPOUND];
    write_string(&mut output, name)?;
    write_compound(&mut output, root)?;
    Ok(output)
}

struct Reader<'a> {
    data: &'a [u8],
    position: usize,
}

/// The smallest payload of a tag type, to refuse lengths the remaining bytes cannot hold
/// before allocating for them.
fn min_size(kind: u8) -> usize {
    match kind {
        SHORT | STRING => 2,
        INT | FLOAT | BYTE_ARRAY | INT_ARRAY | LONG_ARRAY => 4,
        LIST => 5,
        LONG | DOUBLE => 8,
        _ => 1,
    }
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self
            .position
            .checked_add(count)
            .filter(|&end| end <= self.data.len())
            .ok_or("unexpected end of data")?;
        let bytes = &self.data[self.position..end];
        self.position = end;
        Ok(bytes)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], String> {
        Ok(self.take(N)?.try_into().expect("take returns N bytes"))
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    /// An array or list length, refused when the rest of the data cannot hold that many
    /// elements of `element_size` bytes.
    fn length(&mut self, element_size: usize) -> Result<usize, String> {
        let length = i32::from_be_bytes(self.array()?);
        let length = usize::try_from(length).map_err(|_| format!("negative length {length}"))?;
        if length > (self.data.len() - self.position) / element_size {
            return Err(format!("length {length} beyond the end of data"));
        }
        Ok(length)
    }

    fn string(&mut self) -> Result<String, String> {
        let length = u16::from_be_bytes(self.array()?);
        decode_mutf8(self.take(length.into())?)
    }

    fn payload(&mut self, kind: u8, depth: usize) -> Result<Tag, String> {
        Ok(match kind {
            BYTE => Tag::Byte(i8::from_be_bytes(self.array()?)),
            SHORT => Tag::Short(i16::from_be_bytes(self.array()?)),
            INT => Tag::Int(i32::from_be_bytes(self.array()?)),
            LONG => Tag::Long(i64::from_be_bytes(self.array()?)),
            FLOAT => Tag::Float(f32::from_be_bytes(self.array()?)),
            DOUBLE => Tag::Double(f64::from_be_bytes(self.array()?)),
            BYTE_ARRAY => {
                let length = self.length(1)?;
                Tag::ByteArray(self.take(length)?.iter().map(|&byte| byte as i8).collect())
            }
            STRING => Tag::String(self.string()?),
            LIST => Tag::List(self.list(depth + 1)?),
            COMPOUND => Tag::Compound(self.compound(depth + 1)?),
            INT_ARRAY => {
                let length = self.length(4)?;
                Tag::IntArray(
                    self.take(length * 4)?
                        .chunks_exact(4)
                        .map(|chunk| i32::from_be_bytes(chunk.try_into().expect("4 bytes")))
                        .collect(),
                )
            }
            LONG_ARRAY => {
                let length = self.length(8)?;
                Tag::LongArray(
                    self.take(length * 8)?
                        .chunks_exact(8)
                        .map(|chunk| i64::from_be_bytes(chunk.try_into().expect("8 bytes")))
                        .collect(),
                )
            }
            kind => return Err(format!("unknown tag type {kind}")),
        })
    }

    fn list(&mut self, depth: usize) -> Result<List, String> {
        if depth > MAX_DEPTH {
            return Err("nested too deeply".into());
        }
        let kind = self.u8()?;
        let length = self.length(min_size(kind))?;
        if kind == END && length > 0 {
            return Err("a list of end tags is not empty".into());
        }
        let items = (0..length)
            .map(|_| self.payload(kind, depth))
            .collect::<Result<_, _>>()?;
        Ok(List { kind, items })
    }

    fn compound(&mut self, depth: usize) -> Result<Compound, String> {
        if depth > MAX_DEPTH {
            return Err("nested too deeply".into());
        }
        let mut entries = Vec::new();
        loop {
            let kind = self.u8()?;
            if kind == END {
                return Ok(Compound(entries));
            }
            let name = self.string()?;
            entries.push((name, self.payload(kind, depth)?));
        }
    }
}

fn write_length(output: &mut Vec<u8>, length: usize) -> Result<(), String> {
    let length = i32::try_from(length).map_err(|_| "too many elements".to_string())?;
    output.extend_from_slice(&length.to_be_bytes());
    Ok(())
}

fn write_string(output: &mut Vec<u8>, value: &str) -> Result<(), String> {
    let start = output.len();
    output.extend_from_slice(&[0, 0]);
    encode_mutf8(value, output);
    let length = u16::try_from(output.len() - start - 2)
        .map_err(|_| "a string is longer than 65535 bytes".to_string())?;
    output[start..start + 2].copy_from_slice(&length.to_be_bytes());
    Ok(())
}

fn write_payload(output: &mut Vec<u8>, tag: &Tag) -> Result<(), String> {
    match tag {
        Tag::Byte(value) => output.extend_from_slice(&value.to_be_bytes()),
        Tag::Short(value) => output.extend_from_slice(&value.to_be_bytes()),
        Tag::Int(value) => output.extend_from_slice(&value.to_be_bytes()),
        Tag::Long(value) => output.extend_from_slice(&value.to_be_bytes()),
        Tag::Float(value) => output.extend_from_slice(&value.to_be_bytes()),
        Tag::Double(value) => output.extend_from_slice(&value.to_be_bytes()),
        Tag::ByteArray(values) => {
            write_length(output, values.len())?;
            output.extend(values.iter().map(|&value| value as u8));
        }
        Tag::String(value) => write_string(output, value)?,
        Tag::List(list) => {
            if list.items.iter().any(|item| item.kind() != list.kind) {
                return Err("a list mixes tag types".into());
            }
            output.push(list.kind);
            write_length(output, list.items.len())?;
            for item in &list.items {
                write_payload(output, item)?;
            }
        }
        Tag::Compound(compound) => write_compound(output, compound)?,
        Tag::IntArray(values) => {
            write_length(output, values.len())?;
            for value in values {
                output.extend_from_slice(&value.to_be_bytes());
            }
        }
        Tag::LongArray(values) => {
            write_length(output, values.len())?;
            for value in values {
                output.extend_from_slice(&value.to_be_bytes());
            }
        }
    }
    Ok(())
}

fn write_compound(output: &mut Vec<u8>, compound: &Compound) -> Result<(), String> {
    for (name, tag) in &compound.0 {
        output.push(tag.kind());
        write_string(output, name)?;
        write_payload(output, tag)?;
    }
    output.push(END);
    Ok(())
}

/// Java's modified UTF-8: UTF-16 code units, NUL as two bytes, no four-byte sequences
/// (supplementary characters are two encoded surrogates).
fn decode_mutf8(bytes: &[u8]) -> Result<String, String> {
    if bytes.is_ascii() {
        return String::from_utf8(bytes.to_vec()).map_err(|err| err.to_string());
    }
    fn continuation(byte: Option<u8>) -> Result<u16, String> {
        match byte {
            Some(byte) if byte & 0xC0 == 0x80 => Ok(u16::from(byte & 0x3F)),
            _ => Err("invalid modified UTF-8".into()),
        }
    }
    let mut units = Vec::with_capacity(bytes.len());
    let mut bytes = bytes.iter().copied();
    while let Some(first) = bytes.next() {
        let unit = match first {
            0x00..=0x7F => u16::from(first),
            0xC0..=0xDF => (u16::from(first & 0x1F) << 6) | continuation(bytes.next())?,
            0xE0..=0xEF => {
                let high = continuation(bytes.next())?;
                (u16::from(first & 0x0F) << 12) | (high << 6) | continuation(bytes.next())?
            }
            _ => return Err("invalid modified UTF-8".into()),
        };
        units.push(unit);
    }
    String::from_utf16(&units).map_err(|_| "a string has an unpaired surrogate".into())
}

fn encode_mutf8(value: &str, output: &mut Vec<u8>) {
    for unit in value.encode_utf16() {
        match unit {
            0x0001..=0x007F => output.push(unit as u8),
            0 | 0x0080..=0x07FF => {
                output.extend_from_slice(&[0xC0 | (unit >> 6) as u8, 0x80 | (unit & 0x3F) as u8])
            }
            _ => output.extend_from_slice(&[
                0xE0 | (unit >> 12) as u8,
                0x80 | ((unit >> 6) & 0x3F) as u8,
                0x80 | (unit & 0x3F) as u8,
            ]),
        }
    }
}

/// `value` quoted the way Minecraft writes SNBT strings: in double quotes unless the first
/// quote character inside is one, backslashes and the chosen quote escaped.
fn write_quoted(f: &mut fmt::Formatter<'_>, value: &str) -> fmt::Result {
    let quote = match value.chars().find(|&c| c == '"' || c == '\'') {
        Some('"') => '\'',
        _ => '"',
    };
    f.write_char(quote)?;
    for c in value.chars() {
        if c == '\\' || c == quote {
            f.write_char('\\')?;
        }
        f.write_char(c)?;
    }
    f.write_char(quote)
}

fn write_sequence<T: fmt::Display>(
    f: &mut fmt::Formatter<'_>,
    open: &str,
    values: impl IntoIterator<Item = T>,
    suffix: &str,
) -> fmt::Result {
    f.write_str(open)?;
    for (index, value) in values.into_iter().enumerate() {
        if index > 0 {
            f.write_char(',')?;
        }
        write!(f, "{value}{suffix}")?;
    }
    f.write_char(']')
}

/// SNBT, as Minecraft prints it (keys in file order).
impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Byte(value) => write!(f, "{value}b"),
            Self::Short(value) => write!(f, "{value}s"),
            Self::Int(value) => write!(f, "{value}"),
            Self::Long(value) => write!(f, "{value}L"),
            Self::Float(value) => write!(f, "{value:?}f"),
            Self::Double(value) => write!(f, "{value:?}d"),
            Self::ByteArray(values) => write_sequence(f, "[B;", values, "B"),
            Self::String(value) => write_quoted(f, value),
            Self::List(list) => write_sequence(f, "[", &list.items, ""),
            Self::Compound(compound) => fmt::Display::fmt(compound, f),
            Self::IntArray(values) => write_sequence(f, "[I;", values, ""),
            Self::LongArray(values) => write_sequence(f, "[L;", values, "L"),
        }
    }
}

impl fmt::Display for Compound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('{')?;
        for (index, (name, tag)) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_char(',')?;
            }
            let plain = !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+'));
            if plain {
                f.write_str(name)?;
            } else {
                write_quoted(f, name)?;
            }
            write!(f, ":{tag}")?;
        }
        f.write_char('}')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `hello_world.nbt` from the NBT specification.
    const HELLO_WORLD: &[u8] = b"\x0a\x00\x0bhello world\x08\x00\x04name\x00\x09Bananrama\x00";

    fn compound(entries: Vec<(&str, Tag)>) -> Compound {
        let mut compound = Compound::default();
        for (key, tag) in entries {
            compound.insert(key, tag);
        }
        compound
    }

    fn string(value: &str) -> Tag {
        Tag::String(value.into())
    }

    #[test]
    fn reads_and_writes_the_spec_example() {
        let (name, root) = read(HELLO_WORLD).unwrap();
        assert_eq!(name, "hello world");
        assert_eq!(root.get("name"), Some(&string("Bananrama")));
        assert_eq!(write(&name, &root).unwrap(), HELLO_WORLD);
    }

    #[test]
    fn round_trips_every_tag_type() {
        let nested = compound(vec![
            ("byte", Tag::Byte(-128)),
            ("short", Tag::Short(-32768)),
            ("int", Tag::Int(i32::MIN)),
            ("long", Tag::Long(i64::MAX)),
            ("float", Tag::Float(-0.5)),
            ("nan", Tag::Float(f32::from_bits(0x7FC0_1234))),
            ("double", Tag::Double(1e300)),
            ("bytes", Tag::ByteArray(vec![-1, 0, 127])),
            ("string", string("plain")),
            ("ints", Tag::IntArray(vec![-1, 0, i32::MAX])),
            ("longs", Tag::LongArray(vec![i64::MIN, 0])),
            ("empty bytes", Tag::ByteArray(vec![])),
        ]);
        let root = compound(vec![
            ("nested", Tag::Compound(nested.clone())),
            (
                "lists",
                Tag::List(List::from(vec![
                    Tag::List(List::from(vec![Tag::Short(1), Tag::Short(2)])),
                    Tag::List(List::from(vec![])),
                    Tag::List(List::from(vec![Tag::Compound(nested)])),
                ])),
            ),
            (
                "typed empty list",
                Tag::List(List {
                    kind: INT,
                    items: vec![],
                }),
            ),
            ("", Tag::Compound(Compound::default())),
            ("text", string("a\0b é ࠀ 😀")),
        ]);
        let bytes = write("root", &root).unwrap();
        let (name, read_back) = read(&bytes).unwrap();
        assert_eq!(name, "root");
        assert_eq!(write(&name, &read_back).unwrap(), bytes);
        let Some(Tag::Compound(nested)) = read_back.get("nested") else {
            panic!("expected the nested compound");
        };
        let Some(Tag::Float(nan)) = nested.get("nan") else {
            panic!("expected a float");
        };
        assert_eq!(nan.to_bits(), 0x7FC0_1234);
        assert_eq!(read_back.get("text"), Some(&string("a\0b é ࠀ 😀")));
        assert_eq!(
            read_back.get("typed empty list"),
            Some(&Tag::List(List {
                kind: INT,
                items: vec![]
            }))
        );
    }

    #[test]
    fn encodes_modified_utf8() {
        let mut output = Vec::new();
        encode_mutf8("a\0é😀", &mut output);
        assert_eq!(
            output,
            [
                0x61, 0xC0, 0x80, 0xC3, 0xA9, 0xED, 0xA0, 0xBD, 0xED, 0xB8, 0x80
            ]
        );
        assert_eq!(decode_mutf8(&output).unwrap(), "a\0é😀");
        // Java's reader also takes a raw NUL byte
        assert_eq!(decode_mutf8(b"a\0").unwrap(), "a\0");
        // a lone surrogate (ED A0 BD) and a four-byte UTF-8 sequence are refused
        assert!(decode_mutf8(&[0xED, 0xA0, 0xBD]).is_err());
        assert!(decode_mutf8("😀".as_bytes()).is_err());
        assert!(decode_mutf8(&[0xC3]).is_err());
    }

    #[test]
    fn refuses_broken_data() {
        assert!(read(b"\x08\x00\x00\x00\x00").is_err());
        assert!(read(&HELLO_WORLD[..HELLO_WORLD.len() - 1]).is_err());
        // a byte array claiming more bytes than there are
        assert!(read(b"\x0a\x00\x00\x07\x00\x01a\x7f\xff\xff\xff\x00").is_err());
        // a negative list length
        assert!(read(b"\x0a\x00\x00\x09\x00\x01a\x01\xff\xff\xff\xff\x00").is_err());
        // a non-empty list of end tags
        assert!(read(b"\x0a\x00\x00\x09\x00\x01a\x00\x00\x00\x00\x01\x00").is_err());
        let mut deep = b"\x0a\x00\x00".to_vec();
        for _ in 0..=MAX_DEPTH {
            deep.extend_from_slice(b"\x0a\x00\x00");
        }
        assert!(read(&deep).is_err());
        let mixed = compound(vec![(
            "mixed",
            Tag::List(List::from(vec![Tag::Int(1), Tag::Byte(1)])),
        )]);
        assert!(write("", &mixed).is_err());
    }

    #[test]
    fn gzip_round_trip() {
        let compressed = gzip(HELLO_WORLD);
        assert_eq!(&compressed[..2], [0x1F, 0x8B]);
        assert_eq!(gunzip(&compressed).unwrap(), HELLO_WORLD);
        assert!(gunzip(HELLO_WORLD).is_err());
    }

    #[test]
    fn prints_snbt() {
        let item = compound(vec![
            ("id", string("minecraft:diamond_sword")),
            ("count", Tag::Int(1)),
            ("Damage", Tag::Short(3)),
            ("weight", Tag::Float(1.5)),
            ("speed", Tag::Double(0.1)),
            ("big", Tag::Long(5)),
            ("flag", Tag::Byte(1)),
            ("custom name", string("It's \"sharp\"")),
            ("path", string("a\\b")),
            ("bytes", Tag::ByteArray(vec![1, -2])),
            ("ints", Tag::IntArray(vec![1, 2])),
            ("longs", Tag::LongArray(vec![3])),
            (
                "list",
                Tag::List(List::from(vec![string("x"), string("y")])),
            ),
        ]);
        assert_eq!(
            item.to_string(),
            r#"{id:"minecraft:diamond_sword",count:1,Damage:3s,weight:1.5f,speed:0.1d,big:5L,flag:1b,"custom name":"It's \"sharp\"",path:"a\\b",bytes:[B;1B,-2B],ints:[I;1,2],longs:[L;3L],list:["x","y"]}"#
        );
        assert_eq!(string("say \"hi\"").to_string(), r#"'say "hi"'"#);
    }
}
