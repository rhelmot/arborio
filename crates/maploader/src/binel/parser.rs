use nom::{
    bytes::complete::{tag, take},
    error::{Error, ErrorKind, ParseError},
    multi::{count, length_count},
    number::complete::{le_f32, le_i16, le_i32, le_u16, le_u8},
    sequence::pair,
    IResult, Parser,
};
use nom_varint::take_varint;

use crate::binel::{BinEl, BinElAttr, BinFile};

/// Take a string with the length being a varint.
///
/// # Examples:
/// ```
/// use celeste::binel::parser::take_string;
///
/// let header = b"\x0bCELESTE MAP";
///
/// assert_eq!(take_string(&header[..]), Ok((&b""[..], "CELESTE MAP".to_string())));
/// ```
pub fn take_str(input: &[u8]) -> IResult<&[u8], &str> {
    let (input, length) = take_varint(input)?;
    take(length)
        .and_then(|s| match std::str::from_utf8(s) {
            Ok(s) => Ok((&[] as &[u8], s)),
            Err(_) => Err(nom::Err::Error(Error::from_error_kind(s, ErrorKind::Fail))),
        })
        .parse(input)
}

pub fn take_file(input: &[u8]) -> IResult<&[u8], BinFile> {
    let (input, _) = tag(b"\x0bCELESTE MAP")(input)?;
    let (input, package) = take_str(input)?;
    let (input, lookup) = length_count(le_u16, take_str)(input)?;
    let (input, root) = take_element(input, &lookup)?;
    Ok((
        input,
        BinFile {
            package: package.to_owned(),
            root,
        },
    ))
}

/// Parse a `BinEl` from a `&[u8]`. Tested solely in integration tests due to complexity.
pub fn take_element<'a>(buf: &'a [u8], lookup: &[&'a str]) -> IResult<&'a [u8], BinEl> {
    let (buf, name) = take_lookup(buf, lookup)?;

    let mut binel = BinEl::new(name);

    // let (mut buf, attr_count) = le_u8(buf)?;

    let (buf, attrs) = length_count(
        le_u8,
        pair((|buf| take_lookup(buf, lookup)).map(str::to_owned), |buf| {
            take_elemattr(buf, lookup)
        }),
    )(buf)?;

    binel.attributes.extend(attrs);

    let (buf, children) = length_count(le_u16, |buf| take_element(buf, lookup))(buf)?;

    children.into_iter().for_each(|child| binel.insert(child));

    Ok((buf, binel))
}

/// Lookup a u16 from a `&[u8]` in a string lookup table.
pub fn take_lookup<'a, 'b>(buf: &'a [u8], lookup: &[&'b str]) -> IResult<&'a [u8], &'b str> {
    le_u16.map(|index| lookup[index as usize]).parse(buf)
}

/// Take a Celeste RLE-encoded string from a `&[u8]`
pub fn take_rle_string(buf: &[u8]) -> IResult<&[u8], String> {
    let (buf, len) = le_i16(buf)?;
    let (buf, chars) = count(take_rle_char, len as usize / 2)(buf)?;
    Ok((buf, chars.concat()))
}

/// Take a single character from a Celeste RLE-encoded string in a `&[u8]`.
pub fn take_rle_char(buf: &[u8]) -> IResult<&[u8], String> {
    pair(le_u8, le_u8)
        .map(|(times, byte)| char::from(byte).to_string().repeat(times as usize))
        .parse(buf)
}

/// Parse a `BinElAttr` from a `&[u8]`.
///
/// # Examples:
/// ```
/// use celeste::binel::*;
///
/// assert_eq!(parser::take_elemattr(b"\x01\x05", &[]), Ok((&b""[..], BinElAttr::Int(5))));
/// ```
#[allow(clippy::cognitive_complexity)]
pub fn take_elemattr<'a>(buf: &'a [u8], lookup: &[&'a str]) -> IResult<&'a [u8], BinElAttr> {
    let (buf, elem_tag) = le_u8(buf)?;
    match elem_tag {
        0 => le_u8.map(|byte| byte != 0).map(BinElAttr::Bool).parse(buf),
        1 => le_u8.map(i32::from).map(BinElAttr::Int).parse(buf),
        2 => le_i16.map(i32::from).map(BinElAttr::Int).parse(buf),
        3 => le_i32.map(BinElAttr::Int).parse(buf),
        4 => le_f32.map(BinElAttr::Float).parse(buf),
        5 => (|input| take_lookup(input, lookup))
            .map(str::to_owned)
            .map(BinElAttr::Text)
            .parse(buf),
        6 => take_str.map(str::to_owned).map(BinElAttr::Text).parse(buf),
        7 => take_rle_string.map(BinElAttr::Text).parse(buf),
        _ => todo!(),
    }
}
