use std::sync::Mutex;

use serde::Serialize;
use tauri::{
    Emitter, LogicalSize, Manager, PhysicalPosition, Position, Size, State, WebviewWindow,
};

use crate::preferences::PersistentPreferences;

const PANEL_WIDTH: f64 = 320.0;
const PANEL_HEIGHT: f64 = 300.0;
const BUBBLE_WIDTH: f64 = 260.0;
const BUBBLE_HEIGHT: f64 = 112.0;
const WINDOW_GAP: f64 = 12.0;
const WORK_AREA_MARGIN: f64 = 12.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum AssistantMode {
    #[default]
    Hidden,
    Bubble,
    Panel,
}

pub struct AssistantWindowState {
    mode: Mutex<AssistantMode>,
    payload: Mutex<Option<AssistantPayload>>,
}

impl Default for AssistantWindowState {
    fn default() -> Self {
        Self {
            mode: Mutex::new(AssistantMode::Hidden),
            payload: Mutex::new(None),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BubblePlacement {
    Above,
    Below,
}

#[derive(Clone, Serialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum AssistantPayload {
    Panel,
    Bubble {
        message: String,
        #[serde(rename = "durationMs")]
        duration_ms: u64,
        placement: BubblePlacement,
    },
}

#[tauri::command]
pub fn get_assistant_payload(
    assistant_state: State<'_, AssistantWindowState>,
) -> Result<Option<AssistantPayload>, String> {
    assistant_state
        .payload
        .lock()
        .map(|payload| payload.clone())
        .map_err(|error| error.to_string())
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PetInteractionPayload {
    animation_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PhysicalBounds {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AssistantSize {
    width: u32,
    height: u32,
}

#[tauri::command]
pub fn toggle_quick_panel(
    window: WebviewWindow,
    assistant_state: State<'_, AssistantWindowState>,
    preferences: State<'_, PersistentPreferences>,
) -> Result<bool, String> {
    let assistant = assistant_window(&window)?;
    let current_mode = current_mode(&assistant_state)?;
    if current_mode == AssistantMode::Panel
        && assistant.is_visible().map_err(|error| error.to_string())?
    {
        hide_window(&assistant, &assistant_state)?;
        return Ok(false);
    }

    show_window(
        &window,
        &assistant,
        AssistantMode::Panel,
        AssistantPayload::Panel,
        preferences.snapshot().always_on_top,
        &assistant_state,
    )?;
    Ok(true)
}

#[tauri::command]
pub fn show_speech_bubble(
    message: String,
    duration_ms: Option<u64>,
    window: WebviewWindow,
    assistant_state: State<'_, AssistantWindowState>,
    preferences: State<'_, PersistentPreferences>,
) -> Result<bool, String> {
    if !preferences.snapshot().speech_bubbles_enabled {
        return Ok(false);
    }

    let message = message.trim();
    if message.is_empty() {
        return Ok(false);
    }
    let duration_ms = duration_ms.unwrap_or(4_000).clamp(2_000, 8_000);
    let assistant = assistant_window(&window)?;
    show_window(
        &window,
        &assistant,
        AssistantMode::Bubble,
        AssistantPayload::Bubble {
            message: message.to_string(),
            duration_ms,
            placement: BubblePlacement::Above,
        },
        preferences.snapshot().always_on_top,
        &assistant_state,
    )?;
    Ok(true)
}

#[tauri::command]
pub fn hide_assistant_window(
    window: WebviewWindow,
    assistant_state: State<'_, AssistantWindowState>,
) -> Result<(), String> {
    let assistant = assistant_window(&window)?;
    hide_window(&assistant, &assistant_state)
}

#[tauri::command]
pub fn request_pet_interaction(
    animation_name: String,
    window: WebviewWindow,
) -> Result<(), String> {
    let allowed = ["waving", "review", "failed", "waiting", "running", "random"];
    if !allowed.contains(&animation_name.as_str()) {
        return Err("不支持的宠物互动动作".to_string());
    }

    window
        .emit_to(
            "main",
            "pet-direct-interaction",
            PetInteractionPayload { animation_name },
        )
        .map_err(|error| error.to_string())
}

pub fn hide_for_app(
    app: &tauri::AppHandle,
    assistant_state: &AssistantWindowState,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("assistant") {
        hide_window(&window, assistant_state)?;
    }
    Ok(())
}

fn assistant_window(window: &WebviewWindow) -> Result<WebviewWindow, String> {
    window
        .app_handle()
        .get_webview_window("assistant")
        .ok_or_else(|| "找不到快捷功能窗口".to_string())
}

fn current_mode(state: &AssistantWindowState) -> Result<AssistantMode, String> {
    state
        .mode
        .lock()
        .map(|mode| *mode)
        .map_err(|error| error.to_string())
}

fn set_mode(state: &AssistantWindowState, mode: AssistantMode) -> Result<(), String> {
    let mut current = state.mode.lock().map_err(|error| error.to_string())?;
    *current = mode;
    Ok(())
}

fn set_payload(
    state: &AssistantWindowState,
    payload: Option<AssistantPayload>,
) -> Result<(), String> {
    let mut current = state.payload.lock().map_err(|error| error.to_string())?;
    *current = payload;
    Ok(())
}

fn hide_window(assistant: &WebviewWindow, state: &AssistantWindowState) -> Result<(), String> {
    assistant.hide().map_err(|error| error.to_string())?;
    set_payload(state, None)?;
    set_mode(state, AssistantMode::Hidden)
}

fn show_window(
    source_window: &WebviewWindow,
    assistant: &WebviewWindow,
    mode: AssistantMode,
    mut payload: AssistantPayload,
    always_on_top: bool,
    state: &AssistantWindowState,
) -> Result<(), String> {
    let was_bubble = current_mode(state)? == AssistantMode::Bubble;
    let already_visible = assistant.is_visible().map_err(|error| error.to_string())?;
    let only_queueing_bubble = mode == AssistantMode::Bubble && was_bubble && already_visible;

    if !only_queueing_bubble {
        assistant.hide().map_err(|error| error.to_string())?;
        assistant
            .set_focusable(mode == AssistantMode::Panel)
            .map_err(|error| error.to_string())?;
        let (width, height) = logical_size(mode);
        assistant
            .set_size(Size::Logical(LogicalSize::new(width, height)))
            .map_err(|error| error.to_string())?;
        assistant
            .set_always_on_top(always_on_top)
            .map_err(|error| error.to_string())?;
    }

    if let Some(placement) = position_window(source_window, assistant, mode)? {
        if let AssistantPayload::Bubble {
            placement: current, ..
        } = &mut payload
        {
            *current = placement;
        }
    }

    set_payload(state, Some(payload.clone()))?;
    assistant
        .emit("assistant-payload", payload)
        .map_err(|error| error.to_string())?;
    set_mode(state, mode)?;

    if !only_queueing_bubble {
        assistant.show().map_err(|error| error.to_string())?;
        if mode == AssistantMode::Panel {
            assistant.set_focus().map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn logical_size(mode: AssistantMode) -> (f64, f64) {
    match mode {
        AssistantMode::Panel => (PANEL_WIDTH, PANEL_HEIGHT),
        AssistantMode::Bubble => (BUBBLE_WIDTH, BUBBLE_HEIGHT),
        AssistantMode::Hidden => (0.0, 0.0),
    }
}

fn position_window(
    source_window: &WebviewWindow,
    assistant: &WebviewWindow,
    mode: AssistantMode,
) -> Result<Option<BubblePlacement>, String> {
    let pet = source_window
        .app_handle()
        .get_webview_window("main")
        .ok_or_else(|| "找不到宠物窗口".to_string())?;
    let pet_position = pet.outer_position().map_err(|error| error.to_string())?;
    let pet_size = pet.outer_size().map_err(|error| error.to_string())?;
    let pet_center_x = pet_position.x + pet_size.width as i32 / 2;
    let pet_center_y = pet_position.y + pet_size.height as i32 / 2;
    let monitors = pet
        .available_monitors()
        .map_err(|error| error.to_string())?;
    let monitor = monitors
        .iter()
        .find(|monitor| {
            let area = monitor.work_area();
            pet_center_x >= area.position.x
                && pet_center_x < area.position.x + area.size.width as i32
                && pet_center_y >= area.position.y
                && pet_center_y < area.position.y + area.size.height as i32
        })
        .or_else(|| monitors.first())
        .ok_or_else(|| "找不到可用显示器".to_string())?;
    let area = monitor.work_area();
    let scale_factor = monitor.scale_factor().max(0.1);
    let (logical_width, logical_height) = logical_size(mode);
    let assistant_size = AssistantSize {
        width: (logical_width * scale_factor).round() as u32,
        height: (logical_height * scale_factor).round() as u32,
    };
    let work_area = PhysicalBounds {
        x: area.position.x,
        y: area.position.y,
        width: area.size.width,
        height: area.size.height,
    };
    let pet_bounds = PhysicalBounds {
        x: pet_position.x,
        y: pet_position.y,
        width: pet_size.width,
        height: pet_size.height,
    };
    let gap = (WINDOW_GAP * scale_factor).round() as i32;
    let margin = (WORK_AREA_MARGIN * scale_factor).round() as i32;
    let positioned = calculate_assistant_position(
        pet_bounds,
        assistant_size,
        work_area,
        gap,
        margin,
        mode == AssistantMode::Bubble,
    );
    assistant
        .set_position(Position::Physical(positioned.position))
        .map_err(|error| error.to_string())?;
    Ok(positioned.bubble_placement)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PositionedAssistant {
    position: PhysicalPosition<i32>,
    bubble_placement: Option<BubblePlacement>,
}

fn calculate_assistant_position(
    pet: PhysicalBounds,
    assistant: AssistantSize,
    work_area: PhysicalBounds,
    gap: i32,
    margin: i32,
    bubble: bool,
) -> PositionedAssistant {
    let min_x = work_area.x + margin;
    let min_y = work_area.y + margin;
    let max_x = (work_area.x + work_area.width as i32 - assistant.width as i32 - margin).max(min_x);
    let max_y =
        (work_area.y + work_area.height as i32 - assistant.height as i32 - margin).max(min_y);

    let (preferred_x, preferred_y, bubble_placement) = if bubble {
        let centered_x = pet.x + pet.width as i32 / 2 - assistant.width as i32 / 2;
        let above_y = pet.y - assistant.height as i32 - gap;
        let below_y = pet.y + pet.height as i32 + gap;
        let (y, placement) = if above_y >= min_y {
            (above_y, BubblePlacement::Above)
        } else {
            (below_y, BubblePlacement::Below)
        };
        (centered_x, y, Some(placement))
    } else {
        let left_x = pet.x - assistant.width as i32 - gap;
        let right_x = pet.x + pet.width as i32 + gap;
        let x = if left_x >= min_x {
            left_x
        } else if right_x <= max_x {
            right_x
        } else {
            let left_space = pet.x - min_x;
            let right_space = max_x - pet.x - pet.width as i32;
            if left_space >= right_space {
                left_x
            } else {
                right_x
            }
        };
        let bottom_aligned_y = pet.y + pet.height as i32 - assistant.height as i32;
        (x, bottom_aligned_y, None)
    };

    PositionedAssistant {
        position: PhysicalPosition::new(
            preferred_x.clamp(min_x, max_x),
            preferred_y.clamp(min_y, max_y),
        ),
        bubble_placement,
    }
}

#[cfg(test)]
mod tests {
    use super::{calculate_assistant_position, AssistantSize, BubblePlacement, PhysicalBounds};
    use tauri::PhysicalPosition;

    const WORK_AREA: PhysicalBounds = PhysicalBounds {
        x: 0,
        y: 0,
        width: 1_920,
        height: 1_040,
    };

    #[test]
    fn panel_prefers_the_left_and_bottom_aligns_with_pet() {
        let position = calculate_assistant_position(
            PhysicalBounds {
                x: 1_700,
                y: 800,
                width: 144,
                height: 156,
            },
            AssistantSize {
                width: 320,
                height: 300,
            },
            WORK_AREA,
            12,
            12,
            false,
        );

        assert_eq!(position.position, PhysicalPosition::new(1_368, 656));
        assert_eq!(position.bubble_placement, None);
    }

    #[test]
    fn panel_moves_right_when_the_pet_is_near_the_left_edge() {
        let position = calculate_assistant_position(
            PhysicalBounds {
                x: 20,
                y: 600,
                width: 144,
                height: 156,
            },
            AssistantSize {
                width: 320,
                height: 300,
            },
            WORK_AREA,
            12,
            12,
            false,
        );

        assert_eq!(position.position.x, 176);
    }

    #[test]
    fn bubble_moves_below_when_there_is_no_space_above() {
        let position = calculate_assistant_position(
            PhysicalBounds {
                x: 500,
                y: 20,
                width: 144,
                height: 156,
            },
            AssistantSize {
                width: 260,
                height: 112,
            },
            WORK_AREA,
            12,
            12,
            true,
        );

        assert_eq!(position.position.y, 188);
        assert_eq!(position.bubble_placement, Some(BubblePlacement::Below));
    }

    #[test]
    fn negative_monitor_coordinates_are_clamped_to_its_work_area() {
        let position = calculate_assistant_position(
            PhysicalBounds {
                x: -1_900,
                y: 900,
                width: 144,
                height: 156,
            },
            AssistantSize {
                width: 320,
                height: 300,
            },
            PhysicalBounds {
                x: -1_920,
                y: 0,
                width: 1_920,
                height: 1_040,
            },
            12,
            12,
            false,
        );

        assert!(position.position.x >= -1_908);
        assert!(position.position.y <= 728);
    }
}
