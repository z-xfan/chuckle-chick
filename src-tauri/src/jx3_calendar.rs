use std::{
    collections::HashSet,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Datelike, Days, FixedOffset, NaiveDate, Timelike, Utc, Weekday};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::Mutex as AsyncMutex;

const CALENDAR_ENDPOINT: &str = "https://www.jx3api.com/active/calendar";
const FUTURE_DAYS: u64 = 7;
const FUTURE_CACHE_FRESHNESS: Duration = Duration::from_secs(6 * 60 * 60);
const MANUAL_REFRESH_COOLDOWN: Duration = Duration::from_secs(60);
const SOURCE_LABEL: &str = "JX3API（第三方）";
const CALENDAR_CACHE_SCHEMA_VERSION: u8 = 5;
const WEEKLY_PUBLIC_CATEGORY: &str = "武林通鉴·公共任务";
const WEEKLY_RAID_CATEGORY: &str = "武林通鉴·团队秘境";

#[derive(Clone)]
pub struct Jx3CalendarState {
    client: Client,
    cache_path: Arc<PathBuf>,
    cache: Arc<Mutex<Option<CalendarCache>>>,
    refresh_lock: Arc<AsyncMutex<()>>,
    last_request_at: Arc<Mutex<Option<u64>>>,
}

impl Jx3CalendarState {
    pub fn load(cache_path: PathBuf) -> Self {
        let cache = fs::read_to_string(&cache_path)
            .ok()
            .and_then(|contents| serde_json::from_str::<CalendarCache>(&contents).ok())
            .filter(|cache| cache.schema_version == CALENDAR_CACHE_SCHEMA_VERSION);
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .user_agent("ChuckleChick/0.1 (JX3 calendar reader)")
            .build()
            .expect("JX3 calendar HTTP client must be constructible");
        Self {
            client,
            cache_path: Arc::new(cache_path),
            cache: Arc::new(Mutex::new(cache)),
            refresh_lock: Arc::new(AsyncMutex::new(())),
            last_request_at: Arc::new(Mutex::new(None)),
        }
    }

    fn cached_view(&self) -> Option<CalendarView> {
        let now = unix_now();
        let game_date = current_game_date();
        let cache = self.cache.lock().ok()?.as_ref()?.clone();
        build_calendar_view(&cache, game_date, now)
    }

    async fn fetch_view(&self, force: bool) -> Result<CalendarView, String> {
        let _refresh_guard = self.refresh_lock.lock().await;
        if !force {
            if let Some(cached) = self.cached_view() {
                if !cached.stale {
                    return Ok(cached);
                }
            }
        }

        let now = unix_now();
        let remaining = {
            let last_request = self
                .last_request_at
                .lock()
                .map_err(|error| error.to_string())?;
            refresh_cooldown_remaining(*last_request, now)
        };
        if remaining > 0 {
            if !force {
                if let Some(cached) = self.cached_view() {
                    return Ok(cached);
                }
            }
            return Err(format!("刷新过于频繁，请 {remaining} 秒后再试"));
        }
        *self
            .last_request_at
            .lock()
            .map_err(|error| error.to_string())? = Some(now);

        let mut data = self.fetch_range().await?;
        let today_detail = self.fetch_today_detail().await?;
        merge_today_detail(&mut data, today_detail);
        let cache = normalize_calendar_response(data, current_game_date(), unix_now())?;
        self.save_cache(cache.clone())?;
        build_calendar_view(&cache, current_game_date(), unix_now())
            .ok_or_else(|| "JX3API 日历没有返回今天或未来数据".to_string())
    }

    async fn fetch_range(&self) -> Result<CalendarApiData, String> {
        let response = self
            .client
            .get(CALENDAR_ENDPOINT)
            .query(&[("mode", "list"), ("num", "7")])
            .send()
            .await
            .map_err(network_error)?
            .error_for_status()
            .map_err(network_error)?
            .json::<CalendarApiResponse>()
            .await
            .map_err(|error| format!("JX3API 日历格式暂时无法识别：{error}"))?;
        if response.code != 200 {
            return Err(format!("JX3API 日历返回失败：{}", response.msg));
        }
        response
            .data
            .ok_or_else(|| "JX3API 日历没有返回数据".to_string())
    }

    async fn fetch_today_detail(&self) -> Result<RawCalendarDay, String> {
        let response = self
            .client
            .get(CALENDAR_ENDPOINT)
            .query(&[("mode", "day"), ("num", "0")])
            .send()
            .await
            .map_err(network_error)?
            .error_for_status()
            .map_err(network_error)?
            .json::<CalendarDayApiResponse>()
            .await
            .map_err(|error| format!("JX3API 今日日历格式暂时无法识别：{error}"))?;
        if response.code != 200 {
            return Err(format!("JX3API 今日日历返回失败：{}", response.msg));
        }
        response
            .data
            .ok_or_else(|| "JX3API 今日日历没有返回数据".to_string())
    }

    fn save_cache(&self, cache: CalendarCache) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&cache).map_err(|error| error.to_string())?;
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(self.cache_path.as_ref(), json).map_err(|error| error.to_string())?;
        *self.cache.lock().map_err(|error| error.to_string())? = Some(cache);
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarView {
    fetched_at: u64,
    source: String,
    stale: bool,
    incomplete: bool,
    days: Vec<CalendarDay>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDay {
    date: String,
    weekday: String,
    predicted: bool,
    weekly_pending: bool,
    items: Vec<CalendarItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarItem {
    id: String,
    category: String,
    name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CalendarCache {
    #[serde(default)]
    schema_version: u8,
    fetched_at: u64,
    game_date: String,
    days: Vec<CachedCalendarDay>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CachedCalendarDay {
    date: String,
    weekday: String,
    items: Vec<CalendarItem>,
}

#[derive(Deserialize)]
struct CalendarApiResponse {
    code: i32,
    #[serde(default)]
    msg: String,
    data: Option<CalendarApiData>,
}

#[derive(Deserialize)]
struct CalendarDayApiResponse {
    code: i32,
    #[serde(default)]
    msg: String,
    data: Option<RawCalendarDay>,
}

#[derive(Deserialize)]
struct CalendarApiData {
    #[serde(default)]
    total: Vec<RawCalendarDay>,
}

#[derive(Deserialize)]
struct RawCalendarDay {
    date: String,
    #[serde(default)]
    week: String,
    war: Option<String>,
    battle: Option<String>,
    orecar: Option<String>,
    school: Option<String>,
    rescue: Option<String>,
    #[serde(default)]
    draw: Vec<String>,
    #[serde(default)]
    lucky: Vec<String>,
    #[serde(default)]
    card: Vec<String>,
    #[serde(default)]
    leader: Vec<String>,
    #[serde(default)]
    weekly: RawWeeklyActivities,
}

#[derive(Default, Deserialize)]
struct RawWeeklyActivities {
    #[serde(default)]
    conn: Vec<String>,
    #[serde(default)]
    raid: Vec<String>,
}

fn merge_today_detail(data: &mut CalendarApiData, today_detail: RawCalendarDay) {
    if let Some(index) = data
        .total
        .iter()
        .position(|day| day.date == today_detail.date)
    {
        data.total[index] = today_detail;
    } else {
        data.total.push(today_detail);
    }
}

#[tauri::command]
pub fn get_cached_jx3_calendar(
    state: State<'_, Jx3CalendarState>,
) -> Result<Option<CalendarView>, String> {
    Ok(state.cached_view())
}

#[tauri::command]
pub async fn fetch_jx3_calendar(
    force: bool,
    state: State<'_, Jx3CalendarState>,
) -> Result<CalendarView, String> {
    state.fetch_view(force).await
}

fn normalize_calendar_response(
    data: CalendarApiData,
    game_date: NaiveDate,
    fetched_at: u64,
) -> Result<CalendarCache, String> {
    let mut days = Vec::new();
    let mut seen_dates = HashSet::new();
    for raw in data.total {
        let Ok(date) = NaiveDate::parse_from_str(raw.date.trim(), "%Y-%m-%d") else {
            continue;
        };
        if !seen_dates.insert(date) {
            continue;
        }
        let mut items = Vec::new();
        let mut seen_items = HashSet::new();
        push_item(&mut items, &mut seen_items, date, "war", "大战", raw.war);
        push_item(
            &mut items,
            &mut seen_items,
            date,
            "battle",
            "战场",
            raw.battle,
        );
        push_item(
            &mut items,
            &mut seen_items,
            date,
            "orecar",
            "阵营矿车",
            raw.orecar,
        );
        push_item(
            &mut items,
            &mut seen_items,
            date,
            "school",
            "门派事件",
            raw.school,
        );
        push_item(
            &mut items,
            &mut seen_items,
            date,
            "rescue",
            "驰援",
            raw.rescue,
        );
        push_items(
            &mut items,
            &mut seen_items,
            date,
            "draw",
            "美人图",
            raw.draw,
        );
        push_items(
            &mut items,
            &mut seen_items,
            date,
            "lucky",
            "宠物福缘",
            raw.lucky,
        );
        push_items(
            &mut items,
            &mut seen_items,
            date,
            "card",
            "家园声望·加倍道具",
            raw.card,
        );
        push_items(
            &mut items,
            &mut seen_items,
            date,
            "leader",
            "世界boss",
            raw.leader,
        );
        push_items(
            &mut items,
            &mut seen_items,
            date,
            "weekly-conn",
            WEEKLY_PUBLIC_CATEGORY,
            raw.weekly.conn,
        );
        push_items(
            &mut items,
            &mut seen_items,
            date,
            "weekly-raid",
            WEEKLY_RAID_CATEGORY,
            raw.weekly.raid,
        );
        days.push(CachedCalendarDay {
            date: date.format("%Y-%m-%d").to_string(),
            weekday: normalized_weekday(&raw.week, date),
            items,
        });
    }
    days.sort_by(|left, right| left.date.cmp(&right.date));
    propagate_current_week_items(&mut days, game_date);
    if days.is_empty() {
        return Err("JX3API 日历数据为空或日期格式已经变化".to_string());
    }
    Ok(CalendarCache {
        schema_version: CALENDAR_CACHE_SCHEMA_VERSION,
        fetched_at,
        game_date: game_date.format("%Y-%m-%d").to_string(),
        days,
    })
}

fn propagate_current_week_items(days: &mut [CachedCalendarDay], game_date: NaiveDate) {
    let game_date_text = game_date.format("%Y-%m-%d").to_string();
    let weekly_items = days
        .iter()
        .find(|day| day.date == game_date_text)
        .map(|day| {
            day.items
                .iter()
                .filter(|item| is_weekly_category(&item.category))
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if weekly_items.is_empty() {
        return;
    }

    let current_week = game_date.iso_week();
    for day in days {
        let Ok(date) = NaiveDate::parse_from_str(&day.date, "%Y-%m-%d") else {
            continue;
        };
        if date <= game_date || date.iso_week() != current_week {
            continue;
        }
        let mut existing = day
            .items
            .iter()
            .map(|item| (item.category.clone(), item.name.clone()))
            .collect::<HashSet<_>>();
        for (index, item) in weekly_items.iter().enumerate() {
            if !existing.insert((item.category.clone(), item.name.clone())) {
                continue;
            }
            day.items.push(CalendarItem {
                id: format!("{}:weekly-current:{index}", day.date),
                category: item.category.clone(),
                name: item.name.clone(),
            });
        }
    }
}

fn is_weekly_category(category: &str) -> bool {
    matches!(category, WEEKLY_PUBLIC_CATEGORY | WEEKLY_RAID_CATEGORY)
}

fn push_item(
    items: &mut Vec<CalendarItem>,
    seen: &mut HashSet<(String, String)>,
    date: NaiveDate,
    key: &str,
    category: &str,
    value: Option<String>,
) {
    let Some(name) = value.map(|value| value.trim().to_string()) else {
        return;
    };
    if name.is_empty() || !seen.insert((category.to_string(), name.clone())) {
        return;
    }
    items.push(CalendarItem {
        id: format!("{}:{key}", date.format("%Y-%m-%d")),
        category: category.to_string(),
        name,
    });
}

fn push_items(
    items: &mut Vec<CalendarItem>,
    seen: &mut HashSet<(String, String)>,
    date: NaiveDate,
    key: &str,
    category: &str,
    values: Vec<String>,
) {
    for (index, value) in values.into_iter().enumerate() {
        let Some(name) = Some(value.trim().to_string()).filter(|name| !name.is_empty()) else {
            continue;
        };
        if !seen.insert((category.to_string(), name.clone())) {
            continue;
        }
        items.push(CalendarItem {
            id: format!("{}:{key}:{index}", date.format("%Y-%m-%d")),
            category: category.to_string(),
            name,
        });
    }
}

fn build_calendar_view(
    cache: &CalendarCache,
    game_date: NaiveDate,
    now: u64,
) -> Option<CalendarView> {
    let last_date = game_date.checked_add_days(Days::new(FUTURE_DAYS))?;
    let mut days = cache
        .days
        .iter()
        .filter_map(|day| {
            let date = NaiveDate::parse_from_str(&day.date, "%Y-%m-%d").ok()?;
            if date < game_date || date > last_date {
                return None;
            }
            Some(CalendarDay {
                date: day.date.clone(),
                weekday: day.weekday.clone(),
                predicted: date > game_date,
                weekly_pending: date.iso_week() != game_date.iso_week(),
                items: day.items.clone(),
            })
        })
        .collect::<Vec<_>>();
    days.sort_by(|left, right| left.date.cmp(&right.date));
    if days.is_empty() {
        return None;
    }
    let game_date_text = game_date.format("%Y-%m-%d").to_string();
    let complete = days.len() == (FUTURE_DAYS + 1) as usize
        && days.first().is_some_and(|day| day.date == game_date_text)
        && days
            .last()
            .is_some_and(|day| day.date == last_date.format("%Y-%m-%d").to_string());
    Some(CalendarView {
        fetched_at: cache.fetched_at,
        source: SOURCE_LABEL.to_string(),
        stale: cache.game_date != game_date_text
            || now.saturating_sub(cache.fetched_at) >= FUTURE_CACHE_FRESHNESS.as_secs(),
        incomplete: !complete,
        days,
    })
}

fn current_game_date() -> NaiveDate {
    let offset = FixedOffset::east_opt(8 * 60 * 60).expect("Shanghai offset must be valid");
    game_date_at(Utc::now().with_timezone(&offset))
}

fn game_date_at(now: DateTime<FixedOffset>) -> NaiveDate {
    let date = now.date_naive();
    if now.hour() < 7 {
        date.pred_opt().unwrap_or(date)
    } else {
        date
    }
}

fn normalized_weekday(value: &str, date: NaiveDate) -> String {
    let value = value.trim().trim_start_matches("星期");
    if matches!(value, "一" | "二" | "三" | "四" | "五" | "六" | "日") {
        return value.to_string();
    }
    match date.weekday() {
        Weekday::Mon => "一",
        Weekday::Tue => "二",
        Weekday::Wed => "三",
        Weekday::Thu => "四",
        Weekday::Fri => "五",
        Weekday::Sat => "六",
        Weekday::Sun => "日",
    }
    .to_string()
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
        format!("读取 JX3API 日历失败：{error}")
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
    use chrono::TimeZone;

    fn sample_data() -> CalendarApiData {
        serde_json::from_str::<CalendarApiResponse>(
            r#"{
              "code": 200,
              "msg": "success",
              "data": {
                "today": {"date": "2026-07-31", "week": "五"},
                "total": [
                  {
                    "date": "2026-07-31",
                    "week": "五",
                    "war": "大战！英雄风砂旧垒",
                    "battle": "神农洇",
                    "orecar": "跨服·河西瀚漠",
                    "school": "七秀·红袖飞花",
                    "rescue": "七秀·乱世",
                    "card": ["英雄仙踪林", "英雄毒神殿", "英雄仙踪林"],
                    "draw": ["衍天才女·成女", "药宗青黛·成女"],
                    "leader": ["岑伤", "阿基修斯"],
                    "lucky": ["絮絮", "瓜瓜"],
                    "weekly": {
                      "conn": ["经首道源·越海珠贝", "侠客岛·雾岛寻丹"],
                      "raid": ["阆风悬城", "敖龙岛", "范阳夜变"]
                    }
                  },
                  {
                    "date": "2026-08-01",
                    "week": "六",
                    "war": "大战！英雄不染窟",
                    "battle": "三国古战场",
                    "orecar": "跨服·河西瀚漠",
                    "school": "七秀·剑气凝光",
                    "rescue": "藏剑·乱世"
                  }
                ]
              }
            }"#,
        )
        .expect("sample should parse")
        .data
        .expect("sample should contain data")
    }

    fn sample_today_detail() -> RawCalendarDay {
        serde_json::from_str::<CalendarDayApiResponse>(
            r#"{
              "code": 200,
              "msg": "success",
              "data": {
                "date": "2026-07-31",
                "week": "五",
                "war": "大战！英雄风砂旧垒",
                "battle": "神农洇",
                "orecar": "跨服·河西瀚漠",
                "school": "七秀·红袖飞花",
                "rescue": "七秀·乱世",
                "weekly": {
                  "conn": ["经首道源·越海珠贝", "侠客岛·雾岛寻丹"],
                  "raid": ["阆风悬城", "敖龙岛", "范阳夜变"]
                }
              }
            }"#,
        )
        .expect("today detail should parse")
        .data
        .expect("today detail should contain data")
    }

    #[test]
    fn game_day_changes_at_seven_in_shanghai() {
        let offset = FixedOffset::east_opt(8 * 60 * 60).expect("offset");
        let before = offset
            .with_ymd_and_hms(2026, 7, 31, 6, 59, 59)
            .single()
            .expect("valid time");
        let boundary = offset
            .with_ymd_and_hms(2026, 7, 31, 7, 0, 0)
            .single()
            .expect("valid time");

        assert_eq!(
            game_date_at(before),
            NaiveDate::from_ymd_opt(2026, 7, 30).expect("valid date")
        );
        assert_eq!(
            game_date_at(boundary),
            NaiveDate::from_ymd_opt(2026, 7, 31).expect("valid date")
        );
    }

    #[test]
    fn normalizes_all_confirmed_calendar_fields_and_deduplicates_lists() {
        let game_date = NaiveDate::from_ymd_opt(2026, 7, 31).expect("valid date");
        let cache =
            normalize_calendar_response(sample_data(), game_date, 100).expect("normalization");

        assert_eq!(cache.days.len(), 2);
        assert_eq!(cache.schema_version, CALENDAR_CACHE_SCHEMA_VERSION);
        assert_eq!(cache.days[0].items.len(), 18);
        assert_eq!(cache.days[0].items[0].category, "大战");
        assert_eq!(
            cache.days[0]
                .items
                .iter()
                .filter(|item| item.category == "家园声望·加倍道具")
                .count(),
            2
        );
        assert!(cache.days[0]
            .items
            .iter()
            .any(|item| item.category == "美人图" && item.name == "衍天才女·成女"));
        assert!(cache.days[0]
            .items
            .iter()
            .any(|item| item.category == "世界boss" && item.name == "阿基修斯"));
        assert!(cache.days[0].items.iter().any(|item| {
            item.category == "武林通鉴·团队秘境" && item.name == "范阳夜变"
        }));
        assert_eq!(
            cache.days[1]
                .items
                .iter()
                .filter(|item| is_weekly_category(&item.category))
                .count(),
            5
        );
    }

    #[test]
    fn merges_today_detail_into_the_range_response() {
        let mut data = sample_data();
        data.total[0].weekly = RawWeeklyActivities::default();

        merge_today_detail(&mut data, sample_today_detail());

        assert_eq!(data.total.len(), 2);
        assert_eq!(data.total[0].weekly.conn.len(), 2);
        assert_eq!(data.total[0].weekly.raid.len(), 3);
    }

    #[test]
    fn view_marks_future_days_as_predictions() {
        let game_date = NaiveDate::from_ymd_opt(2026, 7, 31).expect("valid date");
        let cache =
            normalize_calendar_response(sample_data(), game_date, 100).expect("normalization");
        let view = build_calendar_view(&cache, game_date, 101).expect("view");

        assert!(!view.days[0].predicted);
        assert!(view.days[1].predicted);
        assert!(!view.days[0].weekly_pending);
        assert!(!view.days[1].weekly_pending);
        assert!(view.incomplete);
        assert_eq!(view.source, SOURCE_LABEL);
    }

    #[test]
    fn view_filters_to_today_and_the_next_seven_days() {
        let game_date = NaiveDate::from_ymd_opt(2026, 7, 31).expect("valid date");
        let days = (-1_i64..=8)
            .map(|offset| {
                let date = game_date
                    .checked_add_signed(chrono::Duration::days(offset))
                    .expect("date in range");
                CachedCalendarDay {
                    date: date.format("%Y-%m-%d").to_string(),
                    weekday: normalized_weekday("", date),
                    items: Vec::new(),
                }
            })
            .collect();
        let cache = CalendarCache {
            schema_version: CALENDAR_CACHE_SCHEMA_VERSION,
            fetched_at: 100,
            game_date: game_date.format("%Y-%m-%d").to_string(),
            days,
        };

        let view = build_calendar_view(&cache, game_date, 101).expect("view");
        assert_eq!(view.days.len(), 8);
        assert_eq!(view.days[0].date, "2026-07-31");
        assert_eq!(view.days[7].date, "2026-08-07");
        assert!(!view.days[2].weekly_pending);
        assert!(view.days[3].weekly_pending);
        assert!(!view.incomplete);
    }

    #[test]
    fn cache_becomes_stale_after_six_hours_or_game_day_change() {
        let game_date = NaiveDate::from_ymd_opt(2026, 7, 31).expect("valid date");
        let cache =
            normalize_calendar_response(sample_data(), game_date, 100).expect("normalization");

        assert!(
            !build_calendar_view(&cache, game_date, 101)
                .expect("view")
                .stale
        );
        assert!(
            build_calendar_view(&cache, game_date, 100 + FUTURE_CACHE_FRESHNESS.as_secs())
                .expect("view")
                .stale
        );
        assert!(
            build_calendar_view(&cache, game_date.succ_opt().expect("next day"), 101)
                .expect("view")
                .stale
        );
    }

    #[test]
    fn refresh_cooldown_is_sixty_seconds() {
        assert_eq!(refresh_cooldown_remaining(None, 100), 0);
        assert_eq!(refresh_cooldown_remaining(Some(100), 100), 60);
        assert_eq!(refresh_cooldown_remaining(Some(100), 159), 1);
        assert_eq!(refresh_cooldown_remaining(Some(100), 160), 0);
    }
}
