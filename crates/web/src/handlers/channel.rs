use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use program1_contracts::{
    AuditLogEntry, ChannelStatusDto, ChannelType,
};

pub async fn list_channels(
    State(state): State<AppState>,
) -> Result<Json<Vec<ChannelStatusDto>>, ApiError> {
    let channels = state.channel_contract.get_channel_statuses().await?;
    Ok(Json(channels))
}

pub async fn sync_channel(
    Path(channel_name): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let channel = match channel_name.to_lowercase().as_str() {
        "tiktok" | "tiktokshop" => ChannelType::TikTokShop,
        "shopee" => ChannelType::Shopee,
        "tokopedia" => ChannelType::Tokopedia,
        _ => ChannelType::NativeWeb,
    };

    let count = state.channel_contract.sync_channel_stock(channel.clone()).await?;

    let _ = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: None,
        actor_username: "system".to_string(),
        action: "CHANNEL_SYNCED".to_string(),
        resource_type: "channel".to_string(),
        resource_id: None,
        details: json!({ "channel": channel.to_string(), "synced_products": count }).to_string(),
        ip_address: None,
    }).await;

    Ok(Json(json!({ "synced_products": count })))
}
