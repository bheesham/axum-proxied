# axum-proxied

Helpers for running an [axum] service behind a reverse proxy.

Features:

* Extractors for `Forwarded` and `X-Forwarded-For` ([example][ex-extract]);
  and
* a [PROXY v1][proxy] TCP listener ([example][ex-proxy]).

## License

Licensed under whichever suits you best:

* [Apache License, Version 2.0](LICENSE-APACHE) (`Apache-2.0`);
* [MIT License](LICENSE-MIT) (`MIT`).

[axum]: https://github.com/tokio-rs/axum
[proxy]: https://www.haproxy.org/download/1.8/doc/proxy-protocol.txt
[ex-extract]: examples/extract.rs
[ex-proxy]: examples/proxy.rs
