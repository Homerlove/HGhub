//! 线路（数据源）：固定线路 + 地址发布页上的镜像线路。
//!
//! 上游只是域名在变，请求链路与数据模型完全一致，所以「切线路」等价于换 `net::base()`。
//! 镜像地址来自 <https://huangguoai.pages.dev/>：那个页面本身是空壳，真正的清单在
//! `publish.js` 里（`var urls` 域名数组 + `var subdomains` 子域数组，页面自己随机拼）。

use serde_json::{json, Value};
use std::time::{Duration, Instant};

/// 固定线路：写死在客户端里，不依赖任何外部页面。
pub const FIXED: [(&str, &str); 1] = [
    ("固定线路 1", "https://huangguoai.com"),
];

const PUBLISH_PAGE: &str = "https://huangguoai.pages.dev/";
/// 推荐镜像源：这个页面每次打开都会重挑线路，正好当「推荐镜像组」的来源。
const RECOMMEND_PAGE: &str = "https://huangguoai.ai/";
/// 推荐页里会出现、但不是站点本体的域名（地址发布页 / 下载站 / 图床 / 统计 / 跳转）。
const NOT_SITE: [&str; 10] = [
    "pages.dev",
    "huangguoai.ai",
    "huangguoju.com",
    "gvsnzgbim.cc",
    "hgai12.com",
    "drleao.cn",
    "googletagmanager.com",
    "cloudfront.net",
    "t.me",
    "x.com",
];
/// 镜像列表上限（子域 × 域名是笛卡尔积，界面只展示这么多）
const MIRROR_MAX: usize = 60;
/// 单条线路探测超时
const PROBE_TIMEOUT: Duration = Duration::from_secs(8);

pub fn fixed() -> Value {
    json!(FIXED
        .iter()
        .map(|(name, url)| json!({ "name": name, "url": url }))
        .collect::<Vec<_>>())
}

/// 拉地址发布页 → 解析 publish.js → 拼出镜像地址。只在用户点开「线路」时调用。
pub async fn mirrors() -> Result<Value, String> {
    let html = get_text(PUBLISH_PAGE).await?;
    let js_rel = script_src(&html).unwrap_or_else(|| "publish.js".to_string());
    let js_url = absolute(PUBLISH_PAGE, &js_rel);
    let js = get_text(&js_url).await?;

    let domains = js_array(&js, "var urls");
    let subs = js_array(&js, "var subdomains");
    if domains.is_empty() {
        return Err("地址发布页格式有变化：没找到镜像域名".to_string());
    }

    let mut items: Vec<Value> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    'outer: for domain in domains {
        let domain = domain.trim_end_matches('/').to_string();
        if domain.is_empty() {
            continue;
        }
        let hosts: Vec<String> = if subs.is_empty() {
            vec![domain.clone()]
        } else {
            subs.iter().map(|s| format!("{s}.{domain}")).collect()
        };
        for host in hosts {
            if !seen.insert(host.clone()) {
                continue;
            }
            items.push(json!({ "url": format!("https://{host}"), "host": host }));
            if items.len() >= MIRROR_MAX {
                break 'outer;
            }
        }
    }
    if items.is_empty() {
        return Err("地址发布页里没有可用的镜像地址".to_string());
    }

    let recommended = recommend().await;
    Ok(json!({
        "source": PUBLISH_PAGE,
        "script": js_url,
        "items": items,
        "recommended_source": RECOMMEND_PAGE,
        "recommended": recommended,
    }))
}

/// 推荐镜像组：抓推荐源页面，取页面上挂出的站点域名。
/// 只由 `mirrors()` 调用；条目结构与 `items` 完全一致，顺序保持页面自己的排列。
async fn recommend() -> Vec<Value> {
    let Ok(html) = get_text(RECOMMEND_PAGE).await else {
        return Vec::new();
    };

    let mut items: Vec<Value> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for host in href_hosts(&html) {
        if NOT_SITE.iter().any(|bad| host.ends_with(*bad)) {
            continue;
        }
        if !seen.insert(host.clone()) {
            continue;
        }
        items.push(json!({ "url": format!("https://{host}"), "host": host }));
        if items.len() >= MIRROR_MAX {
            break;
        }
    }
    items
}

/// 从 HTML 里取出 `href="https://<host>/…"` 的 host（去重交给调用方）。
fn href_hosts(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(i) = rest.find("href=\"https://") {
        let after = &rest[i + 14..];
        let end = after.find('"').unwrap_or(after.len());
        let host = after[..end]
            .split('/')
            .next()
            .unwrap_or_default()
            .split('?')
            .next()
            .unwrap_or_default();
        if !host.is_empty() {
            out.push(host.to_string());
        }
        rest = &after[end..];
    }
    out
}

/// 可达性探测（并发 6 条一组）。只报结果，不做自动切换。
pub async fn probe(urls: Vec<String>) -> Value {
    let mut out: Vec<Value> = Vec::new();
    for chunk in urls.chunks(6) {
        let mut handles = Vec::new();
        for u in chunk {
            let u = u.clone();
            handles.push(tokio::spawn(async move { probe_one(u).await }));
        }
        for h in handles {
            if let Ok(v) = h.await {
                out.push(v);
            }
        }
    }
    json!(out)
}

async fn probe_one(url: String) -> Value {
    let target = format!("{}/", url.trim_end_matches('/'));
    let started = Instant::now();
    let res = crate::net::get(
        &target,
        PROBE_TIMEOUT,
        Some("text/html,application/xhtml+xml"),
        None,
    )
    .await;
    let ms = started.elapsed().as_millis() as u64;
    match res {
        Ok(r) => {
            let status = r.status().as_u16();
            json!({ "url": url, "ok": status < 400, "status": status, "ms": ms })
        }
        Err(e) => json!({ "url": url, "ok": false, "ms": ms, "error": e }),
    }
}

/* ───────────────────────── 自动选线 ───────────────────────── */

/// 自动选线的两条候选：主站（海外能连）与国内专线。
/// 国内基本连不上 `.com`；在海外 `.ai` 会绕远，一定比 `.com` 慢。
/// 所以直接比这两条就够：不判断地区、不扫镜像列表、不看延迟阈值。
pub const AUTO_LINES: [(&str, &str); 2] = [
    ("main", crate::net::DEFAULT_BASE),
    ("cn", "https://huangguoai.ai"),
];

/// 自动探测比手动探测短：这个超时只在启动/换网时用，等太久不如直接用另一条。
const AUTO_PROBE_TIMEOUT: Duration = Duration::from_secs(4);

/// 一条候选线路的探测结果。
#[derive(Clone)]
pub struct Hit {
    pub tier: &'static str,
    pub url: String,
    pub ok: bool,
    pub reachable: bool,
    pub ms: u64,
    pub status: Option<u16>,
}

impl Hit {
    fn json(&self) -> Value {
        json!({
            "tier": self.tier,
            "url": self.url,
            "ok": self.ok,
            "reachable": self.reachable,
            "ms": self.ms,
            "status": self.status,
        })
    }
}

async fn probe_hit(tier: &'static str, url: &str) -> Hit {
    let target = format!("{}/", url.trim_end_matches('/'));
    let started = Instant::now();
    let res = crate::net::get(
        &target,
        AUTO_PROBE_TIMEOUT,
        Some("text/html,application/xhtml+xml"),
        None,
    )
    .await;
    let ms = started.elapsed().as_millis() as u64;
    match res {
        Ok(r) => {
            let status = r.status().as_u16();
            // 4xx 也说明域名后面是活的站点（鉴权、跳转都可能），只有 5xx / 网络错误才算不通
            Hit {
                tier,
                url: url.to_string(),
                ok: status < 400,
                reachable: status < 500,
                ms,
                status: Some(status),
            }
        }
        Err(_) => Hit {
            tier,
            url: url.to_string(),
            ok: false,
            reachable: false,
            ms,
            status: None,
        },
    }
}

/// 自动选线：`.com` 与 `.ai` 各探一次，通的、快的胜出。
///
/// 两条都不通时返回当前线路（`tier: "none"`），由调用方决定要不要提示用户。
pub async fn auto_pick() -> Value {
    let mut handles = Vec::new();
    for (tier, url) in AUTO_LINES {
        handles.push(tokio::spawn(async move { probe_hit(tier, url).await }));
    }
    let mut hits: Vec<Hit> = Vec::new();
    for h in handles {
        if let Ok(hit) = h.await {
            hits.push(hit);
        }
    }
    // 完全正常的优先，其次 4xx 也算活着，最后才比延迟
    hits.sort_by_key(|h| (!h.ok, !h.reachable, h.ms));
    let candidates: Vec<Value> = hits.iter().map(Hit::json).collect();

    match hits.first().filter(|h| h.reachable).cloned() {
        Some(h) => json!({
            "base": h.url,
            "tier": h.tier,
            "ms": h.ms,
            "reason": "fastest-reachable",
            "candidates": candidates,
        }),
        None => json!({
            "base": crate::net::base(),
            "tier": "none",
            "ms": 0,
            "reason": "all-unreachable",
            "candidates": candidates,
        }),
    }
}

async fn get_text(url: &str) -> Result<String, String> {
    let resp = crate::net::get(url, Duration::from_secs(12), None, None).await?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| crate::net::err_text(&e))
}

/// 从 HTML 里挑出脚本地址（发布页只有一个 `publish.js?<版本>`）。
fn script_src(html: &str) -> Option<String> {
    let mut rest = html;
    while let Some(i) = rest.find("src=\"") {
        let after = &rest[i + 5..];
        let end = after.find('"')?;
        let candidate = &after[..end];
        if candidate.contains(".js") {
            return Some(candidate.to_string());
        }
        rest = &after[end..];
    }
    None
}

fn absolute(base: &str, rel: &str) -> String {
    if rel.starts_with("http://") || rel.starts_with("https://") {
        return rel.to_string();
    }
    let origin = url::Url::parse(base)
        .ok()
        .map(|u| format!("{}://{}", u.scheme(), u.host_str().unwrap_or_default()));
    match origin {
        Some(o) if rel.starts_with('/') => format!("{o}{rel}"),
        Some(o) => format!("{o}/{rel}"),
        None => rel.to_string(),
    }
}

/// 取 JS 里 `var <name> = [ ... ]` 的字符串数组（去掉行尾 `//` 注释，但保留字符串里的 `//`）。
fn js_array(src: &str, decl: &str) -> Vec<String> {
    let Some(at) = src.find(decl) else {
        return Vec::new();
    };
    let rest = &src[at..];
    let Some(open) = rest.find('[') else {
        return Vec::new();
    };
    let Some(close) = rest[open..].find(']') else {
        return Vec::new();
    };
    strip_line_comments(&rest[open + 1..open + close])
        .split(',')
        .map(|raw| {
            raw.trim()
                .trim_matches(|c: char| c == '\'' || c == '"')
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// 去掉 `//` 行尾注释（引号内的 `//` 是 URL，不能动）。
fn strip_line_comments(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut quote: Option<char> = None;
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                out.push(c);
                if c == q {
                    quote = None;
                }
            }
            None => {
                if c == '\'' || c == '"' {
                    quote = Some(c);
                    out.push(c);
                } else if c == '/' && chars.peek() == Some(&'/') {
                    for n in chars.by_ref() {
                        if n == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                } else {
                    out.push(c);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
var subdomains = [
    'thu',  // 清华
    'pku',  // 北大
];
var urls=[
	'xxbgwypx.cc/',
];
var otherUrls = [
	'https://huangguoai.pages.dev',
];
var foreverUrls = [
	  'https://huangguoai.com',
	JumpPage,
	'https://huangguoai.ai',
];
"#;

    #[test]
    fn parses_publish_js_arrays() {
        assert_eq!(js_array(SAMPLE, "var urls"), vec!["xxbgwypx.cc/"]);
        assert_eq!(js_array(SAMPLE, "var subdomains"), vec!["thu", "pku"]);
        assert_eq!(
            js_array(SAMPLE, "var foreverUrls"),
            vec![
                "https://huangguoai.com",
                "JumpPage",
                "https://huangguoai.ai"
            ]
        );
    }

    #[test]
    fn resolves_relative_script() {
        assert_eq!(
            absolute("https://huangguoai.pages.dev/", "publish.js?20261103-2"),
            "https://huangguoai.pages.dev/publish.js?20261103-2"
        );
        assert_eq!(
            absolute("https://huangguoai.pages.dev/", "/a/b.js"),
            "https://huangguoai.pages.dev/a/b.js"
        );
    }

    #[test]
    fn picks_script_src() {
        let html = r#"<link rel="stylesheet" href="publish.css?1" /><script type="text/javascript" src="publish.js?20261103-2"></script>"#;
        assert_eq!(script_src(html).as_deref(), Some("publish.js?20261103-2"));
    }

    #[test]
    fn extracts_href_hosts() {
        let html = r#"<a href="https://gnpra.adihsppe.cc/ai-huanlian/">a</a><a href="https://huangguoai.com/terms/?x=1">b</a><a href="https://t.me/huangguofans">c</a>"#;
        assert_eq!(
            href_hosts(html),
            vec!["gnpra.adihsppe.cc", "huangguoai.com", "t.me"]
        );
    }

    /// 手动跑：`cargo test --lib -- --ignored lines::tests::mirrors_live -- --nocapture`
    #[tokio::test]
    #[ignore = "联网测试：请求上游镜像列表，手动用 --ignored 运行"]
    async fn mirrors_live() {
        match mirrors().await {
            Ok(v) => println!("{}", serde_json::to_string_pretty(&v).expect("json")),
            Err(e) => panic!("mirrors failed: {e}"),
        }
    }

    /// 手动跑：`cargo test --lib -- --ignored lines::tests::probe_live -- --nocapture`
    #[tokio::test]
    #[ignore = "联网测试：探测上游线路连通性，手动用 --ignored 运行"]
    async fn probe_live() {
        let urls = vec![
            "https://huangguoai.com".to_string(),
            "https://huangguoai.ai".to_string(),
            "https://thu.xxbgwypx.cc".to_string(),
        ];
        println!(
            "{}",
            serde_json::to_string_pretty(&probe(urls).await).expect("json")
        );
    }
}
