mod config;

use config::WindowConfig;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, Position, Size,
};

const BASE_SIZE: f64 = 180.0;
const SCALE_MIN: f64 = 0.5;
const SCALE_MAX: f64 = 3.0;

#[tauri::command]
fn save_window_scale(scale: f64) -> Result<(), String> {
    let normalized = scale.clamp(SCALE_MIN, SCALE_MAX);
    WindowConfig { scale: normalized }.save()
}

#[tauri::command]
fn load_window_scale() -> f64 {
    WindowConfig::load().scale.clamp(SCALE_MIN, SCALE_MAX)
}

#[tauri::command]
fn hide_main_window(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("main window not found")?;
    window.hide().map_err(|e| e.to_string())
}

#[tauri::command]
fn show_main_window(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("main window not found")?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) -> Result<(), String> {
    app.emit("m1://open-settings", ()).map_err(|e| e.to_string())
}

fn apply_window_scale(window: &tauri::WebviewWindow, scale: f64) -> Result<(), String> {
    let size = BASE_SIZE * scale;
    window
        .set_size(Size::Logical(tauri::LogicalSize::new(size, size)))
        .map_err(|e| e.to_string())
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let show_i = MenuItem::with_id(app, "show", "显示宠物", true, None::<&str>)?;
    let hide_i = MenuItem::with_id(app, "hide", "隐藏宠物", true, None::<&str>)?;
    let settings_i = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_i, &hide_i, &settings_i, &quit_i])?;

    let tray = TrayIconBuilder::new().menu(&menu).on_menu_event(|app, event| {
        let window = app.get_webview_window("main");
        match event.id.as_ref() {
            "show" => {
                if let Some(w) = window {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "hide" => {
                if let Some(w) = window {
                    let _ = w.hide();
                }
            }
            "settings" => {
                let _ = app.emit("m1://open-settings", ());
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        }
    })
    .on_tray_icon_event(|tray, event| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            if let Some(window) = tray.app_handle().get_webview_window("main") {
                let visible = window.is_visible().unwrap_or(true);
                if visible {
                    let _ = window.hide();
                } else {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        }
    });

    tray.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            save_window_scale,
            load_window_scale,
            hide_main_window,
            show_main_window,
            open_settings
        ])
        .setup(|app| {
            build_tray(app)?;

            let window = app
                .get_webview_window("main")
                .ok_or("main window not found")?;

            let conf = WindowConfig::load();
            apply_window_scale(&window, conf.scale)?;

            // Place at bottom-right when app starts.
            if let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? {
                let monitor_size = monitor.size().to_logical::<f64>(monitor.scale_factor());
                let margin = 24.0;
                let win_size = BASE_SIZE * conf.scale;
                let x = (monitor_size.width - win_size - margin).max(0.0);
                let y = (monitor_size.height - win_size - margin).max(0.0);
                let _ = window.set_position(Position::Logical(tauri::LogicalPosition::new(x, y)));
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
