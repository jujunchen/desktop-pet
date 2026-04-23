mod config;

use config::WindowConfig;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    window::Color,
    Emitter, LogicalPosition, Manager, Position, Size, WebviewUrl, WebviewWindowBuilder,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::SystemInformation::GetTickCount;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

const BASE_SIZE: f64 = 180.0;
const SCALE_MIN: f64 = 0.1;
const SCALE_MAX: f64 = 1.0;
const SETTINGS_LABEL: &str = "settings";
const MENU_SHOW: &str = "show";
const MENU_HIDE: &str = "hide";
const MENU_SETTINGS: &str = "settings";
const MENU_QUIT: &str = "quit";
const EVT_SCALE_CHANGED: &str = "m1://scale-changed";

#[tauri::command]
fn save_window_scale(app: tauri::AppHandle, scale: f64) -> Result<(), String> {
    let normalized = scale.clamp(SCALE_MIN, SCALE_MAX);
    WindowConfig { scale: normalized }.save()?;
    app.emit(EVT_SCALE_CHANGED, normalized)
        .map_err(|e| e.to_string())
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
    ensure_settings_window(&app)
}

#[tauri::command]
fn set_main_window_scale(app: tauri::AppHandle, scale: f64) -> Result<(), String> {
    let normalized = scale.clamp(SCALE_MIN, SCALE_MAX);
    WindowConfig { scale: normalized }.save()?;
    if let Some(main) = app.get_webview_window("main") {
        apply_window_scale(&main, normalized)?;
    }
    app.emit(EVT_SCALE_CHANGED, normalized)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn show_pet_context_menu(app: tauri::AppHandle, x: f64, y: f64) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("main window not found")?;
    let menu = build_pet_context_menu(&app).map_err(|e| e.to_string())?;
    window
        .popup_menu_at(&menu, Position::Logical(LogicalPosition::new(x, y)))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_system_idle_ms() -> Result<u64, String> {
    #[cfg(target_os = "windows")]
    {
        let mut info = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };

        let ok = unsafe { GetLastInputInfo(&mut info as *mut LASTINPUTINFO) };
        if ok == 0 {
            return Err("GetLastInputInfo failed".to_string());
        }

        let now = unsafe { GetTickCount() };
        let idle = now.wrapping_sub(info.dwTime) as u64;
        return Ok(idle);
    }

    #[allow(unreachable_code)]
    Err("get_system_idle_ms is only supported on Windows".to_string())
}

fn ensure_settings_window(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(SETTINGS_LABEL) {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    WebviewWindowBuilder::new(
        app,
        SETTINGS_LABEL,
        WebviewUrl::App("index.html?window=settings".into()),
    )
    .title("设置")
    .inner_size(360.0, 220.0)
    .resizable(false)
    .minimizable(false)
    .maximizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn build_pet_context_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let hide_i = MenuItem::with_id(app, MENU_HIDE, "隐藏宠物", true, None::<&str>)?;
    let settings_i = MenuItem::with_id(app, MENU_SETTINGS, "设置", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, MENU_QUIT, "退出", true, None::<&str>)?;
    Menu::with_items(app, &[&hide_i, &settings_i, &quit_i])
}

fn handle_menu_action(app: &tauri::AppHandle, id: &str) {
    let window = app.get_webview_window("main");
    match id {
        MENU_SHOW => {
            if let Some(w) = window {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
        MENU_HIDE => {
            if let Some(w) = window {
                let _ = w.hide();
            }
        }
        MENU_SETTINGS => {
            let _ = ensure_settings_window(app);
        }
        MENU_QUIT => {
            app.exit(0);
        }
        _ => {}
    }
}

fn apply_window_scale(window: &tauri::WebviewWindow, scale: f64) -> Result<(), String> {
    let size = BASE_SIZE * scale;
    window
        .set_size(Size::Logical(tauri::LogicalSize::new(size, size)))
        .map_err(|e| e.to_string())
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let show_i = MenuItem::with_id(app, MENU_SHOW, "显示宠物", true, None::<&str>)?;
    let hide_i = MenuItem::with_id(app, MENU_HIDE, "隐藏宠物", true, None::<&str>)?;
    let settings_i = MenuItem::with_id(app, MENU_SETTINGS, "设置", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, MENU_QUIT, "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_i, &hide_i, &settings_i, &quit_i])?;

    let tray = TrayIconBuilder::new()
        .menu(&menu)
        .on_menu_event(|app, event| {
            handle_menu_action(app, event.id.as_ref());
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
            open_settings,
            set_main_window_scale,
            show_pet_context_menu,
            get_system_idle_ms
        ])
        .setup(|app| {
            build_tray(app)?;

            let window = app
                .get_webview_window("main")
                .ok_or("main window not found")?;
            let _ = window.set_background_color(Some(Color(0, 0, 0, 0)));

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
        .on_menu_event(|app, event| {
            handle_menu_action(app, event.id.as_ref());
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
