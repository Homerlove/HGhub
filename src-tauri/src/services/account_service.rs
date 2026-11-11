//! 账号与用户资料服务。

use crate::adapters::edge_v1::EdgeV1Adapter;
use crate::api::Api;
use crate::infra::source;
use crate::services::{adapter, to_json};
use serde_json::{json, Value};

pub async fn login(api: &Api, username: String, password: String) -> Result<Value, String> {
    let origin = source::base();
    if origin.is_empty() {
        return Err("请先设置接入地址".into());
    }
    let client = EdgeV1Adapter::new(origin.clone(), None);
    let auth = client.login(&username, &password).await?;
    if source::base() != origin {
        return Err("登录期间接入地址已变化".into());
    }
    let user = to_json(&auth.user)?;
    api.session.set(&origin, auth.token, user.clone());
    Ok(json!({ "user": user }))
}

pub async fn register(
    api: &Api,
    username: String,
    password: String,
    confirm_password: String,
    email: Option<String>,
) -> Result<Value, String> {
    let origin = source::base();
    if origin.is_empty() {
        return Err("请先设置接入地址".into());
    }
    let client = EdgeV1Adapter::new(origin.clone(), None);
    let auth = client
        .register(&username, &password, &confirm_password, email.as_deref())
        .await?;
    if source::base() != origin {
        return Err("注册期间接入地址已变化".into());
    }
    let user = to_json(&auth.user)?;
    api.session.set(&origin, auth.token, user.clone());
    Ok(json!({ "user": user }))
}

pub async fn logout(api: &Api) -> Result<Value, String> {
    let client = adapter(api);
    let _ = client.logout().await;
    api.session.clear();
    Ok(json!({ "ok": true }))
}

pub async fn me(api: &Api) -> Result<Value, String> {
    let origin = source::base();
    let client = adapter(api);
    let user = client.me().await?;
    let user = to_json(&user)?;
    if source::base() == origin {
        if let Some(token) = api.session.token(&origin) {
            api.session.set(&origin, token, user.clone());
        }
    }
    Ok(user)
}

pub async fn following(api: &Api, page: u32, size: u32) -> Result<Value, String> {
    let page = adapter(api).following(page, size).await?;
    to_json(&page)
}

pub async fn user(api: &Api, id: i64) -> Result<Value, String> {
    let user = adapter(api).user(id).await?;
    to_json(&user)
}

pub async fn user_stats(api: &Api, id: i64) -> Result<Value, String> {
    let stats = adapter(api).user_stats(id).await?;
    Ok(json!(stats))
}

pub async fn user_videos(api: &Api, id: i64, page: u32, size: u32) -> Result<Value, String> {
    let page = adapter(api).user_videos(id, page, size).await?;
    to_json(&page)
}
