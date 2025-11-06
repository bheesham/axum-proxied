//! Extracts the [Forwarded][docs-forwarded] and
//! [X-Forwarded-For][docs-forwarded-for] header.
//!
//! [docs-forwarded]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Forwarded
//! [docs-forwarded-for]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/X-Forwarded-For

pub mod forwarded;
pub mod xforwardedfor;

pub use crate::extract::forwarded::*;
pub use crate::extract::xforwardedfor::*;
