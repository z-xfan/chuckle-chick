mod assistant;
mod preferences;
mod tray;

use assistant::AssistantWindowState;
use preferences::{AppPreferences, PersistentPreferences, SavedPosition};
use serde::Serialize;
use tauri::{
    LogicalSize, Manager, PhysicalPosition, Position, Size, State, WebviewWindow, WindowEvent,
};
use tray::TrayMenuState;

const PET_WIDTH: f64 = 144.0;
const PET_HEIGHT: f64 = 156.0;
const DEFAULT_MARGIN_LOGICAL: f64 = 32.0;

#[derive(Serialize)]
struct PointerSnapshot {
    cursor: SnapshotPoint,
    #[serde(rename = "windowRect")]
    window_rect: SnapshotRect,
}

#[derive(Serialize)]
struct SnapshotPoint {
    x: f64,
    y: f64,
}

#[derive(Serialize)]
struct SnapshotSize {
    width: f64,
    height: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotRect {
    position: SnapshotPoint,
    size: SnapshotSize,
    scale_factor: f64,
}

#[tauri::command]
fn get_preferences(state: State<'_, PersistentPreferences>) -> AppPreferences {
    state.snapshot()
}

#[tauri::command]
fn set_pet_always_on_top(
    enabled: bool,
    window: WebviewWindow,
    state: State<'_, PersistentPreferences>,
    tray_menu: State<'_, TrayMenuState>,
) -> Result<(), String> {
    let pet_window = window
        .app_handle()
        .get_webview_window("main")
        .ok_or_else(|| "找不到宠物窗口".to_string())?;
    pet_window
        .set_always_on_top(enabled)
        .map_err(|error| error.to_string())?;
    if let Some(assistant_window) = window.app_handle().get_webview_window("assistant") {
        assistant_window
            .set_always_on_top(enabled)
            .map_err(|error| error.to_string())?;
    }
    tray_menu
        .always_on_top
        .set_checked(enabled)
        .map_err(|error| error.to_string())?;
    state.update(|preferences| preferences.always_on_top = enabled)
}

#[tauri::command]
fn set_pet_scale(
    scale: f64,
    window: WebviewWindow,
    state: State<'_, PersistentPreferences>,
) -> Result<(), String> {
    if !(0.4..=2.0).contains(&scale) {
        return Err("宠物缩放必须在 40% 到 200% 之间".to_string());
    }
    let pet_window = window
        .app_handle()
        .get_webview_window("main")
        .ok_or_else(|| "找不到宠物窗口".to_string())?;
    pet_window
        .set_size(Size::Logical(LogicalSize::new(
            PET_WIDTH * scale,
            PET_HEIGHT * scale,
        )))
        .map_err(|error| error.to_string())?;
    state.update(|preferences| preferences.scale = scale)
}

#[tauri::command]
fn close_settings_window(window: WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|error| error.to_string())
}

#[tauri::command]
fn open_settings_window(
    window: WebviewWindow,
    assistant_state: State<'_, AssistantWindowState>,
) -> Result<(), String> {
    assistant::hide_for_app(window.app_handle(), &assistant_state)?;
    let settings = window
        .app_handle()
        .get_webview_window("settings")
        .ok_or_else(|| "找不到设置窗口".to_string())?;
    settings.show().map_err(|error| error.to_string())?;
    settings.set_focus().map_err(|error| error.to_string())
}

#[tauri::command]
fn set_speech_bubbles_enabled(
    enabled: bool,
    window: WebviewWindow,
    state: State<'_, PersistentPreferences>,
    assistant_state: State<'_, AssistantWindowState>,
) -> Result<(), String> {
    if !enabled {
        assistant::hide_for_app(window.app_handle(), &assistant_state)?;
    }
    state.update(|preferences| preferences.speech_bubbles_enabled = enabled)
}

#[tauri::command]
fn get_pointer_snapshot(window: WebviewWindow) -> Result<Option<PointerSnapshot>, String> {
    let pet_window = window
        .app_handle()
        .get_webview_window("main")
        .ok_or_else(|| "找不到宠物窗口".to_string())?;
    if !pet_window.is_visible().map_err(|error| error.to_string())? {
        return Ok(None);
    }

    let cursor = pet_window
        .cursor_position()
        .map_err(|error| error.to_string())?;
    let position = pet_window
        .outer_position()
        .map_err(|error| error.to_string())?;
    let size = pet_window.outer_size().map_err(|error| error.to_string())?;
    let scale_factor = pet_window
        .scale_factor()
        .map_err(|error| error.to_string())?;

    #[cfg(target_os = "macos")]
    let snapshot = {
        // tao scales the global cursor with the primary monitor scale, while
        // window geometry uses the window's current monitor scale.
        let cursor_scale_factor = pet_window
            .primary_monitor()
            .map_err(|error| error.to_string())?
            .map(|monitor| monitor.scale_factor())
            .unwrap_or(1.0);
        pointer_snapshot_from_logical_coordinates(
            cursor.x,
            cursor.y,
            position.x as f64,
            position.y as f64,
            size.width as f64,
            size.height as f64,
            cursor_scale_factor,
            scale_factor,
        )
    };

    #[cfg(not(target_os = "macos"))]
    let snapshot = PointerSnapshot {
        cursor: SnapshotPoint {
            x: cursor.x,
            y: cursor.y,
        },
        window_rect: SnapshotRect {
            position: SnapshotPoint {
                x: position.x as f64,
                y: position.y as f64,
            },
            size: SnapshotSize {
                width: size.width as f64,
                height: size.height as f64,
            },
            scale_factor,
        },
    };

    Ok(Some(snapshot))
}

#[tauri::command]
fn is_left_mouse_button_pressed() -> Option<bool> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSEvent;

        // Read current state only while our own drag is active. This is a
        // point-in-time query, not a global event hook or click listener.
        return Some(NSEvent::pressedMouseButtons() & 1 == 1);
    }

    #[cfg(not(target_os = "macos"))]
    None
}

#[cfg(any(target_os = "macos", test))]
#[allow(clippy::too_many_arguments)]
fn pointer_snapshot_from_logical_coordinates(
    cursor_x: f64,
    cursor_y: f64,
    window_x: f64,
    window_y: f64,
    window_width: f64,
    window_height: f64,
    cursor_scale_factor: f64,
    window_scale_factor: f64,
) -> PointerSnapshot {
    let cursor_scale_factor = cursor_scale_factor.max(0.1);
    let window_scale_factor = window_scale_factor.max(0.1);
    PointerSnapshot {
        cursor: SnapshotPoint {
            x: cursor_x / cursor_scale_factor,
            y: cursor_y / cursor_scale_factor,
        },
        window_rect: SnapshotRect {
            position: SnapshotPoint {
                x: window_x / window_scale_factor,
                y: window_y / window_scale_factor,
            },
            size: SnapshotSize {
                width: window_width / window_scale_factor,
                height: window_height / window_scale_factor,
            },
            scale_factor: 1.0,
        },
    }
}

#[tauri::command]
fn reset_pet_position(
    window: WebviewWindow,
    state: State<'_, PersistentPreferences>,
) -> Result<(), String> {
    let pet_window = window
        .app_handle()
        .get_webview_window("main")
        .ok_or_else(|| "找不到宠物窗口".to_string())?;
    let position = place_at_default_position(&pet_window).map_err(|error| error.to_string())?;
    let position = position.ok_or_else(|| "找不到可用显示器".to_string())?;
    let scale_factor = pet_window
        .scale_factor()
        .map_err(|error| error.to_string())?;
    state.update(|preferences| {
        preferences.position = Some(SavedPosition {
            x: position.x,
            y: position.y,
            scale_factor,
        });
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_preferences,
            set_pet_always_on_top,
            set_pet_scale,
            close_settings_window,
            open_settings_window,
            set_speech_bubbles_enabled,
            get_pointer_snapshot,
            is_left_mouse_button_pressed,
            reset_pet_position,
            assistant::toggle_quick_panel,
            assistant::get_assistant_payload,
            assistant::show_speech_bubble,
            assistant::hide_assistant_window,
            assistant::request_pet_interaction
        ])
        .setup(|app| {
            let config_path = app.path().app_config_dir()?.join("preferences.json");
            let state = PersistentPreferences::load(config_path);
            let preferences = state.snapshot();
            app.manage(state.clone());
            app.manage(AssistantWindowState::default());

            let pet_window = app
                .get_webview_window("main")
                .expect("main pet window must exist");
            pet_window.set_always_on_top(preferences.always_on_top)?;
            pet_window.set_size(Size::Logical(LogicalSize::new(
                PET_WIDTH * preferences.scale,
                PET_HEIGHT * preferences.scale,
            )))?;
            restore_position(&pet_window, &preferences)?;

            let movement_state = state.clone();
            let movement_window = pet_window.clone();
            pet_window.on_window_event(move |event| {
                if let WindowEvent::Moved(position) = event {
                    movement_state.update_position(SavedPosition {
                        x: position.x,
                        y: position.y,
                        scale_factor: movement_window.scale_factor().unwrap_or(1.0),
                    });
                }
            });

            if let Some(settings_window) = app.get_webview_window("settings") {
                let window_to_hide = settings_window.clone();
                settings_window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_to_hide.hide();
                    }
                });
            }

            if let Some(assistant_window) = app.get_webview_window("assistant") {
                assistant_window.set_always_on_top(preferences.always_on_top)?;
                assistant_window.set_focusable(false)?;
                let window_to_hide = assistant_window.clone();
                assistant_window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_to_hide.hide();
                    }
                });
            }

            let tray_menu = tray::create_tray(app.handle())?;
            app.manage(tray_menu);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run ChuckleChick");
}

fn restore_position(window: &WebviewWindow, preferences: &AppPreferences) -> tauri::Result<()> {
    let Some(saved) = &preferences.position else {
        place_at_default_position(window)?;
        return Ok(());
    };

    let current_scale_factor = window.scale_factor()?;
    let saved_scale_factor = saved.scale_factor.max(0.1);
    let mut x = (saved.x as f64 * current_scale_factor / saved_scale_factor).round() as i32;
    let mut y = (saved.y as f64 * current_scale_factor / saved_scale_factor).round() as i32;
    let window_size = window.outer_size()?;
    let monitors = window.available_monitors()?;

    let selected_monitor = monitors
        .iter()
        .find(|monitor| {
            let position = monitor.position();
            let size = monitor.size();
            x >= position.x
                && x < position.x + size.width as i32
                && y >= position.y
                && y < position.y + size.height as i32
        })
        .or_else(|| monitors.first());

    if let Some(monitor) = selected_monitor {
        let monitor_position = monitor.position();
        let monitor_size = monitor.size();
        let max_x =
            monitor_position.x + monitor_size.width.saturating_sub(window_size.width) as i32;
        let max_y =
            monitor_position.y + monitor_size.height.saturating_sub(window_size.height) as i32;
        x = x.clamp(monitor_position.x, max_x.max(monitor_position.x));
        y = y.clamp(monitor_position.y, max_y.max(monitor_position.y));
    }

    window.set_position(Position::Physical(PhysicalPosition::new(x, y)))
}

fn place_at_default_position(
    window: &WebviewWindow,
) -> tauri::Result<Option<PhysicalPosition<i32>>> {
    let monitor = match window.primary_monitor()? {
        Some(monitor) => monitor,
        None => match window.available_monitors()?.into_iter().next() {
            Some(monitor) => monitor,
            None => return Ok(None),
        },
    };
    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let window_size = window.outer_size()?;
    let margin = (DEFAULT_MARGIN_LOGICAL * monitor.scale_factor()).round() as i32;
    let x =
        monitor_position.x + monitor_size.width.saturating_sub(window_size.width) as i32 - margin;
    let y =
        monitor_position.y + monitor_size.height.saturating_sub(window_size.height) as i32 - margin;
    let position = PhysicalPosition::new(x.max(monitor_position.x), y.max(monitor_position.y));
    window.set_position(Position::Physical(position))?;
    Ok(Some(position))
}

#[cfg(test)]
mod tests {
    use super::pointer_snapshot_from_logical_coordinates;

    #[test]
    fn macos_pointer_and_external_window_are_normalized_independently() {
        let snapshot = pointer_snapshot_from_logical_coordinates(
            2_400.0, 800.0, 1_000.0, 300.0, 144.0, 156.0, 2.0, 1.0,
        );

        assert_eq!(snapshot.cursor.x, 1_200.0);
        assert_eq!(snapshot.cursor.y, 400.0);
        assert_eq!(snapshot.window_rect.position.x, 1_000.0);
        assert_eq!(snapshot.window_rect.position.y, 300.0);
        assert_eq!(snapshot.window_rect.size.width, 144.0);
        assert_eq!(snapshot.window_rect.scale_factor, 1.0);
    }
}
