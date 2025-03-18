//! A listener that speaks the [PROXY][docs] protocol.
//!
//! Right now only version 1 is implemented.
//!
//! [docs]: https://www.haproxy.org/download/1.8/doc/proxy-protocol.txt
use axum::{extract, serve};
use std::marker::PhantomData;
use std::net::{IpAddr, SocketAddr};
use tokio::io::{self, AsyncReadExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Debug, PartialEq, Eq)]
enum Error {
    InsufficientData,
    InvalidHeader,
    PartialHeader,
    InvalidFormat,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Addr {
    source: SocketAddr,
    destination: Option<SocketAddr>,
}

unsafe impl Send for Addr {}

impl Addr {
    pub fn new(source: SocketAddr) -> Self {
        Self {
            source,
            destination: None,
        }
    }
}

impl From<SocketAddr> for Addr {
    fn from(value: SocketAddr) -> Self {
        Self {
            source: value,
            destination: None,
        }
    }
}

impl From<(SocketAddr, Option<SocketAddr>)> for Addr {
    fn from((source, destination): (SocketAddr, Option<SocketAddr>)) -> Self {
        Self {
            source,
            destination,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum AddressSource {
    FromHeader(Addr),
    FromStream,
    Error(Error),
}

trait Parser {
    const MIN_LENGTH: usize;
    fn parse(buf: &[u8]) -> (usize, AddressSource);
}

const PROXYV1_DELIMITER: &[u8; 2] = b"\r\n";
const PROXYV1_INET_UNKNOWN: &str = "UNKNOWN";

/// Version 1 of the PROXY protocol.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct V1;

/// SAFETY: it's empty?
unsafe impl Send for V1 {}

impl Parser for V1 {
    const MIN_LENGTH: usize = 107;

    fn parse(buf: &[u8]) -> (usize, AddressSource) {
        if buf.len() < PROXYV1_DELIMITER.len() {
            return (0, AddressSource::Error(Error::InsufficientData));
        }
        let Some(contents_end) = buf
            .windows(PROXYV1_DELIMITER.len())
            .position(|b| b == PROXYV1_DELIMITER)
        else {
            return (0, AddressSource::Error(Error::InvalidHeader));
        };
        let header_end = contents_end + PROXYV1_DELIMITER.len();
        let mut header_raw = buf[..contents_end]
            .split(|x| *x == b' ')
            .map(std::str::from_utf8);
        let (Some(Ok(_)), Some(Ok(inet))) = (header_raw.next(), header_raw.next()) else {
            return (header_end, AddressSource::Error(Error::PartialHeader));
        };
        if inet == PROXYV1_INET_UNKNOWN {
            return (header_end, AddressSource::FromStream);
        }
        let (
            Some(Ok(addr_source_raw)),
            Some(Ok(addr_destination_raw)),
            Some(Ok(port_source_raw)),
            Some(Ok(port_destination_raw)),
        ) = (
            header_raw.next(),
            header_raw.next(),
            header_raw.next(),
            header_raw.next(),
        )
        else {
            return (header_end, AddressSource::FromStream);
        };
        let Ok(addr_source) = addr_source_raw.parse::<IpAddr>() else {
            return (header_end, AddressSource::Error(Error::InvalidFormat));
        };
        let Ok(addr_destination) = addr_destination_raw.parse::<IpAddr>() else {
            return (header_end, AddressSource::Error(Error::InvalidFormat));
        };
        let Ok(port_source) = port_source_raw.parse::<u16>() else {
            return (header_end, AddressSource::Error(Error::InvalidFormat));
        };
        let Ok(port_destination) = port_destination_raw.parse::<u16>() else {
            return (header_end, AddressSource::Error(Error::InvalidFormat));
        };
        (
            header_end,
            AddressSource::FromHeader(
                (
                    SocketAddr::from((addr_source, port_source)),
                    Some(SocketAddr::from((addr_destination, port_destination))),
                )
                    .into(),
            ),
        )
    }
}

/// Probably only use this behind a _trusted_ load balancer. It's susceptible to attackers which
/// sends data slowly.
///
/// There's some mention from the docs (HAProxy) that load balancers will be able to fit the header
/// and some of the data into a frame. So, in practise, this may not be a huge deal.
pub struct Listener<P> {
    listener: TcpListener,
    parser: PhantomData<P>,
}

impl<P> Listener<P> {
    pub async fn new(listener: TcpListener) -> Self {
        Self {
            listener,
            parser: PhantomData,
        }
    }
}

impl From<TcpListener> for Listener<V1> {
    fn from(value: TcpListener) -> Self {
        Self {
            listener: value,
            parser: PhantomData,
        }
    }
}

impl<P> serve::Listener for Listener<P>
where
    P: Parser + Send + 'static,
{
    type Io = TcpStream;
    type Addr = Addr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        let mut header_buf: Vec<u8> = vec![0; P::MIN_LENGTH];
        loop {
            match self.listener.accept().await {
                Ok((mut stream, _)) => {
                    let Ok(_) = stream.peek(&mut header_buf[..]).await else {
                        if cfg!(feature = "tracing") {
                            tracing::warn!("could not read header");
                        }
                        continue;
                    };
                    let (advance_by, source) = P::parse(&header_buf[..]);
                    // First, try advancing the stream.
                    let Ok(read) = stream.read(&mut header_buf[..advance_by]).await else {
                        if cfg!(feature = "tracing") {
                            tracing::warn!("could not read from stream");
                        }
                        continue;
                    };
                    assert_eq!(read, advance_by);
                    match source {
                        AddressSource::FromHeader(s) => {
                            return (stream, s);
                        }
                        AddressSource::FromStream => {
                            let Ok(peer_addr) = stream.peer_addr() else {
                                if cfg!(feature = "tracing") {
                                    tracing::warn!("could not read peer address from stream");
                                }
                                continue;
                            };
                            return (stream, peer_addr.into());
                        }
                        AddressSource::Error(e) => {
                            if cfg!(feature = "tracing") {
                                tracing::warn!("parsing PROXY protocol: {e:?}");
                            }
                            continue;
                        }
                    }
                }
                Err(_) => {
                    if cfg!(feature = "tracing") {
                        tracing::warn!("error accepting socket");
                    }
                }
            }
        }
    }

    #[inline]
    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.listener.local_addr().map(|a| a.into())
    }
}

impl<P> extract::connect_info::Connected<serve::IncomingStream<'_, Listener<P>>> for Addr
where
    P: Parser + Send + 'static,
{
    fn connect_info(stream: serve::IncomingStream<'_, Listener<P>>) -> Self {
        stream.remote_addr().clone()
    }
}
