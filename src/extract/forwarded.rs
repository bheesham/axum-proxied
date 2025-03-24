//! Support for HTTP Header [`Forwarded`][mdn].
//!
//! [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Forwarded
use axum::body::Body;
use axum::extract::OptionalFromRequestParts;
use axum::http::request::Parts;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

/// The protocol which initiated the request.
#[derive(Debug, PartialEq, Eq)]
pub enum Protocol {
    Http,
    Https,
    Other(String),
}

impl FromStr for Protocol {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("https") {
            return Ok(Self::Https);
        }
        if s.eq_ignore_ascii_case("http") {
            return Ok(Self::Http);
        }
        Ok(Self::Other(String::from(s)))
    }
}

/// The interface we're forwarding from/to.
///
/// Once
/// [`std::net::SocketAddr::parse_ascii`](https://doc.rust-lang.org/std/net/enum.SocketAddr.html#method.parse_ascii)
/// is out of nightly this should become _much_ simpler.
///
/// At that point
///
/// 1. Try parsing for IPv4
/// 1. Trim quotes,
/// 1. Try parsing for IPv6
/// 1. Else, it's probably an identifier.
#[derive(Debug, PartialEq, Eq)]
pub enum Interface {
    /// Probably a hostname or an internal name of some sort.
    Identifier(String),
    /// The parsed IP.
    Socket(SocketAddr),
    /// A literal `unknown`.
    Unknown,
}

impl From<SocketAddr> for Interface {
    fn from(value: SocketAddr) -> Self {
        Self::Socket(value)
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Parser {
    Begin,
    OpenBracket,
    CloseBracket,
    Colon,
}

impl FromStr for Interface {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("unknown") || s.eq_ignore_ascii_case(r#""unknown""#) {
            return Ok(Self::Unknown);
        }
        let s_trimmed = s.trim().trim_matches('"');
        if let Ok(simple) = s_trimmed.parse::<SocketAddr>() {
            return Ok(Self::Socket(simple));
        };
        if let Ok(simple) = s_trimmed.parse::<IpAddr>() {
            return Ok(Self::Socket(SocketAddr::from((simple, 0))));
        };
        let mut ip_start = 0;
        let mut ip_end = 0;
        let mut port_start = 0;
        let mut state = Parser::Begin;
        let chars = s_trimmed.chars().enumerate();
        for (index, c) in chars {
            match (c, &state) {
                ('[', Parser::Begin) => {
                    ip_start = index + 1;
                    state = Parser::OpenBracket;
                }
                (']', Parser::OpenBracket) => {
                    ip_end = index;
                    state = Parser::CloseBracket;
                }
                (':', Parser::CloseBracket) => {
                    port_start = index + 1;
                    state = Parser::Colon;
                }
                _ => {}
            }
        }
        let ip: Option<IpAddr> = s_trimmed
            .get(ip_start..ip_end)
            .and_then(|ip| ip.parse::<IpAddr>().ok());
        let port: Option<u16> = s_trimmed
            .get(port_start..)
            .and_then(|p| p.parse::<u16>().ok());
        match (ip, port) {
            (Some(ip), Some(port)) => Ok(Self::Socket(SocketAddr::from((ip, port)))),
            (Some(ip), None) => Ok(Self::Socket(SocketAddr::from((ip, 0)))),
            _ => Ok(Self::Identifier(String::from(s_trimmed.trim_matches('"')))),
        }
    }
}

/// A single "forwarded" entry. All fields are optional, as per the spec.
#[derive(Debug, PartialEq, Eq)]
pub struct Forward {
    /// The forwarder (proxy server).
    by: Option<Interface>,
    /// The request initiator.
    r#for: Option<Interface>,
    /// `Host` header, as seen by the proxy.
    host: Option<String>,
    /// The protocol used during this forward. 
    proto: Option<Protocol>,
}

impl Forward {
    pub fn new(
        by: Option<Interface>,
        r#for: Option<Interface>,
        host: Option<String>,
        proto: Option<Protocol>,
    ) -> Self {
        Self {
            by,
            r#for,
            host,
            proto,
        }
    }

    pub fn by(&self) -> &Option<Interface> {
        &self.by
    }

    pub fn r#for(&self) -> &Option<Interface> {
        &self.r#for
    }

    pub fn host(&self) -> &Option<String> {
        &self.host
    }

    pub fn proto(&self) -> &Option<Protocol> {
        &self.proto
    }
}

/// Get the contents of the `Forwarded` header.
///
/// Example:
///
/// ```rust
/// use axum_proxied::extract::Forwarded;
///
/// async fn handler(forwarded: Option<Forwarded>) {
///     todo!()
/// }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Forwarded {
    forwards: Vec<Forward>,
}

impl Forwarded {
    pub fn new(forwards: Vec<Forward>) -> Self {
        Self { forwards }
    }

    pub fn forwards(&self) -> &Vec<Forward> {
        &self.forwards
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ForwardedRejection(&'static str);

impl IntoResponse for ForwardedRejection {
    fn into_response(self) -> Response<Body> {
        (StatusCode::BAD_REQUEST, self.0).into_response()
    }
}

impl<S> OptionalFromRequestParts<S> for Forwarded
where
    S: Send + Sync,
{
    type Rejection = ForwardedRejection;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        let Some(header_raw) = parts.headers.get(header::FORWARDED) else {
            return Ok(None);
        };
        let Ok(header_str) = header_raw.to_str() else {
            return Err(ForwardedRejection("could not parse header into string"));
        };
        let mut forwards = vec![];
        let forwards_raw = header_str
            .split(",")
            .map(|f| f.split(";").map(|p| p.split("=")));
        for mut forward_raw in forwards_raw {
            let mut by: Option<Interface> = None;
            let mut r#for: Option<Interface> = None;
            let mut host: Option<String> = None;
            let mut proto: Option<Protocol> = None;
            for mut params in forward_raw.by_ref() {
                while let (Some(keyword), Some(value)) = (
                    params.next().map(|s| s.trim()),
                    params.next().map(|s| s.trim()),
                ) {
                    if keyword.eq_ignore_ascii_case("by") {
                        by = value.trim().parse::<Interface>().ok();
                    } else if keyword.eq_ignore_ascii_case("for") {
                        r#for = value.trim().parse::<Interface>().ok();
                    } else if keyword.eq_ignore_ascii_case("host") {
                        host = Some(String::from(value.trim()));
                    } else if keyword.eq_ignore_ascii_case("proto") {
                        proto = value.trim().parse::<Protocol>().ok();
                    }
                }
            }
            forwards.push(Forward::new(by, r#for, host, proto));
        }
        Ok(Some(Forwarded::new(forwards)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn simple() {
        let (mut parts, _) = axum::http::request::Builder::new()
            .method("GET")
            .header(
                "Forwarded",
                r#"by="[::0]:443";for=127.0.0.1;proto=https,for=5.5.5.5:444"#,
            )
            .body(())
            .expect("could not build request")
            .into_parts();
        let forwarded = Forwarded::from_request_parts(&mut parts, &())
            .await
            .expect("could not parse HTTP headers")
            .expect("could not parse Forwarded header");
        assert_eq!(
            forwarded,
            Forwarded {
                forwards: vec![
                    Forward::new(
                        r#""[::0]:443""#.parse::<Interface>().ok(),
                        "127.0.0.1:0".parse::<Interface>().ok(),
                        None,
                        Some(Protocol::Https),
                    ),
                    Forward::new(None, "5.5.5.5:444".parse::<Interface>().ok(), None, None,),
                ]
            }
        );
    }

    #[test]
    fn parse_simple_ipv6() {
        let simple = r#""[::0]:338""#;
        let parsed: Interface = simple.parse::<Interface>().expect("could not parse");
        assert_eq!(
            parsed,
            Interface::Socket("[::0]:338".parse::<SocketAddr>().expect("???"))
        );
    }

    #[test]
    fn parse_simple_ipv6_no_port() {
        let simple = r#""[::0]""#;
        let parsed: Interface = simple.parse::<Interface>().expect("could not parse");
        assert_eq!(
            parsed,
            Interface::Socket("[::0]:0".parse::<SocketAddr>().expect("???"))
        );
    }

    #[test]
    fn parse_simple_ipv4() {
        let simple = r#"127.0.0.1:338"#;
        let parsed: Interface = simple.parse::<Interface>().expect("could not parse");
        assert_eq!(
            parsed,
            Interface::Socket("127.0.0.1:338".parse::<SocketAddr>().expect("???"))
        )
    }

    #[test]
    fn parse_simple_ipv4_no_port() {
        let simple = r#"127.0.0.1"#;
        let parsed: Interface = simple.parse::<Interface>().expect("could not parse");
        assert_eq!(
            parsed,
            Interface::Socket("127.0.0.1:0".parse::<SocketAddr>().expect("???"))
        )
    }
}
