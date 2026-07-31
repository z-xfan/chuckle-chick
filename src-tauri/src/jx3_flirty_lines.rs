use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::Mutex as AsyncMutex;

const RANDOM_ENDPOINT: &str = "https://www.jx3api.com/saohua/random";
const DEVOTED_ENDPOINT: &str = "https://www.jx3api.com/saohua/content";
const SOURCE_LABEL: &str = "JX3API（第三方）";
const REQUEST_COOLDOWN_MS: u64 = 1_000;
const MAX_TEXT_CHARS: usize = 1_000;

#[derive(Clone)]
pub struct Jx3FlirtyLineState {
    client: Client,
    random_lock: Arc<AsyncMutex<()>>,
    devoted_lock: Arc<AsyncMutex<()>>,
    last_request_at: Arc<Mutex<HashMap<FlirtyLineKind, u64>>>,
}

impl Default for Jx3FlirtyLineState {
    fn default() -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .user_agent("ChuckleChick/0.1 (JX3 flirty lines reader)")
            .build()
            .expect("JX3 flirty lines HTTP client must be constructible");
        Self {
            client,
            random_lock: Arc::new(AsyncMutex::new(())),
            devoted_lock: Arc::new(AsyncMutex::new(())),
            last_request_at: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Jx3FlirtyLineState {
    async fn fetch_line(&self, kind: FlirtyLineKind) -> Result<FlirtyLineView, String> {
        let request_lock = match kind {
            FlirtyLineKind::Random => &self.random_lock,
            FlirtyLineKind::Devoted => &self.devoted_lock,
        };
        let _request_guard = request_lock
            .try_lock()
            .map_err(|_| "正在寻找这类骚话，请稍候".to_string())?;

        let now_ms = unix_now_ms();
        let cooldown_remaining = {
            let last_requests = self
                .last_request_at
                .lock()
                .map_err(|error| error.to_string())?;
            request_cooldown_remaining(last_requests.get(&kind).copied(), now_ms)
        };
        if cooldown_remaining > 0 {
            return Err("操作太快啦，请稍后再换一句".to_string());
        }
        self.last_request_at
            .lock()
            .map_err(|error| error.to_string())?
            .insert(kind, now_ms);

        let response = self
            .client
            .get(kind.endpoint())
            .send()
            .await
            .map_err(network_error)?
            .error_for_status()
            .map_err(network_error)?
            .json::<FlirtyLineApiResponse>()
            .await
            .map_err(|error| format!("JX3API 骚话格式暂时无法识别：{error}"))?;

        normalize_response(kind, response, unix_now())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum FlirtyLineKind {
    Random,
    Devoted,
}

impl FlirtyLineKind {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "random" => Ok(Self::Random),
            "devoted" => Ok(Self::Devoted),
            _ => Err("不支持的骚话类型".to_string()),
        }
    }

    fn endpoint(self) -> &'static str {
        match self {
            Self::Random => RANDOM_ENDPOINT,
            Self::Devoted => DEVOTED_ENDPOINT,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlirtyLineView {
    kind: FlirtyLineKind,
    text: String,
    source: String,
    fetched_at: u64,
}

#[derive(Deserialize)]
struct FlirtyLineApiResponse {
    code: i32,
    #[serde(default)]
    msg: String,
    data: Option<RawFlirtyLine>,
    #[serde(default)]
    time: u64,
}

#[derive(Deserialize)]
struct RawFlirtyLine {
    #[serde(default)]
    text: String,
}

#[tauri::command]
pub async fn fetch_jx3_flirty_line(
    kind: String,
    state: State<'_, Jx3FlirtyLineState>,
) -> Result<FlirtyLineView, String> {
    state.fetch_line(FlirtyLineKind::parse(&kind)?).await
}

fn normalize_response(
    kind: FlirtyLineKind,
    response: FlirtyLineApiResponse,
    fallback_time: u64,
) -> Result<FlirtyLineView, String> {
    if response.code != 200 {
        let message = response.msg.trim();
        return Err(if message.is_empty() {
            "JX3API 骚话返回失败".to_string()
        } else {
            format!("JX3API 骚话返回失败：{message}")
        });
    }

    let text = response
        .data
        .ok_or_else(|| "JX3API 没有返回骚话内容".to_string())?
        .text;
    let text = text.trim();
    if text.is_empty() {
        return Err("JX3API 返回了空内容，请换一句再试".to_string());
    }
    if text.chars().count() > MAX_TEXT_CHARS {
        return Err("JX3API 返回的内容过长，已停止展示".to_string());
    }

    Ok(FlirtyLineView {
        kind,
        text: text.to_string(),
        source: SOURCE_LABEL.to_string(),
        fetched_at: if response.time == 0 {
            fallback_time
        } else {
            response.time
        },
    })
}

fn request_cooldown_remaining(last_request_at: Option<u64>, now_ms: u64) -> u64 {
    let Some(last_request_at) = last_request_at else {
        return 0;
    };
    REQUEST_COOLDOWN_MS.saturating_sub(now_ms.saturating_sub(last_request_at))
}

fn network_error(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "连接 JX3API 超时".to_string()
    } else if error.is_connect() {
        "暂时无法连接 JX3API".to_string()
    } else {
        format!("读取 JX3API 骚话失败：{error}")
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(code: i32, msg: &str, text: Option<&str>, time: u64) -> FlirtyLineApiResponse {
        FlirtyLineApiResponse {
            code,
            msg: msg.to_string(),
            data: text.map(|text| RawFlirtyLine {
                text: text.to_string(),
            }),
            time,
        }
    }

    #[test]
    fn maps_only_confirmed_kinds_to_exact_endpoints() {
        assert_eq!(
            FlirtyLineKind::parse("random").expect("random").endpoint(),
            RANDOM_ENDPOINT
        );
        assert_eq!(
            FlirtyLineKind::parse("devoted")
                .expect("devoted")
                .endpoint(),
            DEVOTED_ENDPOINT
        );
        assert!(FlirtyLineKind::parse("other").is_err());
    }

    #[test]
    fn normalizes_valid_text_and_uses_upstream_time() {
        let view = normalize_response(
            FlirtyLineKind::Random,
            response(200, "success", Some("  江湖有你真好。  "), 123),
            999,
        )
        .expect("valid response");

        assert_eq!(view.kind, FlirtyLineKind::Random);
        assert_eq!(view.text, "江湖有你真好。");
        assert_eq!(view.source, SOURCE_LABEL);
        assert_eq!(view.fetched_at, 123);
    }

    #[test]
    fn rejects_business_errors_missing_empty_and_oversized_text() {
        assert!(normalize_response(
            FlirtyLineKind::Random,
            response(500, "维护中", None, 0),
            999,
        )
        .expect_err("business error")
        .contains("维护中"));
        assert!(normalize_response(
            FlirtyLineKind::Random,
            response(200, "success", None, 0),
            999,
        )
        .is_err());
        assert!(normalize_response(
            FlirtyLineKind::Random,
            response(200, "success", Some("  "), 0),
            999,
        )
        .is_err());
        let oversized = "骚".repeat(MAX_TEXT_CHARS + 1);
        assert!(normalize_response(
            FlirtyLineKind::Devoted,
            response(200, "success", Some(&oversized), 0),
            999,
        )
        .is_err());
    }

    #[test]
    fn request_cooldown_is_one_second() {
        assert_eq!(request_cooldown_remaining(None, 2_000), 0);
        assert_eq!(request_cooldown_remaining(Some(2_000), 2_999), 1);
        assert_eq!(request_cooldown_remaining(Some(2_000), 3_000), 0);
    }
}
