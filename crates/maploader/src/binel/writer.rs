use byteorder::{LittleEndian, WriteBytesExt};
use itertools::Itertools;
use std::{
    collections::HashMap,
    io::{prelude::*, Error, ErrorKind},
};

use crate::binel::{BinEl, BinElAttr, BinFile};

/// Write a string using a varint for the length.
///
/// # Examples:
/// ```
/// use std::io::Cursor;
/// use celeste::binel::writer::*;
///
/// let mut buf = Cursor::new(vec![0; 12]);
///
/// put_string(&mut buf, "CELESTE MAP").unwrap();
///
/// assert_eq!(&buf.get_ref()[..], b"\x0bCELESTE MAP");
/// ```
pub fn put_string(writer: &mut dyn Write, string: &str) -> std::io::Result<()> {
    let mut length_buf = unsigned_varint::encode::usize_buffer();
    let length = unsigned_varint::encode::usize(string.len(), &mut length_buf);
    writer.write_all(length)?;

    writer.write_all(string.as_bytes())?;

    Ok(())
}

/// Write a bool, tagged with 0x00.
pub fn put_tagged_bool(writer: &mut dyn Write, val: bool) -> std::io::Result<()> {
    writer.write_u8(0x00)?;
    writer.write_u8(val as u8)?;

    Ok(())
}

/// Write an i32 as either a u8 (tagged with 0x01), i16 (tagged with 0x02), or i32 (tagged with 0x03).
pub fn put_tagged_int(writer: &mut dyn Write, val: i32) -> std::io::Result<()> {
    if let Ok(val) = u8::try_from(val) {
        writer.write_u8(0x01)?;
        writer.write_u8(val)?;
    } else if let Ok(val) = i16::try_from(val) {
        writer.write_u8(0x02)?;
        writer.write_i16::<LittleEndian>(val)?;
    } else {
        writer.write_u8(0x03)?;
        writer.write_i32::<LittleEndian>(val)?;
    }

    Ok(())
}

/// Write an f32, tagged with 0x04.
pub fn put_tagged_f32(writer: &mut dyn Write, val: f32) -> std::io::Result<()> {
    writer.write_u8(0x04)?;
    writer.write_f32::<LittleEndian>(val)?;

    Ok(())
}

/// Encode a string in Celeste's RLE format. Allocates two bytes on the heap due to a current limitation of iterators.
pub fn encode_rle_string(string: &str) -> Vec<u8> {
    string
        .bytes()
        .group_by(|e| *e)
        .into_iter()
        .flat_map(|(ch, run)| vec![run.count() as u8, ch])
        .collect() // rust#25725
}

/// Write a string either using a lookup (stored as u16, tagged with 0x05), Celeste's RLE format (tagged with 0x07), or using a varint (tagged with 0x06).
pub fn put_tagged_str(
    mut writer: &mut dyn Write,
    lookup: &[&str],
    val: &str,
) -> std::io::Result<()> {
    if let Some(index) = lookup.iter().position(|e| *e == val) {
        writer.write_u8(0x05)?;
        writer.write_u16::<LittleEndian>(index as u16)?;
    } else {
        let rle = encode_rle_string(val);

        if rle.len() < val.len() && rle.len() <= i16::max_value() as usize {
            writer.write_u8(0x07)?;
            writer.write_i16::<LittleEndian>(rle.len() as i16)?;
            writer.write_all(&rle)?;
        } else {
            writer.write_u8(0x06)?;
            put_string(&mut writer, val)?;
        }
    }

    Ok(())
}

/// Write a `BinEl` using an existing lookup table for element and attribute named.
pub fn put_element(
    mut writer: &mut dyn Write,
    lookup: &[&str],
    elem: &BinEl,
) -> std::io::Result<()> {
    let Some(name_index) = lookup.iter().position(|&e| e == elem.name) else {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Element name {} is missing in lookup", elem.name),
            ))
        };

    writer.write_u16::<LittleEndian>(name_index as u16)?;
    writer.write_u8(elem.attributes.keys().len() as u8)?;

    for (attr, value) in &elem.attributes {
        let Some(attr_index) = lookup.iter().position(|e| e == attr) else {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!("Attribute name {} is missing in lookup", attr),
                ))
            };
        writer.write_u16::<LittleEndian>(attr_index as u16)?;
        match value {
            BinElAttr::Bool(val) => put_tagged_bool(&mut writer, *val)?,
            BinElAttr::Int(val) => put_tagged_int(&mut writer, *val)?,
            BinElAttr::Float(val) => put_tagged_f32(&mut writer, *val)?,
            BinElAttr::Text(val) => put_tagged_str(&mut writer, lookup, val)?,
        }
    }

    writer.write_u16::<LittleEndian>(elem.children().count() as u16)?;
    for child in elem.children() {
        put_element(&mut writer, lookup, child)?;
    }

    Ok(())
}

fn gen_lookup_keys<'a>(binel: &'a BinEl, seen: &mut HashMap<&'a str, usize>) {
    seen.insert(&binel.name, seen.get(binel.name.as_str()).unwrap_or(&0) + 1);

    for (k, v) in &binel.attributes {
        *seen.entry(k).or_default() += 1;
        if k != "innerText" {
            if let BinElAttr::Text(text) = v {
                *seen.entry(text).or_default() += 1;
            }
        }
    }

    for child in binel.children() {
        gen_lookup_keys(child, seen);
    }
}

/// Generate a string lookup using the attributes and element names in a `BinEl`.
pub fn gen_lookup(binel: &BinEl) -> Vec<&str> {
    let mut seen = HashMap::new();
    gen_lookup_keys(binel, &mut seen);
    let mut vec = seen.into_iter().collect::<Vec<(&str, usize)>>();
    vec.sort_unstable_by_key(|e| e.1);
    vec.into_iter().rev().map(|e| e.0).collect()
}

/// Write a `BinFile`. Tested solely in integration tests due to complexity.
pub fn put_file(mut writer: &mut dyn Write, bin: &BinFile) -> std::io::Result<()> {
    put_string(&mut writer, "CELESTE MAP")?;

    put_string(&mut writer, &bin.package)?;

    let lookup = gen_lookup(&bin.root);

    writer.write_i16::<LittleEndian>(lookup.len() as i16)?;

    for s in &lookup {
        put_string(&mut writer, s)?;
    }

    put_element(&mut writer, &lookup, &bin.root)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    #[test]
    fn put_header() {
        let mut buf = Cursor::new(vec![0; 12]);

        super::put_string(&mut buf, "CELESTE MAP").unwrap();

        assert_eq!(&buf.get_ref()[..], b"\x0bCELESTE MAP");
    }
}
