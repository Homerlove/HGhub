//! 领域契约：前端只通过 `DomainCommand` 请求数据，不认识上游接口。

use serde::Deserialize;

fn default_page() -> u32 {
    1
}

fn default_size() -> u32 {
    20
}

/// 前端 → 壳层的领域命令。字段名与 `src/domain/types.ts` 的 `Command` 对齐。
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum DomainCommand {
    Boot,
    Login {
        username: String,
        password: String,
    },
    Register {
        username: String,
        password: String,
        confirm_password: String,
        #[serde(default)]
        email: Option<String>,
    },
    Logout,
    Me,
    Feed {
        #[serde(default = "default_page")]
        page: u32,
        #[serde(default = "default_size")]
        size: u32,
    },
    Topics,
    Topic {
        slug: String,
    },
    Tags,
    TagVideos {
        tag: String,
        #[serde(default = "default_page")]
        page: u32,
        #[serde(default = "default_size")]
        size: u32,
    },
    Category {
        cate: String,
        #[serde(default = "default_page")]
        page: u32,
        #[serde(default = "default_size")]
        size: u32,
    },
    Rank {
        #[serde(default = "default_rank_kind")]
        kind: String,
    },
    Search {
        q: String,
        #[serde(default = "default_page")]
        page: u32,
        #[serde(default = "default_size")]
        size: u32,
        #[serde(default)]
        kind: String,
    },
    SearchHot,
    Posts {
        #[serde(default = "default_page")]
        page: u32,
        #[serde(default = "default_size")]
        size: u32,
    },
    Post {
        id: i64,
    },
    PostPlayback {
        id: i64,
    },
    PostLike {
        id: i64,
        on: bool,
    },
    PostCollect {
        id: i64,
        on: bool,
    },
    PostComments {
        id: i64,
        #[serde(default = "default_page")]
        page: u32,
        #[serde(default = "default_size")]
        size: u32,
    },
    PostCategories,
    PostCategory {
        route: String,
        #[serde(default = "default_page")]
        page: u32,
        #[serde(default = "default_size")]
        size: u32,
    },
    Video {
        id: i64,
    },
    Play {
        id: i64,
        ep: i64,
    },
    Comments {
        id: i64,
        #[serde(default = "default_episode")]
        ep: i64,
        #[serde(default = "default_page")]
        page: u32,
        #[serde(default = "default_size")]
        size: u32,
    },
    CommentSend {
        id: i64,
        #[serde(default = "default_episode")]
        ep: i64,
        text: String,
        #[serde(default)]
        reply_id: i64,
    },
    Like {
        id: i64,
        #[serde(default = "default_episode")]
        ep: i64,
        on: bool,
    },
    Collect {
        id: i64,
        on: bool,
    },
    Follow {
        uid: i64,
        on: bool,
    },
    Related {
        id: i64,
    },
    Following {
        #[serde(default = "default_page")]
        page: u32,
        #[serde(default = "default_size")]
        size: u32,
    },
    User {
        id: i64,
    },
    UserStats {
        id: i64,
    },
    UserVideos {
        id: i64,
        #[serde(default = "default_page")]
        page: u32,
        #[serde(default = "default_size")]
        size: u32,
    },
}

fn default_rank_kind() -> String {
    "hot".to_string()
}

fn default_episode() -> i64 {
    1
}
