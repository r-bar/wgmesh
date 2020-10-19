use std::fmt;
use std::net::SocketAddr;
use std::sync::{Arc, MutexGuard};

use actix_web::{
    dev::HttpResponseBuilder, error, get, http::header, http::StatusCode, middleware, post, web,
    App, HttpResponse, HttpServer, Responder,
};
use chrono::Utc;
use lru::LruCache;
use std::sync::Mutex;
use uuid::Uuid;

use crate::{Config, Event, EventData, Host};

/// Quickly return a web service error with a status code and message
#[derive(Debug, Clone)]
struct ServiceError(u16, &'static str);

struct AppState {
    network_config: Config,
    events: LruCache<Uuid, Event>,
}

type State = web::Data<Arc<Mutex<AppState>>>;

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.0, self.1)
    }
}

impl error::ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        HttpResponseBuilder::new(self.status_code())
            .set_header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.0).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

#[get("/ping")]
async fn ping() -> impl Responder {
    "pong"
}

#[post("/connect")]
async fn connect(state: State, host: web::Json<Host>) -> error::Result<impl Responder> {
    let mut state = state
        .lock()
        .map_err(|_| ServiceError(500, "Unable to access app state"))?;
    let mut host = host.into_inner();
    let output = format!("connect {}: {}", &host.name, &host.wireguard_address);

    let event = Event::connect(host.clone());
    state.events.put(event.id, event);

    match state
        .network_config
        .remote_hosts
        .get_mut(&host.wireguard_address)
    {
        Some(entry) => {
            host.last_seen = Some(Utc::now());
            *entry = host;
        }
        None => {}
    }

    Ok(output)
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
async fn info(state: State) -> error::Result<impl Responder> {
    let state = state
        .lock()
        .map_err(|_| ServiceError(500, "unable to get app state"))?;
    Ok(web::Json(state.network_config.clone()))
}

#[post("/events")]
async fn new_event() -> impl Responder {
    "new_event"
}

#[get("/events")]
async fn list_events() -> impl Responder {
    "list_events"
}

pub async fn server(bind: SocketAddr, network_config: Config) -> std::io::Result<()> {
    let state = Arc::new(Mutex::new(AppState {
        network_config,
        events: LruCache::new(1000),
    }));
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
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
