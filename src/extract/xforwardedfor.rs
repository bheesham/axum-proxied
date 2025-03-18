//! Support for HTTP Header [`X-Forwarded-For`][mdn].
//!
//! [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/X-Forwarded-For
use axum::body::Body;
use axum::extract::OptionalFromRequestParts;
use axum::http::request::Parts;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use std::net::IpAddr;

/// Get the contents of the `X-Forwarded-For` header.
///
/// Example:
///
/// ```rust
/// use axum_proxied::extract::xforwardedfor::XForwardedFor;
///
/// async fn handler(xforwarded: Option<XForwardedFor>) {
///     todo!()
/// }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct XForwardedFor {
    ip_addresses: Vec<IpAddr>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct XForwardedForRejection(&'static str);

impl IntoResponse for XForwardedForRejection {
    fn into_response(self) -> Response<Body> {
        (StatusCode::BAD_REQUEST, self.0).into_response()
    }
}

impl XForwardedFor {
    pub fn new(ip_addresses: Vec<std::net::IpAddr>) -> Self {
        Self { ip_addresses }
    }
}

const X_FORWARDED_FOR_HEADER: header::HeaderName =
    header::HeaderName::from_static("x-forwarded-for");

impl<S> OptionalFromRequestParts<S> for XForwardedFor
where
    S: Send + Sync,
{
    type Rejection = XForwardedForRejection;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        let Some(header_raw) = parts.headers.get(X_FORWARDED_FOR_HEADER) else {
            return Ok(None);
        };
        let Ok(header_str) = header_raw.to_str() else {
            return Err(XForwardedForRejection("could not parse header into string"));
        };
        let mut ip_addresses = vec![];
        let ips_raw = header_str.split(',');
        for ip_raw in ips_raw {
            let Ok(ip) = ip_raw.trim().parse::<std::net::IpAddr>() else {
                return Err(XForwardedForRejection(
                    "could not parse IP in HTTP Header X-Forwarded-For (axum-stuff)",
                ));
            };
            ip_addresses.push(ip);
        }
        Ok(Some(XForwardedFor::new(ip_addresses)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// From
    /// [MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Forwarded).
    #[tokio::test]
    async fn simple() {
        let (mut parts, _) = axum::http::request::Builder::new()
            .method("GET")
            .header("X-Forwarded-For", "192.0.2.43, 2001:db8:cafe::17")
            .body(())
            .expect("could not build request")
            .into_parts();
        let xforwarded = XForwardedFor::from_request_parts(&mut parts, &())
            .await
            .expect("could not parse HTTP headers")
            .expect("could not parse X-Forwarded-For header");
        assert_eq!(
            xforwarded,
            XForwardedFor {
                ip_addresses: vec![
                    "192.0.2.43".parse::<IpAddr>().expect("???"),
                    "2001:db8:cafe::17".parse::<IpAddr>().expect("???"),
                ]
            }
        );
    }
}
