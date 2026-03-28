mod api;
mod domain;
mod error;
mod repository;
mod service;

use std::sync::Arc;

use repository::PostgresQrCodeRepository;
use service::QrCodeService;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() {
    dotenvy::from_path(format!("{}/.env", env!("CARGO_MANIFEST_DIR")))
        .expect("failed to load .env");

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect to postgres");
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    let server_addr = std::env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());

    let repository = Arc::new(PostgresQrCodeRepository::new(pool));
    let service = Arc::new(QrCodeService::new(repository, base_url));

    let app = api::create_router(service);

    let listener = tokio::net::TcpListener::bind(&server_addr).await.unwrap();

    println!("database connected");
    println!("server listening on http://{}", server_addr);

    axum::serve(listener, app).await.unwrap();
}
