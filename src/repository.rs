use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::QrCode;

#[async_trait]
pub trait QrCodeRepository: Send + Sync {
    async fn create(&self, qr_code: QrCode) -> Result<QrCode, String>;
    async fn get_by_token(&self, token: &str) -> Result<Option<QrCode>, String>;
    async fn update_url(
        &self,
        token: &str,
        new_url: String,
        updated_at: String,
    ) -> Result<Option<QrCode>, String>;
    async fn delete(&self, token: &str) -> Result<bool, String>;
    async fn token_exists(&self, token: &str) -> Result<bool, String>;
}

pub struct PostgresQrCodeRepository {
    pool: PgPool,
}

impl PostgresQrCodeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl QrCodeRepository for PostgresQrCodeRepository {
    async fn create(&self, qr_code: QrCode) -> Result<QrCode, String> {
        let row = sqlx::query_as::<_, (i64,)>(
            r#"
            INSERT INTO qr_codes (user_id, qr_token, url, created_at, updated_at)
            VALUES ($1, $2, $3, NOW(), NOW())
            RETURNING id
            "#,
        )
        .bind(qr_code.user_id.map(|id| id as i64))
        .bind(&qr_code.qr_token)
        .bind(&qr_code.url)
        .fetch_one(&self.pool)
        .await
        .map_err(|err| err.to_string())?;

        Ok(QrCode {
            id: row.0 as u64,
            ..qr_code
        })
    }

    async fn get_by_token(&self, token: &str) -> Result<Option<QrCode>, String> {
        let row = sqlx::query_as::<_, (i64, Option<i64>, String, String, String, String)>(
            r#"
            SELECT id, user_id, qr_token, url, created_at::text, updated_at::text
            FROM qr_codes
            WHERE qr_token = $1
            "#,
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| err.to_string())?;

        Ok(row.map(
            |(id, user_id, qr_token, url, created_at, updated_at)| QrCode {
                id: id as u64,
                user_id: user_id.map(|value| value as u64),
                qr_token,
                url,
                created_at,
                updated_at,
            },
        ))
    }

    async fn update_url(
        &self,
        token: &str,
        new_url: String,
        updated_at: String,
    ) -> Result<Option<QrCode>, String> {
        let row = sqlx::query_as::<_, (i64, Option<i64>, String, String, String, String)>(
            r#"
            UPDATE qr_codes
            SET url = $2, updated_at = NOW()
            WHERE qr_token = $1
            RETURNING id, user_id, qr_token, url, created_at::text, updated_at::text
            "#,
        )
        .bind(token)
        .bind(new_url)
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| err.to_string())?;

        let _ = updated_at;

        Ok(row.map(
            |(id, user_id, qr_token, url, created_at, updated_at)| QrCode {
                id: id as u64,
                user_id: user_id.map(|value| value as u64),
                qr_token,
                url,
                created_at,
                updated_at,
            },
        ))
    }

    async fn delete(&self, token: &str) -> Result<bool, String> {
        let result = sqlx::query(
            r#"
            DELETE FROM qr_codes
            WHERE qr_token = $1
            "#,
        )
        .bind(token)
        .execute(&self.pool)
        .await
        .map_err(|err| err.to_string())?;

        Ok(result.rows_affected() > 0)
    }

    async fn token_exists(&self, token: &str) -> Result<bool, String> {
        let row = sqlx::query_as::<_, (i64,)>(
            r#"
            SELECT 1
            FROM qr_codes
            WHERE qr_token = $1
            LIMIT 1
            "#,
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| err.to_string())?;

        Ok(row.is_some())
    }
}
