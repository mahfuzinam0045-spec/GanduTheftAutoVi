//! `RBF0`: RAGE's binary XML format (UI schemas, tuning/config, rig blobs).
//!
//! Pure parser adapted from Leonida (`docs/rbf0.md`, GPL-3.0-or-later).

use std::fmt;

/// Magic bytes at offset 0 of an RBF0 stream.
pub const MAGIC: &[u8; 4] = b"RBF0";
/// 4-byte prologue present in wide (64-bit int/float) streams.
pub const WIDE_PROLOGUE: [u8; 4] = [0xFB, 0xFF, 0x01, 0x00];

/// A typed attribute or child value in an RBF0 tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Float(f64),
    Vec3([f64; 3]),
    String(String),
}

/// An XML-like element decoded from an RBF0 stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub name: String,
    pub attributes: Vec<(String, Value)>,
    pub children: Vec<Element>,
    pub text: Option<String>,
}

/// Errors returned by [`parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    BadMagic,
    UnexpectedEof,
    InvalidUtf8,
    InvalidDescriptorIndex { idx: u8, count: usize },
    UnknownRecordType(u8),
    TrailingClose,
    UnclosedElement,
    UnexpectedTextOutsideElement,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic => write!(f, "missing RBF0 magic"),
            Self::UnexpectedEof => write!(f, "unexpected end of RBF0 stream"),
            Self::InvalidUtf8 => write!(f, "invalid UTF-8 in RBF0 string or descriptor"),
            Self::InvalidDescriptorIndex { idx, count } => {
                write!(f, "descriptor index {idx} out of range (count {count})")
            }
            Self::UnknownRecordType(t) => write!(f, "unknown RBF0 record type 0x{t:02x}"),
            Self::TrailingClose => write!(f, "unexpected close record"),
            Self::UnclosedElement => write!(f, "unclosed RBF0 element"),
            Self::UnexpectedTextOutsideElement => write!(f, "text record outside element"),
        }
    }
}

impl std::error::Error for Error {}

/// Decodes a complete RBF0 document, returning the root [`Element`] and total
/// bytes consumed.
pub fn parse(data: &[u8]) -> Result<(Element, usize), Error> {
    if data.get(..4) != Some(MAGIC.as_slice()) {
        return Err(Error::BadMagic);
    }
    let (wide, mut pos) = if data.get(4..8) == Some(&WIDE_PROLOGUE) {
        (true, 8)
    } else {
        (false, 4)
    };

    let mut descriptors: Vec<String> = Vec::new();
    let mut stack: Vec<(Element, u16)> = Vec::new();

    loop {
        let idx = read_u8(data, &mut pos)?;
        let kind = read_u8(data, &mut pos)?;

        if idx == 0xFF && kind == 0xFF {
            let (finished, remaining_attrs) = stack.pop().ok_or(Error::TrailingClose)?;
            if remaining_attrs != 0 {
                return Err(Error::UnclosedElement);
            }
            if let Some((parent, _)) = stack.last_mut() {
                parent.children.push(finished);
            } else {
                return Ok((finished, pos));
            }
            continue;
        }

        if idx == 0xFD && kind == 0xFF {
            let len = read_u32(data, &mut pos)? as usize;
            let raw = read_bytes(data, &mut pos, len)?;
            let trimmed = raw.strip_suffix(b"\0").unwrap_or(raw);
            let text = std::str::from_utf8(trimmed)
                .map_err(|_| Error::InvalidUtf8)?
                .to_owned();
            let (open, _) = stack
                .last_mut()
                .ok_or(Error::UnexpectedTextOutsideElement)?;
            open.text = Some(text);
            continue;
        }

        let name = resolve_descriptor(data, &mut pos, &mut descriptors, idx)?;

        if kind == 0x00 {
            let _w0 = read_u16(data, &mut pos)?;
            let _w1 = read_u16(data, &mut pos)?;
            let attr_count = read_u16(data, &mut pos)?;
            stack.push((
                Element {
                    name,
                    attributes: Vec::with_capacity(usize::from(attr_count)),
                    children: Vec::new(),
                    text: None,
                },
                attr_count,
            ));
            continue;
        }

        let value = read_value(data, &mut pos, kind, wide)?;
        let (open, remaining_attrs) = stack
            .last_mut()
            .ok_or(Error::UnexpectedTextOutsideElement)?;
        if *remaining_attrs > 0 {
            *remaining_attrs -= 1;
            open.attributes.push((name, value));
        } else {
            open.children.push(Element {
                name,
                attributes: Vec::new(),
                children: vec![],
                text: Some(match value {
                    Value::Int(v) => v.to_string(),
                    Value::Bool(v) => v.to_string(),
                    Value::Float(v) => v.to_string(),
                    Value::Vec3([x, y, z]) => format!("{x} {y} {z}"),
                    Value::String(s) => s,
                }),
            });
        }
    }
}

fn resolve_descriptor(
    data: &[u8],
    pos: &mut usize,
    descriptors: &mut Vec<String>,
    idx: u8,
) -> Result<String, Error> {
    let idx_usize = usize::from(idx);
    if idx_usize < descriptors.len() {
        return Ok(descriptors[idx_usize].clone());
    }
    if idx_usize == descriptors.len() {
        let len = usize::from(read_u16(data, pos)?);
        let raw = read_bytes(data, pos, len)?;
        let name = std::str::from_utf8(raw)
            .map_err(|_| Error::InvalidUtf8)?
            .to_owned();
        descriptors.push(name.clone());
        return Ok(name);
    }
    Err(Error::InvalidDescriptorIndex {
        idx,
        count: descriptors.len(),
    })
}

fn read_value(data: &[u8], pos: &mut usize, kind: u8, wide: bool) -> Result<Value, Error> {
    match kind {
        0x10 => {
            if wide {
                Ok(Value::Int(i64::from_le_bytes(
                    read_bytes(data, pos, 8)?
                        .try_into()
                        .map_err(|_| Error::UnexpectedEof)?,
                )))
            } else {
                Ok(Value::Int(i64::from(i32::from_le_bytes(
                    read_bytes(data, pos, 4)?
                        .try_into()
                        .map_err(|_| Error::UnexpectedEof)?,
                ))))
            }
        }
        0x20 => Ok(Value::Bool(true)),
        0x30 => Ok(Value::Bool(false)),
        0x40 => Ok(Value::Float(read_scalar_float(data, pos, wide)?)),
        0x50 => Ok(Value::Vec3([
            read_scalar_float(data, pos, wide)?,
            read_scalar_float(data, pos, wide)?,
            read_scalar_float(data, pos, wide)?,
        ])),
        0x60 => {
            let len = usize::from(read_u16(data, pos)?);
            let raw = read_bytes(data, pos, len)?;
            let s = std::str::from_utf8(raw)
                .map_err(|_| Error::InvalidUtf8)?
                .to_owned();
            Ok(Value::String(s))
        }
        other => Err(Error::UnknownRecordType(other)),
    }
}

fn read_scalar_float(data: &[u8], pos: &mut usize, wide: bool) -> Result<f64, Error> {
    if wide {
        Ok(f64::from_le_bytes(
            read_bytes(data, pos, 8)?
                .try_into()
                .map_err(|_| Error::UnexpectedEof)?,
        ))
    } else {
        Ok(f64::from(f32::from_le_bytes(
            read_bytes(data, pos, 4)?
                .try_into()
                .map_err(|_| Error::UnexpectedEof)?,
        )))
    }
}

fn read_u8(data: &[u8], pos: &mut usize) -> Result<u8, Error> {
    let b = *data.get(*pos).ok_or(Error::UnexpectedEof)?;
    *pos += 1;
    Ok(b)
}

fn read_u16(data: &[u8], pos: &mut usize) -> Result<u16, Error> {
    let bytes = read_bytes(data, pos, 2)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], pos: &mut usize) -> Result<u32, Error> {
    let bytes = read_bytes(data, pos, 4)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_bytes<'a>(data: &'a [u8], pos: &mut usize, len: usize) -> Result<&'a [u8], Error> {
    let end = pos.checked_add(len).ok_or(Error::UnexpectedEof)?;
    let slice = data.get(*pos..end).ok_or(Error::UnexpectedEof)?;
    *pos = end;
    Ok(slice)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_desc(out: &mut Vec<u8>, idx: u8, kind: u8, name: &str) {
        out.push(idx);
        out.push(kind);
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(name.as_bytes());
    }

    #[test]
    fn parses_wide_tree_with_reused_descriptor_types_and_text() {
        let mut buf = Vec::new();
        buf.extend_from_slice(MAGIC);
        buf.extend_from_slice(&WIDE_PROLOGUE);

        // Root element "Root" (idx 0) with 1 attribute ("val" = Int(42), idx 1).
        push_desc(&mut buf, 0, 0x00, "Root");
        buf.extend_from_slice(&0u16.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes());

        push_desc(&mut buf, 1, 0x10, "val");
        buf.extend_from_slice(&42i64.to_le_bytes());

        // Child "Item" (idx 2) reusing descriptor idx 1 ("val") as a String instead of Int.
        push_desc(&mut buf, 2, 0x00, "Item");
        buf.extend_from_slice(&0u16.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes());
        buf.push(1);
        buf.push(0x60);
        buf.extend_from_slice(&2u16.to_le_bytes());
        buf.extend_from_slice(b"ok");

        // Text on Item (NUL-terminated).
        buf.extend_from_slice(&[0xFD, 0xFF]);
        buf.extend_from_slice(&6u32.to_le_bytes());
        buf.extend_from_slice(b"hello\0");

        // Close Item, then close Root.
        buf.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);

        let (root, consumed) = parse(&buf).unwrap();
        assert_eq!(consumed, buf.len());
        assert_eq!(root.name, "Root");
        assert_eq!(root.attributes, vec![("val".into(), Value::Int(42))]);
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].name, "Item");
        assert_eq!(
            root.children[0].attributes,
            vec![("val".into(), Value::String("ok".into()))]
        );
        assert_eq!(root.children[0].text.as_deref(), Some("hello"));
    }

    #[test]
    fn parses_narrow_stream_without_prologue() {
        let mut buf = Vec::new();
        buf.extend_from_slice(MAGIC);
        push_desc(&mut buf, 0, 0x00, "CTuningFile");
        buf.extend_from_slice(&0u16.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes());
        buf.extend_from_slice(&2u16.to_le_bytes());

        push_desc(&mut buf, 1, 0x40, "scale");
        buf.extend_from_slice(&1.5f32.to_le_bytes());

        push_desc(&mut buf, 2, 0x20, "enabled");
        buf.extend_from_slice(&[0xFF, 0xFF]);

        let (root, consumed) = parse(&buf).unwrap();
        assert_eq!(consumed, buf.len());
        assert_eq!(root.name, "CTuningFile");
        assert_eq!(
            root.attributes,
            vec![
                ("scale".into(), Value::Float(1.5)),
                ("enabled".into(), Value::Bool(true)),
            ]
        );
    }

    #[test]
    fn rejects_bad_magic_and_out_of_range_descriptors() {
        assert_eq!(parse(b"NOPE"), Err(Error::BadMagic));
        let mut bad = MAGIC.to_vec();
        bad.extend_from_slice(&[3, 0x00]);
        assert_eq!(
            parse(&bad),
            Err(Error::InvalidDescriptorIndex { idx: 3, count: 0 })
        );
    }
}
