use axum::{Router, routing::get};
use axum_proxied::extract;

async fn handler(
    forwarded: Option<extract::forwarded::Forwarded>,
    xforwarded: Option<extract::xforwardedfor::XForwardedFor>,
) -> String {
    format!("oy, {forwarded:?}, or potentially {xforwarded:?}\n")
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(handler));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
