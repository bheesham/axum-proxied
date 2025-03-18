use axum::{Router, routing::get};
use axum_proxied::extract;

async fn handler(
    forwarded: Option<extract::forwarded::Forwarded>,
    xforwarded: Option<extract::xforwardedfor::XForwardedFor>,
) -> String {
    if let Some(ref forwarded) = forwarded {
        println!("forwarded: first: {:?}", forwarded.forwards().first());
        for f in forwarded.forwards() {
            println!("forwarded: for {:?}", f.r#for());
        }
    }
    if let Some(ref xforwarded) = xforwarded {
        println!("xforwarded: first: {:?}", xforwarded.forwards().first());
        for f in xforwarded.forwards() {
            println!("xforwarded: for {:?}", f);
        }
    }
    format!("oy, {forwarded:?}, or potentially {xforwarded:?}\n")
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(handler));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
