//! A parser for the [PROXY][proxy] protocol.
//!
//! The spec gives an example of how to do this with casting, but it's more fun to write manually.
//!
//! [proxy]: https://www.haproxy.org/download/1.8/doc/proxy-protocol.txt

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::ops::Range;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    ShortHeader,
    InvalidFormat,
    UnsupportedVersion(u8),
    UnsupportedCommand(u8),
    UnsupportedFamily(u8),
    UnsupportedProtocol(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Where {
    Header {
        source: SocketAddr,
        destination: SocketAddr,
    },
    Underlying,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseResult(pub(crate) usize, pub(crate) Where);

#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
enum Version {
    V1 = 1,
    V2 = 2,
    Other(u8),
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
enum Command {
    Local = 0,
    Proxy = 1,
    Other(u8),
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
enum Family {
    Unspecified = 0,
    Inet = 1,
    Inet6 = 2,
    Unix = 3,
    Other(u8),
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
enum Protocol {
    Unspecified = 0,
    Stream = 1,
    Datagram = 2,
    Other(u8),
}

const PROXY_V1_MAGIC: &[u8] = b"PROXY ";
const PROXY_V1_DELIMITER: &[u8] = b"\r\n";
const PROXY_V1_UNKNOWN_PROTO: &[u8] = b"UNKNOWN";
const PROXY_V2_MAGIC: &[u8] = &[
    0x0D, 0x0A, 0x0D, 0x0A, 0x00, 0x0D, 0x0A, 0x51, 0x55, 0x49, 0x54, 0x0A,
];

fn is_proxy_protocol(buf: &[u8]) -> Option<Version> {
    if buf.starts_with(PROXY_V1_MAGIC) {
        return Some(Version::V1);
    }
    if buf.starts_with(PROXY_V2_MAGIC) {
        return Some(Version::V2);
    }
    None
}

impl From<std::str::Utf8Error> for Error {
    fn from(_: std::str::Utf8Error) -> Self {
        Self::InvalidFormat
    }
}

impl From<std::net::AddrParseError> for Error {
    fn from(_: std::net::AddrParseError) -> Self {
        Self::InvalidFormat
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(_: std::num::ParseIntError) -> Self {
        Self::InvalidFormat
    }
}

impl From<std::array::TryFromSliceError> for Error {
    fn from(_: std::array::TryFromSliceError) -> Self {
        Self::InvalidFormat
    }
}

macro_rules! try_parse_v1_addr {
    ($var:ident, $kind:ty) => {
        $var.next()
            .ok_or(Error::ShortHeader)
            .and_then(|s| Ok(std::str::from_utf8(s)?))
            .and_then(|s| Ok(s.parse::<$kind>()?))
    };
}

fn parse_v1(buf: &[u8]) -> Result<ParseResult, Error> {
    let Some(contents_end) = buf[..]
        .windows(PROXY_V1_DELIMITER.len())
        .position(|w| w == PROXY_V1_DELIMITER)
    else {
        return Err(Error::ShortHeader);
    };
    let header_end = contents_end + PROXY_V1_DELIMITER.len();
    let mut fields = buf[PROXY_V1_MAGIC.len()..contents_end].split(|x| *x == b' ');
    let inet_proto = fields.next().ok_or(Error::ShortHeader)?;
    if inet_proto == PROXY_V1_UNKNOWN_PROTO {
        return Ok(ParseResult(header_end, Where::Underlying));
    }
    let ip_source = try_parse_v1_addr!(fields, IpAddr)?;
    let ip_destination = try_parse_v1_addr!(fields, IpAddr)?;
    let port_source = try_parse_v1_addr!(fields, u16)?;
    let port_destination = try_parse_v1_addr!(fields, u16)?;
    let source = SocketAddr::from((ip_source, port_source));
    let destination = SocketAddr::from((ip_destination, port_destination));
    Ok(ParseResult(
        header_end,
        Where::Header {
            source,
            destination,
        },
    ))
}

// From the spec.
const PROXY_V2_VERSION_COMMAND_INDEX: usize = 12;
const PROXY_V2_FAMILY_PROTO_INDEX: usize = 13;
const PROXY_V2_LENGTH_RANGE: Range<usize> = 14..16;

impl From<u8> for Version {
    fn from(value: u8) -> Self {
        match (value & 0b11110000) >> 4 {
            1 => Self::V1,
            2 => Self::V2,
            v => Self::Other(v),
        }
    }
}

impl From<u8> for Command {
    fn from(value: u8) -> Self {
        match value & 0b00001111 {
            0 => Self::Local,
            1 => Self::Proxy,
            o => Self::Other(o),
        }
    }
}

fn version_command_from_u8(value: u8) -> (Version, Command) {
    (Version::from(value), Command::from(value))
}

impl From<u8> for Family {
    fn from(value: u8) -> Self {
        match (value & 0b11110000) >> 4 {
            0 => Self::Unspecified,
            1 => Self::Inet,
            2 => Self::Inet6,
            3 => Self::Unix,
            o => Self::Other(o),
        }
    }
}

impl From<u8> for Protocol {
    fn from(value: u8) -> Self {
        match value & 0b00001111 {
            0 => Self::Unspecified,
            1 => Self::Stream,
            2 => Self::Datagram,
            o => Self::Other(o),
        }
    }
}

fn family_protocol_from_u8(value: u8) -> (Family, Protocol) {
    (Family::from(value), Protocol::from(value))
}

macro_rules! try_parse_v2_inet_addr {
    ($buf:expr, $buf_repr:ty, $int_repr:ty, $socket_ty:ty) => {
        $buf.ok_or(Error::ShortHeader)
            .and_then(|s| Ok(TryInto::<$buf_repr>::try_into(s)?))
            .and_then(|s| Ok(<$int_repr>::from_be_bytes(s)))
            .and_then(|s| Ok(<$socket_ty>::from(s)))
    };
    ($buf:expr, $buf_repr:ty, $socket_ty:ty) => {
        $buf.ok_or(Error::ShortHeader)
            .and_then(|s| Ok(TryInto::<$buf_repr>::try_into(s)?))
            .and_then(|s| Ok(<$socket_ty>::from(s)))
    };
}

macro_rules! try_parse_u16 {
    ($buf:expr) => {
        $buf.ok_or(Error::InvalidFormat)?
            .try_into()
            .map(u16::from_be_bytes)
    };
}

fn parse_v2_inet(buf: &[u8]) -> Result<(usize, SocketAddr, SocketAddr), Error> {
    let source_addr = try_parse_v2_inet_addr!(buf.get(0..4), [u8; 4], u32, Ipv4Addr)?;
    let destination_addr = try_parse_v2_inet_addr!(buf.get(4..8), [u8; 4], u32, Ipv4Addr)?;
    let source_port: u16 = try_parse_u16!(buf.get(8..10))?;
    let destination_port: u16 = try_parse_u16!(buf.get(10..12))?;
    Ok((
        12,
        SocketAddr::from((source_addr, source_port)),
        SocketAddr::from((destination_addr, destination_port)),
    ))
}

fn parse_v2_inet6(buf: &[u8]) -> Result<(usize, SocketAddr, SocketAddr), Error> {
    let source_addr = try_parse_v2_inet_addr!(buf.get(0..16), [u8; 16], IpAddr)?;
    let destination_addr = try_parse_v2_inet_addr!(buf.get(16..32), [u8; 16], IpAddr)?;
    let source_port: u16 = try_parse_u16!(buf.get(32..34))?;
    let destination_port: u16 = try_parse_u16!(buf.get(34..36))?;
    Ok((
        36,
        SocketAddr::from((source_addr, source_port)),
        SocketAddr::from((destination_addr, destination_port)),
    ))
}

fn parse_v2(buf: &[u8]) -> Result<ParseResult, Error> {
    let (version, command) = buf
        .get(PROXY_V2_VERSION_COMMAND_INDEX)
        .ok_or(Error::ShortHeader)
        .map(|vc| version_command_from_u8(*vc))?;
    let (family, protocol) = buf
        .get(PROXY_V2_FAMILY_PROTO_INDEX)
        .ok_or(Error::ShortHeader)
        .map(|fp| family_protocol_from_u8(*fp))?;
    let length = try_parse_u16!(buf.get(PROXY_V2_LENGTH_RANGE)).map(usize::from)?;
    let contents_start = PROXY_V2_LENGTH_RANGE.end;
    let header_end = PROXY_V2_LENGTH_RANGE.end + length;
    if buf.len() < header_end {
        return Err(Error::ShortHeader);
    }
    match version {
        Version::V2 => {}
        Version::V1 => {
            return Err(Error::UnsupportedVersion(1));
        }
        Version::Other(v) => {
            return Err(Error::UnsupportedVersion(v));
        }
    }
    let (_addresses_length, source, destination) = match (command, family, protocol) {
        (Command::Other(c), _, _) => {
            return Err(Error::UnsupportedCommand(c));
        }
        (_, Family::Other(f), _) => {
            return Err(Error::UnsupportedFamily(f));
        }
        // Maybe one day.
        (_, Family::Unix, _) => {
            return Err(Error::UnsupportedFamily(3));
        }
        (_, _, Protocol::Other(p)) => {
            return Err(Error::UnsupportedProtocol(p));
        }
        (_, Family::Unspecified, _) => {
            return Ok(ParseResult(header_end, Where::Underlying));
        }
        (_, _, Protocol::Unspecified) => {
            return Ok(ParseResult(header_end, Where::Underlying));
        }
        (_, Family::Inet, _) => parse_v2_inet(&buf[contents_start..])?,
        (_, Family::Inet6, _) => parse_v2_inet6(&buf[contents_start..])?,
    };
    // In the future, consider using _addresses_length to parse the extended headers.
    
    Ok(ParseResult(
        header_end,
        Where::Header {
            source,
            destination,
        },
    ))
}

pub fn parse(buf: &[u8]) -> Result<ParseResult, Error> {
    match is_proxy_protocol(buf) {
        Some(Version::V1) => parse_v1(buf),
        Some(Version::V2) => parse_v2(buf),
        Some(Version::Other(v)) => Err(Error::UnsupportedVersion(v)),
        None => Ok(ParseResult(0, Where::Underlying)),
    }
}
