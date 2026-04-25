mod config;
mod llm;

use config::{load_config as load_app_config, save_config as save_app_config, AppConfig};
use tauri::{
    menu::{Menu, MenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    window::Color,
    Emitter, LogicalPosition, Manager, Position, Size, WebviewUrl, WebviewWindowBuilder,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::SystemInformation::GetTickCount;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state_id: i32, event_type: u32) -> f64;
}

const BASE_SIZE: f64 = 180.0;
const SCALE_MIN: f64 = 0.1;
const SCALE_MAX: f64 = 1.0;
const SETTINGS_LABEL: &str = "settings";
const MENU_SHOW: &str = "show";
const MENU_HIDE: &str = "hide";
const MENU_SETTINGS: &str = "settings";
const MENU_QUIT: &str = "quit";
const MENU_TEXT_CHAT: &str = "text-chat";
const EVT_SCALE_CHANGED: &str = "m1://scale-changed";
const EVT_CONFIG_CHANGED: &str = "m6://config-changed";
const EVT_OPEN_TEXT_CHAT: &str = "voice://open-text-chat";

fn clamp_scale(scale: f64) -> f64 {
    scale.clamp(SCALE_MIN, SCALE_MAX)
}

fn read_app_config_or_default() -> AppConfig {
    match load_app_config() {
        Ok(conf) => conf,
        Err(_) => AppConfig::default(),
    }
}

fn persist_scale(app: &tauri::AppHandle, scale: f64) -> Result<f64, String> {
    let normalized = clamp_scale(scale);
    let mut conf = read_app_config_or_default();
    conf.pet.scale = normalized;
    let saved = save_app_config(conf)?;

    app.emit(EVT_SCALE_CHANGED, saved.pet.scale)
        .map_err(|e| e.to_string())?;
    app.emit(EVT_CONFIG_CHANGED, &saved)
        .map_err(|e| e.to_string())?;
    Ok(saved.pet.scale)
}

#[tauri::command]
fn save_window_scale(app: tauri::AppHandle, scale: f64) -> Result<(), String> {
    let _ = persist_scale(&app, scale)?;
    Ok(())
}

#[tauri::command]
fn load_window_scale() -> f64 {
    clamp_scale(read_app_config_or_default().pet.scale)
}

#[tauri::command]
fn load_config() -> Result<AppConfig, String> {
    load_app_config()
}

#[tauri::command]
fn save_config(app: tauri::AppHandle, config: AppConfig) -> Result<AppConfig, String> {
    let saved = save_app_config(config)?;

    app.emit(EVT_SCALE_CHANGED, saved.pet.scale)
        .map_err(|e| e.to_string())?;
    app.emit(EVT_CONFIG_CHANGED, &saved)
        .map_err(|e| e.to_string())?;

    Ok(saved)
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
    let normalized = persist_scale(&app, scale)?;
    if let Some(main) = app.get_webview_window("main") {
        apply_window_scale(&main, normalized)?;
    }
    Ok(())
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
async fn chat_with_llm_stream(app: tauri::AppHandle, prompt: String) -> Result<(), String> {
    let config = read_app_config_or_default();
    llm::chat_with_llm_stream(app, config.llm, prompt).await
}

#[tauri::command]
fn open_chat_window(app: tauri::AppHandle) -> Result<(), String> {
    const CHAT_WINDOW_LABEL: &str = "chat";

    if let Some(window) = app.get_webview_window(CHAT_WINDOW_LABEL) {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    tauri::WebviewWindowBuilder::new(
        &app,
        CHAT_WINDOW_LABEL,
        tauri::WebviewUrl::App("index.html?window=chat".into()),
    )
    .title("和小白聊天")
    .inner_size(420.0, 560.0)
    .resizable(true)
    .minimizable(true)
    .maximizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
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

    #[cfg(target_os = "macos")]
    {
        // kCGEventSourceStateHIDSystemState = 1
        // kCGAnyInputEventType = 0xFFFFFFFF
        let seconds = unsafe { CGEventSourceSecondsSinceLastEventType(1, u32::MAX) };
        if !seconds.is_finite() || seconds < 0.0 {
            return Err("CGEventSourceSecondsSinceLastEventType returned invalid value".to_string());
        }
        return Ok((seconds * 1000.0) as u64);
    }

    #[allow(unreachable_code)]
    Err("get_system_idle_ms is only supported on Windows/macOS".to_string())
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
    .inner_size(520.0, 640.0)
    .resizable(true)
    .minimizable(true)
    .maximizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn build_pet_context_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let chat_i = MenuItem::with_id(app, MENU_TEXT_CHAT, "文本对话", true, None::<&str>)?;
    let hide_i = MenuItem::with_id(app, MENU_HIDE, "隐藏宠物", true, None::<&str>)?;
    let settings_i = MenuItem::with_id(app, MENU_SETTINGS, "设置", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, MENU_QUIT, "退出", true, None::<&str>)?;
    Menu::with_items(app, &[&chat_i, &hide_i, &settings_i, &quit_i])
}

#[cfg(target_os = "macos")]
fn build_macos_app_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let settings_i = MenuItem::with_id(app, MENU_SETTINGS, "设置", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, MENU_QUIT, "退出", true, None::<&str>)?;
    let app_submenu = Submenu::with_items(app, "desktop-pet", true, &[&settings_i, &quit_i])?;
    Menu::with_items(app, &[&app_submenu])
}

fn handle_menu_action(app: &tauri::AppHandle, id: &str) {
    let window = app.get_webview_window("main");
    match id {
        MENU_TEXT_CHAT => {
            let _ = open_chat_window(app.clone());
        }
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
    let size = BASE_SIZE * clamp_scale(scale);
    window
        .set_size(Size::Logical(tauri::LogicalSize::new(size, size)))
        .map_err(|e| e.to_string())
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let chat_i = MenuItem::with_id(app, MENU_TEXT_CHAT, "文本对话", true, None::<&str>)?;
    let show_i = MenuItem::with_id(app, MENU_SHOW, "显示宠物", true, None::<&str>)?;
    let hide_i = MenuItem::with_id(app, MENU_HIDE, "隐藏宠物", true, None::<&str>)?;
    let settings_i = MenuItem::with_id(app, MENU_SETTINGS, "设置", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, MENU_QUIT, "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&chat_i, &show_i, &hide_i, &settings_i, &quit_i])?;

    let mut tray = TrayIconBuilder::new().menu(&menu);
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    let tray = tray
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
    let mut builder = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            save_window_scale,
            load_window_scale,
            load_config,
            save_config,
            hide_main_window,
            show_main_window,
            open_settings,
            open_chat_window,
            set_main_window_scale,
            show_pet_context_menu,
            get_system_idle_ms,
            chat_with_llm_stream
        ])
        .setup(|app| {
            build_tray(app)?;

            let window = app
                .get_webview_window("main")
                .ok_or("main window not found")?;
            let _ = window.set_background_color(Some(Color(0, 0, 0, 0)));

            let conf = read_app_config_or_default();
            apply_window_scale(&window, conf.pet.scale)?;

            // Place at bottom-right when app starts.
            if let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? {
                let monitor_size = monitor.size().to_logical::<f64>(monitor.scale_factor());
                let margin = 24.0;
                let win_size = BASE_SIZE * clamp_scale(conf.pet.scale);
                let x = (monitor_size.width - win_size - margin).max(0.0);
                let y = (monitor_size.height - win_size - margin).max(0.0);
                let _ = window.set_position(Position::Logical(tauri::LogicalPosition::new(x, y)));
            }

            Ok(())
        })
        .on_menu_event(|app, event| {
            handle_menu_action(app, event.id.as_ref());
        });

    #[cfg(target_os = "macos")]
    {
        builder = builder.menu(|app| build_macos_app_menu(app));
    }

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
