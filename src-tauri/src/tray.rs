use tauri::{
    menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    AppHandle, Manager, Wry,
};

use crate::{
    assistant::{self, AssistantWindowState},
    preferences::PersistentPreferences,
};

pub struct TrayMenuState {
    pub always_on_top: CheckMenuItem<Wry>,
}

pub fn create_tray(app: &AppHandle) -> tauri::Result<TrayMenuState> {
    let preferences = app.state::<PersistentPreferences>().snapshot();
    let show_hide = MenuItemBuilder::with_id("toggle-visibility", "隐藏宠物").build(app)?;
    let always_on_top = CheckMenuItemBuilder::with_id("always-on-top", "窗口置顶")
        .checked(preferences.always_on_top)
        .build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "设置…").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出 ChuckleChick").build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&show_hide)
        .item(&always_on_top)
        .item(&settings)
        .separator()
        .item(&quit)
        .build()?;

    let show_hide_for_event = show_hide.clone();
    let always_on_top_for_event = always_on_top.clone();
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".into()))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("ChuckleChick")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "toggle-visibility" => {
                if let Some(window) = app.get_webview_window("main") {
                    let visible = window.is_visible().unwrap_or(true);
                    if visible {
                        let assistant_state = app.state::<AssistantWindowState>();
                        let _ = assistant::hide_for_app(app, &assistant_state);
                        let _ = window.hide();
                        let _ = show_hide_for_event.set_text("显示宠物");
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                        let _ = show_hide_for_event.set_text("隐藏宠物");
                    }
                }
            }
            "always-on-top" => {
                let state = app.state::<PersistentPreferences>();
                let enabled = !state.snapshot().always_on_top;
                if let Some(window) = app.get_webview_window("main") {
                    if window.set_always_on_top(enabled).is_ok() {
                        if let Some(assistant) = app.get_webview_window("assistant") {
                            let _ = assistant.set_always_on_top(enabled);
                        }
                        let _ = always_on_top_for_event.set_checked(enabled);
                        let _ = state.update(|preferences| preferences.always_on_top = enabled);
                    }
                }
            }
            "settings" => {
                let assistant_state = app.state::<AssistantWindowState>();
                let _ = assistant::hide_for_app(app, &assistant_state);
                if let Some(window) = app.get_webview_window("settings") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                let assistant_state = app.state::<AssistantWindowState>();
                let _ = assistant::hide_for_app(app, &assistant_state);
                let _ = app.state::<PersistentPreferences>().persist();
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(TrayMenuState { always_on_top })
}
