//! # axum-proxied
//!
//! Helpers for axum when running behind a proxy of some sort.
//!
//! See
//! * [`crate::extract`] for HTTP Header extractors for  `Forwarded` and `X-Forwarded-For`, and
//! * [`crate::proxy`] for a [`tokio::net::TcpListener`]-based listener which supports the [HAProxy
//!   PROXY protocol][proxy].
//!
//! [proxy]: https://www.haproxy.org/download/1.8/doc/proxy-protocol.txt
pub mod extract;
pub mod proxy;

#[doc(hidden)]
pub(crate) mod parser;
