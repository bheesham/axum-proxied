use axum::{Router, extract::ConnectInfo, routing::get};
use axum_proxied::proxy;

async fn handler(ConnectInfo(addr): ConnectInfo<proxy::Addr>) -> String {
    format!("yo, {addr:?}\n")
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(handler));
    let listener: proxy::Listener<proxy::V1> = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap()
        .into();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<proxy::Addr>(),
    )
    .await
    .unwrap();
}
