//! A listener that speaks the [PROXY][docs] protocol.
//!
//! [docs]: https://www.haproxy.org/download/1.8/doc/proxy-protocol.txt
use axum::{extract, serve};
use std::net::{Ipv4Addr, SocketAddr};
use tokio::io::{self, AsyncReadExt};
use tokio::net::{TcpListener, TcpStream};

use crate::parser::{parse, ParseResult, Where};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Addr {
    source: SocketAddr,
    destination: SocketAddr,
}

unsafe impl Send for Addr {}

impl Addr {
    pub fn new(source: SocketAddr, destination: SocketAddr) -> Self {
        Self {
            source,
            destination,
        }
    }
}

impl From<SocketAddr> for Addr {
    fn from(value: SocketAddr) -> Self {
        Self {
            source: value,
            destination: SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)),
        }
    }
}

impl From<(SocketAddr, SocketAddr)> for Addr {
    fn from((source, destination): (SocketAddr, SocketAddr)) -> Self {
        Self {
            source,
            destination,
        }
    }
}

/// Probably only use this behind a _trusted_ load balancer. It's susceptible to attackers which
/// sends data slowly.
///
/// There's some mention from the docs (HAProxy) that load balancers will be able to fit the header
/// and some of the data into a frame. So, in practise, this may not be a huge deal.
pub struct Listener {
    listener: TcpListener,
}

impl Listener {
    pub async fn new(listener: TcpListener) -> Self {
        Self {
            listener,
        }
    }
}

impl From<TcpListener> for Listener {
    fn from(value: TcpListener) -> Self {
        Self {
            listener: value,
        }
    }
}

impl serve::Listener for Listener {
    type Io = TcpStream;
    type Addr = Addr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        let mut header_buf: Vec<u8> = vec![0; 512];
        loop {
            let mut stream = match self.listener.accept().await {
                Ok((stream, _)) => stream,
                Err(e) => {
                    if cfg!(feature = "tracing") {
                        tracing::warn!("error accepting socket {e:?}");
                    }
                    continue;
                }
            };
            let Ok(read) = stream.peek(&mut header_buf[..]).await else {
                if cfg!(feature = "tracing") {
                    tracing::warn!("could not read header");
                }
                continue;
            };
            let (advance_by, source, destination) = match parse(&header_buf[..read]) {
                Ok(ParseResult(advance_by, Where::Underlying)) => {
                    let Ok(source) = stream.peer_addr() else {
                        if cfg!(feature = "tracing") {
                            tracing::warn!("could not read source from peer addr");
                        }
                        continue;
                    };
                    (advance_by, source, SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))
                },
                Ok(ParseResult(advance_by, Where::Header { source, destination })) => {
                    (advance_by, source, destination)
                },
                Err(e) => {
                    if cfg!(feature = "tracing") {
                        tracing::warn!("could not parse PROXY information {e:?}");
                    }
                    continue;
                },
            };
            // First, try advancing the stream.
            let Ok(read) = stream.read(&mut header_buf[..advance_by]).await else {
                if cfg!(feature = "tracing") {
                    tracing::warn!("could not read from stream");
                }
                continue;
            };
            assert_eq!(read, advance_by);
            return (stream, Addr::new(source, destination));
        }
    }

    #[inline]
    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.listener.local_addr().map(|a| a.into())
    }
}

impl extract::connect_info::Connected<serve::IncomingStream<'_, Listener>> for Addr {
    fn connect_info(stream: serve::IncomingStream<'_, Listener>) -> Self {
        stream.remote_addr().clone()
    }
}
