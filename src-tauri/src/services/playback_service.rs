//! 视频详情 / 播放 / 评论 / 互动服务。

use crate::api::Api;
use crate::services::{adapter, to_json};
use serde_json::Value;

pub async fn video(api: &Api, id: i64) -> Result<Value, String> {
    to_json(&adapter(api).video(id).await?)
}

pub async fn play(api: &Api, id: i64, ep: i64) -> Result<Value, String> {
    to_json(&adapter(api).play(id, ep).await?)
}

pub async fn comments(api: &Api, id: i64, ep: i64, page: u32, size: u32) -> Result<Value, String> {
    to_json(&adapter(api).comments(id, ep, page, size).await?)
}

pub async fn comment_send(
    api: &Api,
    id: i64,
    ep: i64,
    text: String,
    reply_id: i64,
) -> Result<Value, String> {
    to_json(&adapter(api).comment_send(id, ep, &text, reply_id).await?)
}

pub async fn like(api: &Api, id: i64, ep: i64, on: bool) -> Result<Value, String> {
    to_json(&adapter(api).like(id, ep, on).await?)
}

pub async fn collect(api: &Api, id: i64, on: bool) -> Result<Value, String> {
    to_json(&adapter(api).collect(id, on).await?)
}

pub async fn follow(api: &Api, uid: i64, on: bool) -> Result<Value, String> {
    to_json(&adapter(api).follow(uid, on).await?)
}

pub async fn related(api: &Api, id: i64) -> Result<Value, String> {
    Ok(serde_json::json!({ "items": adapter(api).related(id).await? }))
}
