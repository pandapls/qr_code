use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Redirect},
    routing::{get, post},
};

use crate::{
    domain::{CreateQrCodeRequest, GetQrCodeImageQuery, UpdateQrCodeRequest},
    error::AppError,
    repository::PostgresQrCodeRepository,
    service::QrCodeService,
};

type AppState = Arc<QrCodeService<PostgresQrCodeRepository>>;

pub fn create_router(service: AppState) -> Router {
    Router::new()
        .route("/v1/qr_code", post(create_qr_code))
        .route(
            "/v1/qr_code/{qr_token}",
            get(get_original_url)
                .put(update_qr_code)
                .delete(delete_qr_code),
        )
        .route("/v1/qr_code_image/{qr_token}", get(get_qr_code_image))
        .route("/assets/qr/{qr_token}", get(render_qr_code_image))
        .route("/{qr_token}", get(redirect_by_token))
        .with_state(service)
}

async fn create_qr_code(
    State(service): State<AppState>,
    Json(request): Json<CreateQrCodeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let response = service.create(request).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

async fn get_original_url(
    State(service): State<AppState>,
    Path(qr_token): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let response = service.get_original_url(&qr_token).await?;
    Ok((StatusCode::OK, Json(response)))
}

async fn update_qr_code(
    State(service): State<AppState>,
    Path(qr_token): Path<String>,
    Json(request): Json<UpdateQrCodeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let response = service.update(&qr_token, request).await?;
    Ok((StatusCode::OK, Json(response)))
}

async fn delete_qr_code(
    State(service): State<AppState>,
    Path(qr_token): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    service.delete(&qr_token).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_qr_code_image(
    State(service): State<AppState>,
    Path(qr_token): Path<String>,
    Query(query): Query<GetQrCodeImageQuery>,
) -> Result<impl IntoResponse, AppError> {
    let response = service.get_image_location(&qr_token, &query).await?;
    Ok((StatusCode::OK, Json(response)))
}

async fn render_qr_code_image(
    State(service): State<AppState>,
    Path(qr_token): Path<String>,
    Query(query): Query<GetQrCodeImageQuery>,
) -> Result<impl IntoResponse, AppError> {
    let svg = service.render_svg(&qr_token, &query).await?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")],
        svg,
    ))
}

async fn redirect_by_token(
    State(service): State<AppState>,
    Path(qr_token): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let target_url = service.resolve_redirect(&qr_token).await?;
    Ok(Redirect::temporary(&target_url))
}
