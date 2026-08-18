use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tauri::State;
use tokio::sync::Mutex as AsyncMutex;

const ANSWER_ENDPOINT: &str = "https://www.jx3api.com/saohua/answer";
const EAT_ENDPOINT: &str = "https://www.jx3api.com/saohua/eat";
const DRINK_ENDPOINT: &str = "https://www.jx3api.com/saohua/drink";
const SOURCE_LABEL: &str = "JX3API（第三方）";
const REQUEST_COOLDOWN_MS: u64 = 1_000;
const MAX_ITEM_CHARS: usize = 1_000;
const MAX_ITEMS: usize = 10;

#[derive(Clone)]
pub struct Jx3DecisionState {
    client: Client,
    answer_lock: Arc<AsyncMutex<()>>,
    eat_lock: Arc<AsyncMutex<()>>,
    drink_lock: Arc<AsyncMutex<()>>,
    last_request_at: Arc<Mutex<HashMap<DecisionKind, u64>>>,
}

impl Default for Jx3DecisionState {
    fn default() -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .user_agent("ChuckleChick/0.1 (JX3 decision helper)")
            .build()
            .expect("JX3 decision HTTP client must be constructible");
        Self {
            client,
            answer_lock: Arc::new(AsyncMutex::new(())),
            eat_lock: Arc::new(AsyncMutex::new(())),
            drink_lock: Arc::new(AsyncMutex::new(())),
            last_request_at: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Jx3DecisionState {
    async fn fetch(&self, kind: DecisionKind) -> Result<DecisionView, String> {
        let request_lock = match kind {
            DecisionKind::Answer => &self.answer_lock,
            DecisionKind::Eat => &self.eat_lock,
            DecisionKind::Drink => &self.drink_lock,
        };
        let _request_guard = request_lock
            .try_lock()
            .map_err(|_| "正在寻找这个答案，请稍候".to_string())?;

        let now_ms = unix_now_ms();
        let cooldown_remaining = {
            let last_requests = self
                .last_request_at
                .lock()
                .map_err(|error| error.to_string())?;
            request_cooldown_remaining(last_requests.get(&kind).copied(), now_ms)
        };
        if cooldown_remaining > 0 {
            return Err("操作太快啦，请稍后再试".to_string());
        }
        self.last_request_at
            .lock()
            .map_err(|error| error.to_string())?
            .insert(kind, now_ms);

        match kind {
            DecisionKind::Answer => {
                let response = self.fetch_response::<RawAnswer>(kind).await?;
                normalize_answer(response, unix_now())
            }
            DecisionKind::Eat | DecisionKind::Drink => {
                let response = self.fetch_response::<Vec<String>>(kind).await?;
                normalize_choices(kind, response, unix_now())
            }
        }
    }

    async fn fetch_response<T: DeserializeOwned>(
        &self,
        kind: DecisionKind,
    ) -> Result<ApiResponse<T>, String> {
        self.client
            .get(kind.endpoint())
            .send()
            .await
            .map_err(network_error)?
            .error_for_status()
            .map_err(network_error)?
            .json::<ApiResponse<T>>()
            .await
            .map_err(|error| format!("JX3API 小决定格式暂时无法识别：{error}"))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum DecisionKind {
    Answer,
    Eat,
    Drink,
}

impl DecisionKind {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "answer" => Ok(Self::Answer),
            "eat" => Ok(Self::Eat),
            "drink" => Ok(Self::Drink),
            _ => Err("不支持的小决定类型".to_string()),
        }
    }

    fn endpoint(self) -> &'static str {
        match self {
            Self::Answer => ANSWER_ENDPOINT,
            Self::Eat => EAT_ENDPOINT,
            Self::Drink => DRINK_ENDPOINT,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionView {
    kind: DecisionKind,
    items: Vec<String>,
    source: String,
    fetched_at: u64,
}

#[derive(Deserialize)]
struct ApiResponse<T> {
    code: i32,
    #[serde(default)]
    msg: String,
    data: Option<T>,
    #[serde(default)]
    time: u64,
}

#[derive(Deserialize)]
struct RawAnswer {
    #[serde(default)]
    answer: String,
    #[serde(default)]
    hearten: String,
}

#[tauri::command]
pub async fn fetch_jx3_decision(
    kind: String,
    state: State<'_, Jx3DecisionState>,
) -> Result<DecisionView, String> {
    state.fetch(DecisionKind::parse(&kind)?).await
}

fn normalize_answer(
    response: ApiResponse<RawAnswer>,
    fallback_time: u64,
) -> Result<DecisionView, String> {
    ensure_success(response.code, &response.msg)?;
    let answer = response
        .data
        .ok_or_else(|| "JX3API 没有返回答案".to_string())?;
    let items = normalize_items([answer.answer, answer.hearten])?;
    if items.len() != 2 {
        return Err("JX3API 返回的答案不完整，请再试一次".to_string());
    }
    Ok(view(
        DecisionKind::Answer,
        items,
        response.time,
        fallback_time,
    ))
}

fn normalize_choices(
    kind: DecisionKind,
    response: ApiResponse<Vec<String>>,
    fallback_time: u64,
) -> Result<DecisionView, String> {
    ensure_success(response.code, &response.msg)?;
    let choices = response
        .data
        .ok_or_else(|| "JX3API 没有返回候选项".to_string())?;
    if choices.len() > MAX_ITEMS {
        return Err("JX3API 返回的候选项过多，已停止展示".to_string());
    }
    let items = normalize_items(choices)?;
    Ok(view(kind, items, response.time, fallback_time))
}

fn normalize_items(items: impl IntoIterator<Item = String>) -> Result<Vec<String>, String> {
    let items = items
        .into_iter()
        .map(|item| item.trim().to_string())
        .collect::<Vec<_>>();
    if items.is_empty() || items.iter().any(|item| item.is_empty()) {
        return Err("JX3API 返回了空内容，请再试一次".to_string());
    }
    if items
        .iter()
        .any(|item| item.chars().count() > MAX_ITEM_CHARS)
    {
        return Err("JX3API 返回的内容过长，已停止展示".to_string());
    }
    Ok(items)
}

fn ensure_success(code: i32, message: &str) -> Result<(), String> {
    if code == 200 {
        return Ok(());
    }
    let message = message.trim();
    Err(if message.is_empty() {
        "JX3API 小决定返回失败".to_string()
    } else {
        format!("JX3API 小决定返回失败：{message}")
    })
}

fn view(kind: DecisionKind, items: Vec<String>, time: u64, fallback_time: u64) -> DecisionView {
    DecisionView {
        kind,
        items,
        source: SOURCE_LABEL.to_string(),
        fetched_at: if time == 0 { fallback_time } else { time },
    }
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
        format!("读取 JX3API 小决定失败：{error}")
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

    fn response<T>(code: i32, msg: &str, data: Option<T>, time: u64) -> ApiResponse<T> {
        ApiResponse {
            code,
            msg: msg.to_string(),
            data,
            time,
        }
    }

    #[test]
    fn maps_only_free_confirmed_kinds_to_exact_endpoints() {
        assert_eq!(
            DecisionKind::parse("answer").unwrap().endpoint(),
            ANSWER_ENDPOINT
        );
        assert_eq!(DecisionKind::parse("eat").unwrap().endpoint(), EAT_ENDPOINT);
        assert_eq!(
            DecisionKind::parse("drink").unwrap().endpoint(),
            DRINK_ENDPOINT
        );
        assert!(DecisionKind::parse("context").is_err());
        assert!(DecisionKind::parse("other").is_err());
    }

    #[test]
    fn normalizes_answer_and_preserves_both_fields() {
        let result = normalize_answer(
            response(
                200,
                "success",
                Some(RawAnswer {
                    answer: "  可以  ".to_string(),
                    hearten: "  相信自己  ".to_string(),
                }),
                123,
            ),
            999,
        )
        .unwrap();
        assert_eq!(result.items, ["可以", "相信自己"]);
        assert_eq!(result.fetched_at, 123);
    }

    #[test]
    fn rejects_incomplete_answer() {
        let result = normalize_answer(
            response(
                200,
                "success",
                Some(RawAnswer {
                    answer: "可以".to_string(),
                    hearten: " ".to_string(),
                }),
                0,
            ),
            999,
        );
        assert!(result.is_err());
    }

    #[test]
    fn normalizes_choices_and_preserves_order() {
        let result = normalize_choices(
            DecisionKind::Eat,
            response(
                200,
                "success",
                Some(vec!["  小面 ".to_string(), "火锅".to_string()]),
                0,
            ),
            999,
        )
        .unwrap();
        assert_eq!(result.items, ["小面", "火锅"]);
        assert_eq!(result.fetched_at, 999);
    }

    #[test]
    fn rejects_business_errors_empty_and_oversized_choices() {
        assert!(normalize_choices(
            DecisionKind::Drink,
            response::<Vec<String>>(401, "禁止访问", None, 0),
            999,
        )
        .unwrap_err()
        .contains("禁止访问"));
        assert!(normalize_choices(
            DecisionKind::Drink,
            response(200, "success", Some(vec![]), 0),
            999,
        )
        .is_err());
        assert!(normalize_choices(
            DecisionKind::Drink,
            response(200, "success", Some(vec![" ".to_string()]), 0),
            999,
        )
        .is_err());
        assert!(normalize_choices(
            DecisionKind::Drink,
            response(
                200,
                "success",
                Some(vec!["喝".repeat(MAX_ITEM_CHARS + 1)]),
                0
            ),
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
