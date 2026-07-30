use std::{
    collections::{HashMap, HashSet},
    fs,
    io::ErrorKind,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use encoding_rs::GBK;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;
use tokio::{net::TcpStream, sync::Mutex as AsyncMutex, task::JoinSet, time};

use crate::{assistant, preferences::PersistentPreferences};

const ANNOUNCEMENT_ENDPOINT: &str = "https://jx3.xoyo.com/api.php";
const OFFICIAL_SERVER_LIST_URL: &str =
    "https://jx3comm.xoyocdn.com/jx3hd/zhcn_hd/serverlist/serverlist.ini";
const NEWS_CACHE_FRESHNESS: Duration = Duration::from_secs(10 * 60);
const SERVER_LIST_FRESHNESS: Duration = Duration::from_secs(15 * 60);
const CLOSED_POLL_INTERVAL: Duration = Duration::from_secs(5 * 60);
const OPEN_CONFIRM_INTERVAL: Duration = Duration::from_secs(30);
const OPEN_POLL_INTERVAL: Duration = Duration::from_secs(15 * 60);
const MONITOR_TICK: Duration = Duration::from_secs(10);
const TCP_PROBE_TIMEOUT: Duration = Duration::from_secs(3);
const ANNOUNCEMENT_PAGE_SIZE: u32 = 10;
const OFFICIAL_SKILL_SCAN_PAGE_SIZE: u32 = 30;
const MAX_SKILL_SCAN_PAGES: u32 = 80;

#[derive(Clone)]
pub struct Jx3State {
    client: Client,
    cache_path: Arc<PathBuf>,
    news_cache: Arc<Mutex<NewsCacheStore>>,
    news_refresh_lock: Arc<AsyncMutex<()>>,
    server_cache: Arc<Mutex<Option<ServerListCache>>>,
}

impl Jx3State {
    pub fn load(cache_path: PathBuf) -> Self {
        let news_cache = fs::read_to_string(&cache_path)
            .ok()
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_default();
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .user_agent("ChuckleChick/0.1 (JX3 official information reader)")
            .build()
            .expect("JX3 HTTP client must be constructible");
        Self {
            client,
            cache_path: Arc::new(cache_path),
            news_cache: Arc::new(Mutex::new(news_cache)),
            news_refresh_lock: Arc::new(AsyncMutex::new(())),
            server_cache: Arc::new(Mutex::new(None)),
        }
    }

    fn cached_news_page(&self, kind: NewsKind, page: u32) -> Option<NewsPage> {
        let key = cache_key(kind, page);
        let mut result = self.news_cache.lock().ok()?.pages.get(&key)?.clone();
        result.stale =
            unix_now().saturating_sub(result.fetched_at) >= NEWS_CACHE_FRESHNESS.as_secs();
        Some(result)
    }

    fn save_news_page(&self, page: NewsPage) -> Result<(), String> {
        let json = {
            let mut cache = self.news_cache.lock().map_err(|error| error.to_string())?;
            cache.pages.insert(cache_key(page.kind, page.page), page);
            serde_json::to_string_pretty(&*cache).map_err(|error| error.to_string())?
        };
        self.write_news_cache(json)
    }

    fn cached_official_page(&self, page: u32) -> Option<OfficialArticlePage> {
        let cache = self.news_cache.lock().ok()?;
        let cached = cache.official_pages.get(&page.to_string())?;
        if unix_now().saturating_sub(cached.fetched_at) >= NEWS_CACHE_FRESHNESS.as_secs() {
            return None;
        }
        Some(cached.page.clone())
    }

    fn save_official_page(&self, page: u32, value: OfficialArticlePage) -> Result<(), String> {
        let json = {
            let mut cache = self.news_cache.lock().map_err(|error| error.to_string())?;
            cache.official_pages.insert(
                page.to_string(),
                CachedOfficialArticlePage {
                    fetched_at: unix_now(),
                    page: value,
                },
            );
            serde_json::to_string_pretty(&*cache).map_err(|error| error.to_string())?
        };
        self.write_news_cache(json)
    }

    fn write_news_cache(&self, json: String) -> Result<(), String> {
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(self.cache_path.as_ref(), json).map_err(|error| error.to_string())
    }

    async fn fetch_news_page(
        &self,
        kind: NewsKind,
        page: u32,
        force: bool,
    ) -> Result<NewsPage, String> {
        let _refresh_guard = self.news_refresh_lock.lock().await;
        match kind {
            NewsKind::Announcement => self.fetch_announcement_page(page, force).await,
            NewsKind::SkillChange => self.fetch_skill_change_page(page, force).await,
        }
    }

    async fn fetch_announcement_page(&self, page: u32, force: bool) -> Result<NewsPage, String> {
        let (source_page, offset) = announcement_source_position(page);
        let response = self.fetch_official_articles(source_page, force).await?;
        let total_pages = response.count.div_ceil(ANNOUNCEMENT_PAGE_SIZE).max(1);
        let items = response
            .list
            .into_iter()
            .skip(offset)
            .take(ANNOUNCEMENT_PAGE_SIZE as usize)
            .map(map_article)
            .collect();
        let result = NewsPage {
            kind: NewsKind::Announcement,
            page,
            page_size: ANNOUNCEMENT_PAGE_SIZE,
            total_items: Some(response.count),
            total_pages: Some(total_pages),
            has_more: page < total_pages,
            fetched_at: unix_now(),
            stale: false,
            items,
        };
        self.save_news_page(result.clone())?;
        Ok(result)
    }

    async fn fetch_skill_change_page(&self, page: u32, force: bool) -> Result<NewsPage, String> {
        let target_start = ((page - 1) * ANNOUNCEMENT_PAGE_SIZE) as usize;
        let target_end = (page * ANNOUNCEMENT_PAGE_SIZE) as usize;
        let target_with_lookahead = target_end + 1;
        let scan_limit = (page.saturating_mul(20)).clamp(20, MAX_SKILL_SCAN_PAGES);
        let mut matches = Vec::new();
        let mut scanned_items = 0_u32;
        let mut total_articles = u32::MAX;

        for source_page in 1..=scan_limit {
            let response = self.fetch_official_articles(source_page, force).await?;
            total_articles = response.count;
            let returned = response.list.len() as u32;
            scanned_items = scanned_items.saturating_add(returned);
            matches.extend(
                response
                    .list
                    .into_iter()
                    .filter(|item| is_skill_change(&item.title))
                    .map(map_article),
            );
            if matches.len() >= target_with_lookahead
                || returned == 0
                || scanned_items >= total_articles
            {
                break;
            }
        }

        let items = if target_start < matches.len() {
            matches[target_start..matches.len().min(target_end)].to_vec()
        } else {
            Vec::new()
        };
        let has_more = matches.len() > target_end || scanned_items < total_articles;
        let result = NewsPage {
            kind: NewsKind::SkillChange,
            page,
            page_size: ANNOUNCEMENT_PAGE_SIZE,
            total_items: None,
            total_pages: None,
            has_more,
            fetched_at: unix_now(),
            stale: false,
            items,
        };
        self.save_news_page(result.clone())?;
        Ok(result)
    }

    async fn fetch_official_articles(
        &self,
        page: u32,
        force: bool,
    ) -> Result<OfficialArticlePage, String> {
        if !force {
            if let Some(cached) = self.cached_official_page(page) {
                return Ok(cached);
            }
        }
        let response = self
            .client
            .get(ANNOUNCEMENT_ENDPOINT)
            .query(&[
                ("op", "search_api".to_string()),
                ("order", "auto".to_string()),
                ("action", "get_customer_article_list".to_string()),
                ("game", "jx3".to_string()),
                ("num", OFFICIAL_SKILL_SCAN_PAGE_SIZE.to_string()),
                ("page", page.to_string()),
            ])
            .send()
            .await
            .map_err(network_error)?
            .error_for_status()
            .map_err(network_error)?
            .json::<OfficialNewsResponse>()
            .await
            .map_err(|error| format!("西山居公告格式暂时无法识别：{error}"))?;
        if response.code != 1 {
            return Err(format!("西山居公告返回失败：{}", response.msg));
        }
        self.save_official_page(page, response.data.clone())?;
        Ok(response.data)
    }

    async fn server_records(&self, force: bool) -> Result<Vec<ServerRecord>, String> {
        if !force {
            if let Ok(cache) = self.server_cache.lock() {
                if let Some(cache) = cache.as_ref() {
                    if unix_now().saturating_sub(cache.fetched_at) < SERVER_LIST_FRESHNESS.as_secs()
                    {
                        return Ok(cache.records.clone());
                    }
                }
            }
        }

        let bytes = self
            .client
            .get(OFFICIAL_SERVER_LIST_URL)
            .send()
            .await
            .map_err(network_error)?
            .error_for_status()
            .map_err(network_error)?
            .bytes()
            .await
            .map_err(network_error)?;
        let (decoded, _, had_errors) = GBK.decode(&bytes);
        if had_errors {
            return Err("西山居服务器列表编码异常".to_string());
        }
        let records = parse_server_list(&decoded)?;
        let cache = ServerListCache {
            fetched_at: unix_now(),
            records: records.clone(),
        };
        *self
            .server_cache
            .lock()
            .map_err(|error| error.to_string())? = Some(cache);
        Ok(records)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NewsKind {
    Announcement,
    SkillChange,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsItem {
    id: String,
    title: String,
    published_at: String,
    source_url: String,
    is_skill_change: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsPage {
    kind: NewsKind,
    page: u32,
    page_size: u32,
    total_items: Option<u32>,
    total_pages: Option<u32>,
    has_more: bool,
    fetched_at: u64,
    stale: bool,
    items: Vec<NewsItem>,
}

#[derive(Default, Deserialize, Serialize)]
struct NewsCacheStore {
    #[serde(default)]
    pages: HashMap<String, NewsPage>,
    #[serde(default)]
    official_pages: HashMap<String, CachedOfficialArticlePage>,
}

#[derive(Clone, Deserialize, Serialize)]
struct CachedOfficialArticlePage {
    fetched_at: u64,
    page: OfficialArticlePage,
}

#[derive(Clone, Deserialize, Serialize)]
struct OfficialNewsResponse {
    code: i32,
    #[serde(default)]
    msg: String,
    data: OfficialArticlePage,
}

#[derive(Clone, Deserialize, Serialize)]
struct OfficialArticlePage {
    #[serde(default)]
    list: Vec<OfficialArticle>,
    #[serde(default)]
    count: u32,
}

#[derive(Clone, Deserialize, Serialize)]
struct OfficialArticle {
    id: u64,
    title: String,
    asktime: String,
}

#[derive(Clone, Debug)]
struct ServerRecord {
    name: String,
    zone: String,
    address: SocketAddr,
}

#[derive(Clone, Debug)]
struct ServerListCache {
    fetched_at: u64,
    records: Vec<ServerRecord>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerOption {
    name: String,
    zone: String,
    monitored: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ServerAvailability {
    Unknown,
    Closed,
    Open,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    name: String,
    zone: String,
    status: ServerAvailability,
    checked_at: u64,
}

#[tauri::command]
pub fn get_cached_jx3_news_page(
    kind: NewsKind,
    page: u32,
    state: State<'_, Jx3State>,
) -> Result<Option<NewsPage>, String> {
    validate_page(page)?;
    Ok(state.cached_news_page(kind, page))
}

#[tauri::command]
pub async fn fetch_jx3_news_page(
    kind: NewsKind,
    page: u32,
    force: bool,
    state: State<'_, Jx3State>,
) -> Result<NewsPage, String> {
    validate_page(page)?;
    state.fetch_news_page(kind, page, force).await
}

#[tauri::command]
pub async fn get_jx3_server_options(
    preferences: State<'_, PersistentPreferences>,
    state: State<'_, Jx3State>,
) -> Result<Vec<ServerOption>, String> {
    let monitored: HashSet<String> = preferences
        .snapshot()
        .monitored_servers
        .into_iter()
        .collect();
    let records = state.server_records(false).await?;
    Ok(records
        .into_iter()
        .map(|record| ServerOption {
            monitored: monitored.contains(&record.name),
            name: record.name,
            zone: record.zone,
        })
        .collect())
}

#[tauri::command]
pub async fn check_jx3_server(
    server_name: String,
    state: State<'_, Jx3State>,
) -> Result<ServerStatus, String> {
    let server_name = normalized_server_name(&server_name)?;
    let record = state
        .server_records(false)
        .await?
        .into_iter()
        .find(|record| record.name == server_name)
        .ok_or_else(|| "没有找到该服务器".to_string())?;
    Ok(status_from_probe(&record, probe_server(&record).await))
}

#[tauri::command]
pub async fn set_jx3_server_monitoring(
    server_name: String,
    enabled: bool,
    preferences: State<'_, PersistentPreferences>,
    state: State<'_, Jx3State>,
) -> Result<Vec<String>, String> {
    let server_name = normalized_server_name(&server_name)?;
    let exists = state
        .server_records(false)
        .await?
        .iter()
        .any(|record| record.name == server_name);
    if !exists {
        return Err("没有找到该服务器".to_string());
    }
    preferences.update(|current| {
        current
            .monitored_servers
            .retain(|existing| existing != &server_name);
        if enabled {
            current.monitored_servers.push(server_name.clone());
            current.monitored_servers.sort();
            current.monitored_servers.dedup();
        }
    })?;
    Ok(preferences.snapshot().monitored_servers)
}

#[tauri::command]
pub fn stop_all_jx3_server_monitoring(
    preferences: State<'_, PersistentPreferences>,
) -> Result<(), String> {
    preferences.update(|current| current.monitored_servers.clear())
}

#[tauri::command]
pub fn open_jx3_official_url(url: String, app: AppHandle) -> Result<(), String> {
    let parsed = validated_official_url(&url)?;
    app.opener()
        .open_url(parsed.as_str(), None::<&str>)
        .map_err(|error| error.to_string())
}

pub async fn run_monitor_loop(app: AppHandle, state: Jx3State) {
    let mut runtimes: HashMap<String, MonitorRuntime> = HashMap::new();
    loop {
        let monitored: HashSet<String> = app
            .state::<PersistentPreferences>()
            .snapshot()
            .monitored_servers
            .into_iter()
            .collect();
        runtimes.retain(|name, _| monitored.contains(name));
        let now = Instant::now();
        for name in monitored {
            runtimes
                .entry(name)
                .or_insert_with(|| MonitorRuntime::new(now));
        }

        let due: Vec<String> = runtimes
            .iter()
            .filter(|(_, runtime)| runtime.next_check <= now)
            .map(|(name, _)| name.clone())
            .collect();
        if !due.is_empty() {
            match state.server_records(false).await {
                Ok(records) => {
                    let by_name: HashMap<String, ServerRecord> = records
                        .into_iter()
                        .map(|record| (record.name.clone(), record))
                        .collect();
                    let mut probes = JoinSet::new();
                    for name in &due {
                        if let Some(record) = by_name.get(name).cloned() {
                            probes.spawn(async move {
                                let status = probe_server(&record).await;
                                (record.name, status)
                            });
                        } else if let Some(runtime) = runtimes.get_mut(name) {
                            runtime.record(ServerAvailability::Unknown, Instant::now());
                        }
                    }

                    while let Some(result) = probes.join_next().await {
                        let Ok((name, status)) = result else {
                            continue;
                        };
                        let should_report = runtimes
                            .get_mut(&name)
                            .map(|runtime| runtime.record(status, Instant::now()))
                            .unwrap_or(false);
                        if should_report {
                            let _ = assistant::show_speech_bubble_for_app(
                                &app,
                                format!("{name}已经开服啦，快去闯荡江湖吧！"),
                                6_000,
                            );
                        }
                    }
                }
                Err(_) => {
                    for name in due {
                        if let Some(runtime) = runtimes.get_mut(&name) {
                            runtime.record(ServerAvailability::Unknown, Instant::now());
                        }
                    }
                }
            }
        }
        time::sleep(MONITOR_TICK).await;
    }
}

#[derive(Clone, Debug)]
struct MonitorRuntime {
    confirmed: Option<ServerAvailability>,
    candidate_open: bool,
    next_check: Instant,
}

impl MonitorRuntime {
    fn new(now: Instant) -> Self {
        Self {
            confirmed: None,
            candidate_open: false,
            next_check: now,
        }
    }

    fn record(&mut self, observed: ServerAvailability, now: Instant) -> bool {
        let (report, delay) =
            apply_probe_result(&mut self.confirmed, &mut self.candidate_open, observed);
        self.next_check = now + delay;
        report
    }
}

fn apply_probe_result(
    confirmed: &mut Option<ServerAvailability>,
    candidate_open: &mut bool,
    observed: ServerAvailability,
) -> (bool, Duration) {
    if observed == ServerAvailability::Unknown {
        *candidate_open = false;
        return (
            false,
            if *confirmed == Some(ServerAvailability::Open) {
                OPEN_POLL_INTERVAL
            } else {
                CLOSED_POLL_INTERVAL
            },
        );
    }

    let Some(current) = *confirmed else {
        *confirmed = Some(observed);
        *candidate_open = false;
        return (
            false,
            if observed == ServerAvailability::Open {
                OPEN_POLL_INTERVAL
            } else {
                CLOSED_POLL_INTERVAL
            },
        );
    };

    match (current, observed, *candidate_open) {
        (ServerAvailability::Closed, ServerAvailability::Open, false) => {
            *candidate_open = true;
            (false, OPEN_CONFIRM_INTERVAL)
        }
        (ServerAvailability::Closed, ServerAvailability::Open, true) => {
            *candidate_open = false;
            *confirmed = Some(ServerAvailability::Open);
            (true, OPEN_POLL_INTERVAL)
        }
        (ServerAvailability::Open, ServerAvailability::Closed, _) => {
            *candidate_open = false;
            *confirmed = Some(ServerAvailability::Closed);
            (false, CLOSED_POLL_INTERVAL)
        }
        (ServerAvailability::Closed, ServerAvailability::Closed, _) => {
            *candidate_open = false;
            (false, CLOSED_POLL_INTERVAL)
        }
        (ServerAvailability::Open, ServerAvailability::Open, _) => {
            *candidate_open = false;
            (false, OPEN_POLL_INTERVAL)
        }
        _ => (false, CLOSED_POLL_INTERVAL),
    }
}

async fn probe_server(record: &ServerRecord) -> ServerAvailability {
    match time::timeout(TCP_PROBE_TIMEOUT, TcpStream::connect(record.address)).await {
        Ok(Ok(stream)) => {
            drop(stream);
            ServerAvailability::Open
        }
        Ok(Err(error)) if error.kind() == ErrorKind::ConnectionRefused => {
            ServerAvailability::Closed
        }
        Ok(Err(_)) | Err(_) => ServerAvailability::Unknown,
    }
}

fn status_from_probe(record: &ServerRecord, status: ServerAvailability) -> ServerStatus {
    ServerStatus {
        name: record.name.clone(),
        zone: record.zone.clone(),
        status,
        checked_at: unix_now(),
    }
}

fn parse_server_list(contents: &str) -> Result<Vec<ServerRecord>, String> {
    let mut records = HashMap::new();
    for line in contents.lines() {
        let fields: Vec<&str> = line.trim().split('\t').collect();
        if fields.len() < 12 || fields[1] != fields[10] {
            continue;
        }
        let Ok(ip) = fields[3].parse::<IpAddr>() else {
            continue;
        };
        let Ok(port) = fields[4].parse::<u16>() else {
            continue;
        };
        let name = fields[1].trim();
        let zone = fields[11].trim();
        if name.is_empty() || zone.is_empty() {
            continue;
        }
        records.entry(name.to_string()).or_insert(ServerRecord {
            name: name.to_string(),
            zone: zone.to_string(),
            address: SocketAddr::new(ip, port),
        });
    }
    let mut records: Vec<_> = records.into_values().collect();
    records.sort_by(|left, right| {
        left.zone
            .cmp(&right.zone)
            .then_with(|| left.name.cmp(&right.name))
    });
    if records.is_empty() {
        return Err("西山居服务器列表为空或格式已经变化".to_string());
    }
    Ok(records)
}

fn map_article(article: OfficialArticle) -> NewsItem {
    let title = article.title.trim().to_string();
    NewsItem {
        id: article.id.to_string(),
        published_at: article.asktime,
        source_url: format!("https://jx3.xoyo.com/announce/gg.html?id={}", article.id),
        is_skill_change: is_skill_change(&title),
        title,
    }
}

fn is_skill_change(title: &str) -> bool {
    title.contains("武学调整")
}

fn cache_key(kind: NewsKind, page: u32) -> String {
    let prefix = match kind {
        NewsKind::Announcement => "announcement",
        NewsKind::SkillChange => "skill-change",
    };
    format!("{prefix}:{page}")
}

fn announcement_source_position(page: u32) -> (u32, usize) {
    let first_item = (page - 1) * ANNOUNCEMENT_PAGE_SIZE;
    (
        first_item / OFFICIAL_SKILL_SCAN_PAGE_SIZE + 1,
        (first_item % OFFICIAL_SKILL_SCAN_PAGE_SIZE) as usize,
    )
}

fn validated_official_url(url: &str) -> Result<Url, String> {
    let parsed = Url::parse(url).map_err(|_| "公告链接无效".to_string())?;
    if parsed.scheme() != "https" || parsed.host_str() != Some("jx3.xoyo.com") {
        return Err("只允许打开剑网 3 西山居官方链接".to_string());
    }
    Ok(parsed)
}

fn validate_page(page: u32) -> Result<(), String> {
    if (1..=500).contains(&page) {
        Ok(())
    } else {
        Err("页码必须在 1 到 500 之间".to_string())
    }
}

fn normalized_server_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 20 {
        return Err("服务器名称无效".to_string());
    }
    Ok(name.to_string())
}

fn network_error(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "连接西山居数据源超时".to_string()
    } else if error.is_connect() {
        "暂时无法连接西山居数据源".to_string()
    } else {
        format!("读取西山居数据失败：{error}")
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_main_servers() {
        let input = concat!(
            "电信区\t梦江南\t81\t109.244.61.59\t3724\t电信区\t梦江南\t0\t0\tz05\t梦江南\t电信区\t\t\t\t1\n",
            "电信区\t旧服别名\t81\t109.244.61.59\t3724\t电信区\t旧服别名\t0\t0\tz05\t梦江南\t电信区\t\t\t\t0\n"
        );
        let records = parse_server_list(input).expect("server list should parse");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].name, "梦江南");
        assert_eq!(records[0].zone, "电信区");
    }

    #[test]
    fn skill_change_filter_is_deterministic() {
        assert!(is_skill_change("7月13日资料片武学调整"));
        assert!(!is_skill_change("7月13日例行维护公告"));
    }

    #[test]
    fn announcement_pages_share_thirty_item_source_pages() {
        assert_eq!(announcement_source_position(1), (1, 0));
        assert_eq!(announcement_source_position(2), (1, 10));
        assert_eq!(announcement_source_position(3), (1, 20));
        assert_eq!(announcement_source_position(4), (2, 0));
    }

    #[test]
    fn official_url_allowlist_rejects_spoofed_hosts_and_plain_http() {
        assert!(validated_official_url("https://jx3.xoyo.com/announce/gg.html?id=123").is_ok());
        assert!(validated_official_url("https://jx3.xoyo.com.example.com/announce").is_err());
        assert!(validated_official_url("http://jx3.xoyo.com/announce").is_err());
    }

    #[test]
    fn monitor_requires_two_open_samples_after_closed() {
        let mut confirmed = Some(ServerAvailability::Closed);
        let mut candidate = false;
        let first = apply_probe_result(&mut confirmed, &mut candidate, ServerAvailability::Open);
        assert_eq!(first, (false, OPEN_CONFIRM_INTERVAL));
        assert!(candidate);
        assert_eq!(confirmed, Some(ServerAvailability::Closed));

        let second = apply_probe_result(&mut confirmed, &mut candidate, ServerAvailability::Open);
        assert_eq!(second, (true, OPEN_POLL_INTERVAL));
        assert!(!candidate);
        assert_eq!(confirmed, Some(ServerAvailability::Open));
    }

    #[test]
    fn initial_open_establishes_baseline_without_report() {
        let mut confirmed = None;
        let mut candidate = false;
        let result = apply_probe_result(&mut confirmed, &mut candidate, ServerAvailability::Open);
        assert_eq!(result, (false, OPEN_POLL_INTERVAL));
        assert_eq!(confirmed, Some(ServerAvailability::Open));
    }

    #[test]
    fn unknown_cancels_candidate_without_overwriting_confirmed_state() {
        let mut confirmed = Some(ServerAvailability::Closed);
        let mut candidate = true;
        let result =
            apply_probe_result(&mut confirmed, &mut candidate, ServerAvailability::Unknown);
        assert_eq!(result, (false, CLOSED_POLL_INTERVAL));
        assert!(!candidate);
        assert_eq!(confirmed, Some(ServerAvailability::Closed));
    }
}
