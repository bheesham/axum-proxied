# axum-proxied

[![CI](https://github.com/bheesham/axum-proxied/actions/workflows/ci.yml/badge.svg?branch=master)][ci]
[![Crates.io](https://img.shields.io/crates/v/axum-proxied)][crate]
[![Documentation](https://docs.rs/axum-proxied/badge.svg)][docs]

Helpers for running an [axum] service behind a reverse proxy.

Features:

* Extractors for `Forwarded` and `X-Forwarded-For` ([example][ex-extract]);
  and
* a [PROXY][proxy] TCP listener ([example][ex-proxy]).

## Disclaimer

I don't actually use this, I just wrote it for fun.

## License

Licensed under whichever suits you best:

* [Apache License, Version 2.0](LICENSE-APACHE) (`Apache-2.0`);
* [MIT License](LICENSE-MIT) (`MIT`).

[axum]: https://github.com/tokio-rs/axum
[proxy]: https://www.haproxy.org/download/1.8/doc/proxy-protocol.txt
[ex-extract]: https://github.com/bheesham/axum-proxied/blob/master/examples/extract.rs
[ex-proxy]: https://github.com/bheesham/axum-proxied/blob/master/examples/proxy.rs
[ci]: https://github.com/bheesham/axum-proxied/actions
[crate]: https://crates.io/crates/axum-proxied
[docs]: https://docs.rs/axum-proxied/
