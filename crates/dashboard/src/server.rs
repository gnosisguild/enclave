use crate::actor::{DashboardActor, GetMetrics, QueryPersistedEvents};
use crate::tracing_layer::{LogEntry, SharedLogBuffer};
use actix::Addr;
use actix_web::{web, App, HttpResponse, HttpServer};
use anyhow::Result;
use serde::{Deserialize, Serialize};

struct AppState {
    log_buffer: SharedLogBuffer,
    dashboard_actor: Addr<DashboardActor>,
}

#[derive(Deserialize)]
struct LogQuery {
    level: Option<String>,
    search: Option<String>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct EventQuery {
    #[serde(rename = "type")]
    event_type: Option<String>,
    e3_id: Option<String>,
    limit: Option<u64>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

async fn index() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("dashboard.html"))
}

async fn get_logs(state: web::Data<AppState>, query: web::Query<LogQuery>) -> HttpResponse {
    let limit = query.limit.unwrap_or(500);

    // Clone filtered entries and release the lock before serialization
    // to avoid blocking the tracing layer.
    let logs: Vec<LogEntry> = {
        let buf = state.log_buffer.lock();
        buf.iter()
            .rev()
            .filter(|entry| {
                if let Some(ref level) = query.level {
                    if !level.is_empty() && entry.level != *level {
                        return false;
                    }
                }
                if let Some(ref search) = query.search {
                    if !search.is_empty() {
                        let search_lower = search.to_lowercase();
                        if !entry.message.to_lowercase().contains(&search_lower)
                            && !entry.target.to_lowercase().contains(&search_lower)
                        {
                            return false;
                        }
                    }
                }
                true
            })
            .take(limit)
            .cloned()
            .collect()
    };

    HttpResponse::Ok().json(logs)
}

async fn get_events(state: web::Data<AppState>, query: web::Query<EventQuery>) -> HttpResponse {
    let result = state
        .dashboard_actor
        .send(QueryPersistedEvents {
            event_type: query.event_type.clone(),
            e3_id: query.e3_id.clone(),
            limit: query.limit,
        })
        .await;

    match result {
        Ok(events) => HttpResponse::Ok().json(events),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Actor error: {}", e),
        }),
    }
}

async fn get_metrics(state: web::Data<AppState>) -> HttpResponse {
    let result = state.dashboard_actor.send(GetMetrics).await;

    match result {
        Ok(metrics) => HttpResponse::Ok().json(metrics),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Actor error: {}", e),
        }),
    }
}

pub async fn start_dashboard_server(
    port: u16,
    log_buffer: SharedLogBuffer,
    dashboard_actor: Addr<DashboardActor>,
) -> Result<u16> {
    let state = web::Data::new(AppState {
        log_buffer,
        dashboard_actor,
    });

    let server = HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/", web::get().to(index))
            .route("/api/logs", web::get().to(get_logs))
            .route("/api/events", web::get().to(get_events))
            .route("/api/metrics", web::get().to(get_metrics))
    })
    .bind(format!("127.0.0.1:{}", port))?;

    let addrs = server.addrs();
    let actual_port = addrs.first().map(|a| a.port()).unwrap_or(port);

    // server.run() spawns workers on the actix system and returns a Server handle.
    // We spawn it on the actix local runtime (doesn't require Send).
    let server_handle = server.run();
    actix_web::rt::spawn(async move {
        if let Err(e) = server_handle.await {
            tracing::error!("Dashboard server error: {}", e);
        }
    });

    Ok(actual_port)
}
