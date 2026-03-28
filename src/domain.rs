use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct QrCode {
    pub id: u64,
    pub user_id: Option<u64>,
    pub qr_token: String,
    pub url: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateQrCodeRequest {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateQrCodeRequest {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct GetQrCodeImageQuery {
    pub dimension: Option<u32>,
    pub color: Option<String>,
    pub border: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct CreateQrCodeResponse {
    pub qr_token: String,
}

#[derive(Debug, Serialize)]
pub struct GetQrCodeResponse {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct GetQrCodeImageResponse {
    pub image_location: String,
}
