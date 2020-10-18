use std::net::SocketAddr;

use crate::Config;
use actix_web::{get, post, web, App, HttpServer, Responder};
use std::sync::Mutex;

#[get("/ping")]
async fn ping() -> impl Responder {
    "pong"
}

#[post("/connect")]
async fn connect() -> impl Responder {
    "connect"
}

#[post("/disconnect")]
async fn disconnect() -> impl Responder {
    "disconnect"
}

#[get("/discover")]
async fn discover() -> impl Responder {
    "discover"
}

#[get("/")]
async fn info() -> impl Responder {
    "info"
}

struct AppState {
    network_config: Mutex<Config>,
}

pub async fn server(bind: SocketAddr, network_config: Config) -> std::io::Result<()> {
    let state = web::Data::new(AppState {
        network_config: Mutex::new(network_config),
    });
    HttpServer::new(move || {
        App::new()
            .data(state.clone())
            .service(ping)
            .service(connect)
            .service(disconnect)
            .service(discover)
    })
    .bind(bind.to_string().as_str())?
    .run()
    .await
}
