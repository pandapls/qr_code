use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use qrcode::{QrCode as SvgQrCode, render::svg};
use rand::{Rng, distr::Alphanumeric, rng};

use crate::{
    domain::{
        CreateQrCodeRequest, CreateQrCodeResponse, GetQrCodeImageQuery, GetQrCodeImageResponse,
        GetQrCodeResponse, QrCode, UpdateQrCodeRequest,
    },
    error::AppError,
    repository::QrCodeRepository,
};

const TOKEN_LENGTH: usize = 8;
const MAX_TOKEN_RETRIES: usize = 10;

pub struct QrCodeService<R: QrCodeRepository> {
    repository: Arc<R>,
    base_url: String,
}

impl<R: QrCodeRepository> QrCodeService<R> {
    pub fn new(repository: Arc<R>, base_url: String) -> Self {
        Self {
            repository,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn create(
        &self,
        request: CreateQrCodeRequest,
    ) -> Result<CreateQrCodeResponse, AppError> {
        let url = normalize_url(&request.url)?;
        let qr_token = self.generate_unique_token().await?;
        let now = current_timestamp();

        let qr_code = QrCode {
            id: current_id(),
            user_id: None,
            qr_token: qr_token.clone(),
            url,
            created_at: now.clone(),
            updated_at: now,
        };

        self.repository
            .create(qr_code)
            .await
            .map_err(AppError::Database)?;

        Ok(CreateQrCodeResponse { qr_token })
    }

    pub async fn get_original_url(&self, token: &str) -> Result<GetQrCodeResponse, AppError> {
        let qr_code = self
            .repository
            .get_by_token(token)
            .await
            .map_err(AppError::Database)?
            .ok_or(AppError::NotFound)?;

        Ok(GetQrCodeResponse { url: qr_code.url })
    }

    pub async fn update(
        &self,
        token: &str,
        request: UpdateQrCodeRequest,
    ) -> Result<GetQrCodeResponse, AppError> {
        let url = normalize_url(&request.url)?;
        let updated = self
            .repository
            .update_url(token, url, current_timestamp())
            .await
            .map_err(AppError::Database)?
            .ok_or(AppError::NotFound)?;

        Ok(GetQrCodeResponse { url: updated.url })
    }

    pub async fn delete(&self, token: &str) -> Result<(), AppError> {
        if self
            .repository
            .delete(token)
            .await
            .map_err(AppError::Database)?
        {
            Ok(())
        } else {
            Err(AppError::NotFound)
        }
    }

    pub async fn get_image_location(
        &self,
        token: &str,
        query: &GetQrCodeImageQuery,
    ) -> Result<GetQrCodeImageResponse, AppError> {
        self.repository
            .get_by_token(token)
            .await
            .map_err(AppError::Database)?
            .ok_or(AppError::NotFound)?;

        let dimension = query.dimension.unwrap_or(300);
        let color = query.color.clone().unwrap_or_else(|| "000000".to_string());
        let border = query.border.unwrap_or(10);
        validate_hex_color(&color)?;

        Ok(GetQrCodeImageResponse {
            image_location: format!(
                "{}/assets/qr/{}?dimension={}&color={}&border={}",
                self.base_url, token, dimension, color, border
            ),
        })
    }

    pub async fn render_svg(
        &self,
        token: &str,
        query: &GetQrCodeImageQuery,
    ) -> Result<String, AppError> {
        let qr_code = self
            .repository
            .get_by_token(token)
            .await
            .map_err(AppError::Database)?
            .ok_or(AppError::NotFound)?;

        let dimension = query.dimension.unwrap_or(300);
        let color = query.color.clone().unwrap_or_else(|| "000000".to_string());
        let border = query.border.unwrap_or(10);
        let dark_color = validate_hex_color(&color)?;
        let light_color = "#FFFFFF";
        let quiet_zone = border > 0;
        let encoded_url = format!("{}/{}", self.base_url, qr_code.qr_token);

        SvgQrCode::new(encoded_url.as_bytes())
            .map(|code| {
                code.render::<svg::Color<'_>>()
                    .min_dimensions(dimension, dimension)
                    .quiet_zone(quiet_zone)
                    .dark_color(svg::Color(dark_color.as_str()))
                    .light_color(svg::Color(light_color))
                    .build()
            })
            .map_err(|_| AppError::RenderFailed)
    }

    pub async fn resolve_redirect(&self, token: &str) -> Result<String, AppError> {
        let qr_code = self
            .repository
            .get_by_token(token)
            .await
            .map_err(AppError::Database)?
            .ok_or(AppError::NotFound)?;
        Ok(qr_code.url)
    }

    async fn generate_unique_token(&self) -> Result<String, AppError> {
        for _ in 0..MAX_TOKEN_RETRIES {
            let qr_token: String = rng()
                .sample_iter(Alphanumeric)
                .take(TOKEN_LENGTH)
                .map(char::from)
                .collect();

            if !self
                .repository
                .token_exists(&qr_token)
                .await
                .map_err(AppError::Database)?
            {
                return Ok(qr_token);
            }
        }

        Err(AppError::TokenGenerationFailed)
    }
}

fn normalize_url(raw: &str) -> Result<String, AppError> {
    let parsed = url::Url::parse(raw.trim()).map_err(|_| AppError::InvalidUrl)?;

    match parsed.scheme() {
        "http" | "https" => Ok(parsed.to_string()),
        _ => Err(AppError::InvalidUrl),
    }
}

fn validate_hex_color(raw: &str) -> Result<String, AppError> {
    let value = raw.trim().trim_start_matches('#');
    let is_valid = value.len() == 6 && value.chars().all(|ch| ch.is_ascii_hexdigit());

    if is_valid {
        Ok(format!("#{}", value.to_uppercase()))
    } else {
        Err(AppError::InvalidColor)
    }
}

fn current_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_secs();
    seconds.to_string()
}

fn current_id() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_micros() as u64
}
