use std::net::SocketAddr;

use crate::{Config, Event};
use actix_web::{get, post, web, App, HttpServer, Responder};
use lru::LruCache;
use std::sync::Mutex;
use uuid::Uuid;

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

#[post("/events")]
async fn new_event() -> impl Responder {
    "new_event"
}

#[get("/events")]
async fn list_events() -> impl Responder {
    "list_events"
}

struct AppState {
    network_config: Mutex<Config>,
    events: Mutex<LruCache<Uuid, Event>>,
}

pub async fn server(bind: SocketAddr, network_config: Config) -> std::io::Result<()> {
    let state = web::Data::new(AppState {
        network_config: Mutex::new(network_config),
        events: Mutex::new(LruCache::new(1000)),
    });
    HttpServer::new(move || {
        App::new()
            .data(state.clone())
            .service(info)
            .service(ping)
            .service(connect)
            .service(disconnect)
            .service(discover)
            .service(new_event)
            .service(list_events)
    })
    .bind(bind.to_string().as_str())?
    .run()
    .await
}
