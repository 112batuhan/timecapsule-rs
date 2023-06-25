pub mod api;
pub mod entities;
pub mod queries;
pub mod resend;

use std::net::SocketAddr;
use std::time::Duration;

use api::authentication::{auto_login, check_session_token, login, logout, sign_up};
use api::emails::{create_email, get_emails, send_demo_email};
use api::SharedState;
use axum::error_handling::HandleErrorLayer;
use axum::http::{header, Method};
use axum::routing::{delete, get, post};
use axum::{middleware, BoxError, Router};
use dotenv::dotenv;
use tower::ServiceBuilder;
use tower_governor::errors::display_error;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_governor::GovernorLayer;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let state = SharedState::new()
        .await
        .expect("Failed to initiate Shared State");

    let origins = [std::env::var("FRONTEND_URL").unwrap().parse().unwrap()];

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::COOKIE, header::ALLOW, header::CONTENT_TYPE])
        .allow_origin(origins)
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600));

    let governor_conf = Box::new(
        GovernorConfigBuilder::default()
            .per_second(20)
            .burst_size(5)
            .key_extractor(SmartIpKeyExtractor)
            .use_headers()
            .finish()
            .unwrap(),
    );

    let app = Router::new()
        .route("/send_email/:email_id", get(send_demo_email))
        .route("/auto_login", get(auto_login))
        .route("/get_emails", get(get_emails))
        .route("/create_email", post(create_email))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            check_session_token,
        ))
        .route("/logout", delete(logout))
        .route("/sign_up", post(sign_up))
        .route("/login", post(login))
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|e: BoxError| async move {
                    display_error(e)
                }))
                .layer(GovernorLayer {
                    // We can leak this because it is created once and then
                    config: Box::leak(governor_conf),
                }),
        )
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from((
        [0, 0, 0, 0],
        std::env::var("PORT").unwrap().parse::<u16>().unwrap(),
    ));

    tracing::info!("Started serving on: {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .expect("Failed to start server");
}
