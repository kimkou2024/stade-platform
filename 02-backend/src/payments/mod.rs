//! Payment gateway — SATIM e-payment for CIB & DAHABIA cards [M §7.4].
//!
//! In Algeria both CIB and DAHABIA are cleared through SATIM's gateway, which
//! uses the classic register / getOrderStatus REST flow:
//!   1. register.do           → returns orderId + formUrl (hosted card page)
//!   2. (cardholder pays on SATIM's page — no PAN/CVV ever touches us [M §10.4])
//!   3. getOrderStatusExtended → orderStatus == 2 means paid (deposited)
//! Currency is 012 (DZD); amount is sent in centimes (× 100).
//!
//! Mode is config-driven: when SATIM credentials are absent (the API is not yet
//! provisioned), `mode == "mock"` and `pay_order` issues immediately so dev and
//! tests work. When credentials are set, the real redirect flow is used and
//! tickets are issued only after the return callback confirms payment.

use rust_decimal::prelude::ToPrimitive;
use serde::Deserialize;

use crate::config::Config;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq)]
pub enum Mode { Mock, Satim }

pub fn mode(cfg: &Config) -> Mode {
    if cfg.payment_mode == "satim" { Mode::Satim } else { Mode::Mock }
}

/// SATIM `register.do` response.
#[derive(Debug, Deserialize)]
pub struct RegisterResp {
    #[serde(rename = "orderId")]
    pub order_id: Option<String>,
    #[serde(rename = "formUrl")]
    pub form_url: Option<String>,
    #[serde(rename = "errorCode")]
    pub error_code: Option<String>,
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
}

/// SATIM `getOrderStatusExtended.do` response (subset).
#[derive(Debug, Deserialize)]
pub struct StatusResp {
    #[serde(rename = "orderStatus")]
    pub order_status: Option<i64>,
    #[serde(rename = "errorCode")]
    pub error_code: Option<String>,
    #[serde(rename = "actionCode")]
    pub action_code: Option<i64>,
}

fn amount_centimes(amount_dzd: rust_decimal::Decimal) -> i64 {
    (amount_dzd * rust_decimal::Decimal::from(100)).to_i64().unwrap_or(0)
}

/// Register an order with SATIM; returns (satimOrderId, formUrl) to redirect to.
pub async fn register(
    cfg: &Config,
    local_order_ref: &str,
    amount_dzd: rust_decimal::Decimal,
) -> AppResult<(String, String)> {
    let return_url = format!("{}/api/payments/satim/return", cfg.return_base_url.trim_end_matches('/'));
    let fail_url = format!("{}/api/payments/satim/return", cfg.return_base_url.trim_end_matches('/'));
    let params = [
        ("userName", cfg.satim_username.as_str()),
        ("password", cfg.satim_password.as_str()),
        ("orderNumber", local_order_ref),
        ("amount", &amount_centimes(amount_dzd).to_string()),
        ("currency", "012"),
        ("returnUrl", &return_url),
        ("failUrl", &fail_url),
        ("language", "fr"),
    ];
    let url = format!("{}/register.do", cfg.satim_base_url.trim_end_matches('/'));
    let resp: RegisterResp = reqwest::Client::new()
        .post(&url)
        .form(&params)
        .send()
        .await
        .map_err(|e| { tracing::error!("satim register: {e}"); AppError::Internal })?
        .json()
        .await
        .map_err(|e| { tracing::error!("satim register decode: {e}"); AppError::Internal })?;

    match (resp.order_id, resp.form_url, resp.error_code.as_deref()) {
        (Some(oid), Some(form), Some("0") | None) => Ok((oid, form)),
        _ => {
            tracing::error!("satim register rejected: {:?}", resp.error_message);
            Err(AppError::BadRequest("payment registration refused by gateway".into()))
        }
    }
}

/// Confirm an order's status with SATIM. Returns true when paid (status 2).
pub async fn is_paid(cfg: &Config, satim_order_id: &str) -> AppResult<bool> {
    let params = [
        ("userName", cfg.satim_username.as_str()),
        ("password", cfg.satim_password.as_str()),
        ("orderId", satim_order_id),
        ("language", "fr"),
    ];
    let url = format!("{}/getOrderStatusExtended.do", cfg.satim_base_url.trim_end_matches('/'));
    let resp: StatusResp = reqwest::Client::new()
        .post(&url)
        .form(&params)
        .send()
        .await
        .map_err(|e| { tracing::error!("satim status: {e}"); AppError::Internal })?
        .json()
        .await
        .map_err(|e| { tracing::error!("satim status decode: {e}"); AppError::Internal })?;
    Ok(resp.order_status == Some(2))
}
