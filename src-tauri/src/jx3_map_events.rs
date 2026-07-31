use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::Mutex as AsyncMutex;

const MAP_EVENTS_ENDPOINT: &str = "https://www.jx3api.com/active/celebs";
const CACHE_FRESHNESS: Duration = Duration::from_secs(60);
const MANUAL_REFRESH_COOLDOWN: Duration = Duration::from_secs(60);
const SOURCE_LABEL: &str = "JX3API（第三方）";
const MAX_EVENTS: usize = 10;

#[derive(Clone)]
pub struct Jx3MapEventState {
    client: Client,
    cache_path: Arc<PathBuf>,
    cache: Arc<Mutex<MapEventCacheStore>>,
    refresh_lock: Arc<AsyncMutex<()>>,
    last_request_at: Arc<Mutex<HashMap<MapEventCategory, u64>>>,
}

impl Jx3MapEventState {
    pub fn load(cache_path: PathBuf) -> Self {
        let cache = fs::read_to_string(&cache_path)
            .ok()
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_default();
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .user_agent("ChuckleChick/0.1 (JX3 map events reader)")
            .build()
            .expect("JX3 map events HTTP client must be constructible");
        Self {
            client,
            cache_path: Arc::new(cache_path),
            cache: Arc::new(Mutex::new(cache)),
            refresh_lock: Arc::new(AsyncMutex::new(())),
            last_request_at: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn cached_view(&self, category: MapEventCategory) -> Option<MapEventView> {
        let mut view = self
            .cache
            .lock()
            .ok()?
            .pages
            .get(category.cache_key())?
            .clone();
        view.stale = cache_is_stale(view.fetched_at, unix_now());
        Some(view)
    }

    async fn fetch_view(
        &self,
        category: MapEventCategory,
        force: bool,
    ) -> Result<MapEventView, String> {
        let _refresh_guard = self.refresh_lock.lock().await;
        if !force {
            if let Some(cached) = self.cached_view(category) {
                if !cached.stale {
                    return Ok(cached);
                }
            }
        }

        let now = unix_now();
        let remaining = {
            let last_requests = self
                .last_request_at
                .lock()
                .map_err(|error| error.to_string())?;
            refresh_cooldown_remaining(last_requests.get(&category).copied(), now)
        };
        if remaining > 0 {
            if !force {
                if let Some(cached) = self.cached_view(category) {
                    return Ok(cached);
                }
            }
            return Err(format!("刷新过于频繁，请 {remaining} 秒后再试"));
        }
        self.last_request_at
            .lock()
            .map_err(|error| error.to_string())?
            .insert(category, now);

        let response = self
            .client
            .get(MAP_EVENTS_ENDPOINT)
            .query(&[("name", category.label())])
            .send()
            .await
            .map_err(network_error)?
            .error_for_status()
            .map_err(network_error)?
            .json::<MapEventApiResponse>()
            .await
            .map_err(|error| format!("JX3API 地图事件格式暂时无法识别：{error}"))?;
        if response.code != 200 {
            return Err(format!("JX3API 地图事件返回失败：{}", response.msg));
        }
        let view = normalize_map_events(category, response.data, response.time.max(now));
        self.save_view(view.clone())?;
        Ok(view)
    }

    fn save_view(&self, view: MapEventView) -> Result<(), String> {
        let json = {
            let mut cache = self.cache.lock().map_err(|error| error.to_string())?;
            cache
                .pages
                .insert(view.category.cache_key().to_string(), view);
            serde_json::to_string_pretty(&*cache).map_err(|error| error.to_string())?
        };
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(self.cache_path.as_ref(), json).map_err(|error| error.to_string())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
enum MapEventCategory {
    #[serde(rename = "楚天社")]
    Chutian,
    #[serde(rename = "云从社")]
    Yuncong,
    #[serde(rename = "披风会")]
    Pifeng,
}

impl MapEventCategory {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "楚天社" => Ok(Self::Chutian),
            "云从社" => Ok(Self::Yuncong),
            "披风会" => Ok(Self::Pifeng),
            _ => Err("不支持的地图事件分类".to_string()),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Chutian => "楚天社",
            Self::Yuncong => "云从社",
            Self::Pifeng => "披风会",
        }
    }

    fn cache_key(self) -> &'static str {
        match self {
            Self::Chutian => "chutian",
            Self::Yuncong => "yuncong",
            Self::Pifeng => "pifeng",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapEventView {
    category: MapEventCategory,
    source: String,
    fetched_at: u64,
    stale: bool,
    items: Vec<MapEventItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapEventItem {
    id: String,
    time: String,
    map: String,
    site: String,
    stage: String,
    description: String,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct MapEventCacheStore {
    pages: HashMap<String, MapEventView>,
}

#[derive(Deserialize)]
struct MapEventApiResponse {
    code: i32,
    #[serde(default)]
    msg: String,
    #[serde(default)]
    data: Vec<RawMapEvent>,
    #[serde(default)]
    time: u64,
}

#[derive(Deserialize)]
struct RawMapEvent {
    #[serde(default)]
    time: String,
    #[serde(default)]
    map: String,
    #[serde(default)]
    site: String,
    #[serde(default)]
    stage: String,
    #[serde(default)]
    desc: String,
}

#[tauri::command]
pub fn get_cached_jx3_map_events(
    category: String,
    state: State<'_, Jx3MapEventState>,
) -> Result<Option<MapEventView>, String> {
    Ok(state.cached_view(MapEventCategory::parse(&category)?))
}

#[tauri::command]
pub async fn fetch_jx3_map_events(
    category: String,
    force: bool,
    state: State<'_, Jx3MapEventState>,
) -> Result<MapEventView, String> {
    state
        .fetch_view(MapEventCategory::parse(&category)?, force)
        .await
}

fn normalize_map_events(
    category: MapEventCategory,
    records: Vec<RawMapEvent>,
    fetched_at: u64,
) -> MapEventView {
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    for raw in records {
        let time = normalized_text(raw.time, "--:--");
        let map = normalized_text(raw.map, "地图待确认");
        let site = normalized_text(raw.site, "地点待确认");
        let stage = normalized_text(raw.stage, "未命名事件");
        let description = normalized_text(raw.desc, "暂无说明");
        let key = (time.clone(), map.clone(), site.clone(), stage.clone());
        if !seen.insert(key) {
            continue;
        }
        items.push(MapEventItem {
            id: format!("{}:{}", category.cache_key(), items.len()),
            time,
            map,
            site,
            stage,
            description,
        });
        if items.len() == MAX_EVENTS {
            break;
        }
    }
    MapEventView {
        category,
        source: SOURCE_LABEL.to_string(),
        fetched_at,
        stale: false,
        items,
    }
}

fn normalized_text(value: String, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn cache_is_stale(fetched_at: u64, now: u64) -> bool {
    now.saturating_sub(fetched_at) >= CACHE_FRESHNESS.as_secs()
}

fn refresh_cooldown_remaining(last_request_at: Option<u64>, now: u64) -> u64 {
    let Some(last_request_at) = last_request_at else {
        return 0;
    };
    MANUAL_REFRESH_COOLDOWN
        .as_secs()
        .saturating_sub(now.saturating_sub(last_request_at))
}

fn network_error(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "连接 JX3API 超时".to_string()
    } else if error.is_connect() {
        "暂时无法连接 JX3API".to_string()
    } else {
        format!("读取 JX3API 地图事件失败：{error}")
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

    fn raw_event(time: &str, map: &str, site: &str, stage: &str, desc: &str) -> RawMapEvent {
        RawMapEvent {
            time: time.to_string(),
            map: map.to_string(),
            site: site.to_string(),
            stage: stage.to_string(),
            desc: desc.to_string(),
        }
    }

    #[test]
    fn accepts_only_the_three_confirmed_categories() {
        assert_eq!(
            MapEventCategory::parse("楚天社").expect("category"),
            MapEventCategory::Chutian
        );
        assert_eq!(
            MapEventCategory::parse("云从社").expect("category"),
            MapEventCategory::Yuncong
        );
        assert_eq!(
            MapEventCategory::parse("披风会").expect("category"),
            MapEventCategory::Pifeng
        );
        assert!(MapEventCategory::parse("任意输入").is_err());
    }

    #[test]
    fn normalizes_fields_deduplicates_and_limits_results() {
        let mut records = vec![
            raw_event(
                " 12:14 ",
                " 烂柯山 ",
                " 蚩首山鬼市 ",
                " 凤凰集·醉酒闹事 ",
                " 公共任务 ",
            ),
            raw_event(
                "12:14",
                "烂柯山",
                "蚩首山鬼市",
                "凤凰集·醉酒闹事",
                "重复说明不会产生第二条",
            ),
            raw_event("", "", "", "", ""),
        ];
        for index in 0..12 {
            records.push(raw_event(
                &format!("13:{index:02}"),
                "晟江",
                "黄槲镇",
                &format!("事件 {index}"),
                "说明",
            ));
        }

        let view = normalize_map_events(MapEventCategory::Chutian, records, 100);

        assert_eq!(view.items.len(), MAX_EVENTS);
        assert_eq!(view.items[0].time, "12:14");
        assert_eq!(view.items[0].map, "烂柯山");
        assert_eq!(view.items[1].time, "--:--");
        assert_eq!(view.items[1].stage, "未命名事件");
        assert_eq!(view.items[1].description, "暂无说明");
    }

    #[test]
    fn cache_and_manual_refresh_use_sixty_seconds() {
        assert!(!cache_is_stale(100, 159));
        assert!(cache_is_stale(100, 160));
        assert_eq!(refresh_cooldown_remaining(None, 100), 0);
        assert_eq!(refresh_cooldown_remaining(Some(100), 159), 1);
        assert_eq!(refresh_cooldown_remaining(Some(100), 160), 0);
    }
}
