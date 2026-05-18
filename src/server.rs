use axum::{http::StatusCode, middleware, routing::get, Json, Router};
use mongodb::Database;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use utoipa::OpenApi;
use utoipa_swagger_ui::{Config as SwaggerConfig, SwaggerUi};

use crate::admin;
use crate::config::CONFIG;
use crate::content;
use crate::game;
use crate::leaderboard;
use crate::marketplace;
use crate::moments;
use crate::moments::social_media;
use crate::moments::social_media::worker::scrape_job::SCRAPE_QUEUE;
use crate::moments::MIGRATION_QUEUE;
use crate::player;
use crate::redis::{connect as valkey_connect, ValkyQueue};
use crate::telegram;

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Health check response", body = crate::openapi::HealthResponse)
    ),
    tag = "Health"
)]
pub(crate) async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "ok": true,
        "ts": chrono::Utc::now().to_rfc3339()
    }))
}

/// Fallback handler for unmatched routes - returns structured 404
async fn fallback() -> (StatusCode, Json<serde_json::Value>) {
    tracing::warn!("Route not found");
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "ok": false,
            "message": "Route not found"
        })),
    )
}

use crate::referral;
use std::sync::Arc;

/// Build the application router with all routes
async fn build_router(db: Database) -> Router {
    let client = db.client().clone();
    let player_repo = Arc::new(player::PlayerRepository::new(&db));
    let mut anti_fraud_service = None;
    let mut migration_queue = None;
    let mut scrape_queue = None;
    let mut referral_redis_client = None;
    let mut click_analytics = Arc::new(referral::analytics::ClickAnalyticsService::new(None));

    // Connect to Valkey for queues
    match valkey_connect().await {
        Ok(valkey_client) => {
            tracing::info!("Connected to Valkey for queues");

            // Queue instances — async multiplexed connections
            let m_queue = ValkyQueue::new(valkey_client.clone(), MIGRATION_QUEUE.as_str())
                .await
                .expect("Failed to create migration queue connection");
            let s_queue = ValkyQueue::new(valkey_client.clone(), SCRAPE_QUEUE.as_str())
                .await
                .expect("Failed to create scrape queue connection");
            let c_queue = ValkyQueue::new(valkey_client.clone(), referral::CLICK_QUEUE.as_str())
                .await
                .expect("Failed to create click queue connection");
            let v_queue = ValkyQueue::new(valkey_client.clone(), referral::VERIFY_QUEUE.as_str())
                .await
                .expect("Failed to create verify queue connection");

            migration_queue = Some(m_queue);
            scrape_queue = Some(s_queue);
            referral_redis_client = Some(valkey_client.clone());
            anti_fraud_service = Some(Arc::new(referral::anti_fraud::AntiFraudService::new(
                valkey_client,
                v_queue,
            )));
            click_analytics = Arc::new(referral::analytics::ClickAnalyticsService::new(Some(
                c_queue,
            )));
        }
        Err(e) => {
            tracing::warn!(error = %e, "Valkey not available — queues disabled");
        }
    }

    let referral_service = Arc::new(referral::service::ReferralService::new(
        player_repo,
        referral_redis_client,
    ));

    // HTTP request/response tracing middleware
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO));

    // CORS Middleware
    let cors = if CONFIG.app.cors_origins.len() == 1 && CONFIG.app.cors_origins[0] == "*" {
        tower_http::cors::CorsLayer::new()
            .allow_origin(tower_http::cors::Any)
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any)
    } else {
        let origins: Vec<axum::http::HeaderValue> = CONFIG
            .app
            .cors_origins
            .iter()
            .map(|s| s.parse().unwrap())
            .collect();
        tower_http::cors::CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any)
            .allow_credentials(true)
    };

    let mut api_router = Router::new()
        .route("/health", get(health_check))
        .nest("/content", content::routes(db.clone()))
        .nest("/games", game::routes(db.clone()))
        .nest(
            "/leaderboard",
            leaderboard::routes(db.clone(), client.clone()),
        )
        .nest(
            "/player",
            player::routes(db.clone(), client, anti_fraud_service),
        )
        .nest("/marketplace", marketplace::routes(db.clone()))
        .nest("/moments", moments::routes(db.clone(), migration_queue))
        .nest(
            "/moments/social-media",
            social_media::route::routes(db.clone(), scrape_queue),
        )
        .nest(
            "/upload",
            axum::Router::new().route(
                "/presign",
                axum::routing::post(crate::upload::controller::generate_presigned_url),
            ),
        )
        .nest(
            "/referral",
            referral::route::router().with_state(referral_service),
        )
        .nest(
            "/r",
            referral::redirect_route::router()
                .with_state(referral::redirect_route::RedirectAppState { click_analytics }),
        )
        .nest("/telegram", telegram::routes());

    if CONFIG.app.admin_routes_enabled() {
        tracing::warn!(
            environment = %CONFIG.app.environment,
            "Admin routes enabled"
        );
        api_router = api_router.nest("/admin", admin::routes(db.clone()));
    } else {
        tracing::info!(
            environment = %CONFIG.app.environment,
            "Admin routes disabled"
        );
        api_router = api_router.nest("/admin", Router::new().fallback(fallback));
    }

    let api_router = api_router.layer(middleware::from_fn(crate::middleware::localize_response));

    // Rate limiting: 30 requests per second per IP, burst up to 60
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(2) // Refill interval: 1 token every 2 seconds → ~30/min sustained
        .burst_size(60) // Allow bursts up to 60 requests
        .finish()
        .expect("Failed to build rate limiter config");
    let governor_limiter = governor_conf.limiter().clone();
    let governor_layer = GovernorLayer {
        config: std::sync::Arc::new(governor_conf),
    };

    // Spawn a background task to clean up rate limiter state for expired IPs
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            governor_limiter.retain_recent();
        }
    });

    api_router
        .merge(
            SwaggerUi::new("/docs")
                .url("/openapi.json", crate::openapi::ApiDoc::openapi())
                .config(SwaggerConfig::new(["openapi.json"])),
        )
        .fallback(fallback)
        .layer(governor_layer)
        .layer(trace_layer)
        .layer(cors)
}

/// Start the HTTP server
pub async fn run(
    db: Database,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<(), std::io::Error> {
    let app = build_router(db).await;

    let host = &CONFIG.app.host;
    let port = CONFIG.app.port;
    let addr = format!("{}:{}", host, port);

    tracing::info!(host = %host, port = %port, "Server starting");

    let listener = TcpListener::bind(&addr).await?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        let _ = shutdown_rx.changed().await;
        tracing::info!("HTTP Server received shutdown signal.");
    })
    .await
}
