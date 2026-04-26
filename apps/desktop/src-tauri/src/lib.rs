mod commands;
mod credentials;
mod tray_assets;

// NOTE: Windows Rust targets could not be installed in this sandbox because the toolchain here cannot provision target components; cfg guards remain in place for cross-platform builds once the target is available.

use std::sync::Arc;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, PhysicalPosition, Position, Rect, WebviewWindow, WindowEvent};

const TRAY_WINDOW_GAP: f64 = 8.0;

fn should_auto_hide_on_blur() -> bool {
    if std::env::var("VITE_WIDGET_SYNC_TOKEN").is_ok() {
        return false;
    }

    if cfg!(debug_assertions) && std::env::var("AI_USAGE_PIN_WINDOW").is_ok() {
        return false;
    }

    true
}

fn clamp_window_position(
    desired_left: f64,
    desired_top: f64,
    window_size: tauri::PhysicalSize<u32>,
    monitor: &tauri::Monitor,
) -> PhysicalPosition<i32> {
    let work_area = monitor.work_area();
    let min_x = f64::from(work_area.position.x);
    let min_y = f64::from(work_area.position.y);
    let max_x = f64::from(work_area.position.x)
        + (f64::from(work_area.size.width) - f64::from(window_size.width)).max(0.0);
    let max_y = f64::from(work_area.position.y)
        + (f64::from(work_area.size.height) - f64::from(window_size.height)).max(0.0);

    PhysicalPosition::new(
        desired_left.clamp(min_x, max_x).round() as i32,
        desired_top.clamp(min_y, max_y).round() as i32,
    )
}

#[cfg(target_os = "macos")]
fn position_window_near_tray_macos(window: &WebviewWindow, rect: &Rect) {
    let Ok(window_size) = window.outer_size() else {
        return;
    };
    let scale_factor = window.scale_factor().unwrap_or(1.0);
    let rect_position = rect.position.to_physical::<i32>(scale_factor);
    let rect_size = rect.size.to_physical::<u32>(scale_factor);
    let tray_center_x = f64::from(rect_position.x) + f64::from(rect_size.width) / 2.0;
    let tray_bottom = f64::from(rect_position.y) + f64::from(rect_size.height);
    let monitor = window
        .monitor_from_point(tray_center_x, tray_bottom)
        .ok()
        .flatten()
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten());

    if let Some(monitor) = monitor {
        let position = clamp_window_position(
            tray_center_x - f64::from(window_size.width) / 2.0,
            tray_bottom + TRAY_WINDOW_GAP,
            window_size,
            &monitor,
        );
        let _ = window.set_position(Position::Physical(position));
    }
}

#[cfg(target_os = "windows")]
fn position_window_near_cursor_windows(window: &WebviewWindow, cursor: &PhysicalPosition<f64>) {
    let Ok(window_size) = window.outer_size() else {
        return;
    };
    let monitor = window
        .monitor_from_point(cursor.x, cursor.y)
        .ok()
        .flatten()
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten());

    if let Some(monitor) = monitor {
        let position = clamp_window_position(
            cursor.x - f64::from(window_size.width),
            cursor.y + TRAY_WINDOW_GAP,
            window_size,
            &monitor,
        );
        let _ = window.set_position(Position::Physical(position));
    }
}

fn show_window(window: &WebviewWindow) {
    let _ = window.set_always_on_top(true);
    let _ = window.show();
    let app_handle = window.app_handle().clone();
    let label = window.label().to_string();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(16));
        if let Some(window) = app_handle.get_webview_window(&label) {
            let _ = window.set_focus();
        }
    });
}

fn create_tray(app_handle: &tauri::AppHandle) -> tauri::Result<()> {
    let tray_icon = tray_assets::load_white_masked_tray_icon()
        .map_err(|error| tauri::Error::Io(std::io::Error::other(error)))?;

    let show_dashboard = MenuItem::with_id(
        app_handle,
        "show_dashboard",
        "대시보드 열기",
        true,
        None::<&str>,
    )?;
    let go_to_settings = MenuItem::with_id(
        app_handle,
        "go_to_settings",
        "설정 열기",
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app_handle)?;
    let quit = MenuItem::with_id(app_handle, "quit", "종료", true, None::<&str>)?;

    let menu = Menu::with_items(
        app_handle,
        &[&show_dashboard, &go_to_settings, &separator, &quit],
    )?;

    TrayIconBuilder::with_id("main")
        .icon(tray_icon)
        .icon_as_template(false)
        .tooltip("AI Usage Dashboard")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show_dashboard" => {
                if let Some(window) = app.get_webview_window("main") {
                    show_window(&window);
                    let _ = app.emit("tray:navigate", "home");
                }
            }
            "go_to_settings" => {
                if let Some(window) = app.get_webview_window("main") {
                    show_window(&window);
                    let _ = app.emit("tray:navigate", "settings");
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button_state: MouseButtonState::Up,
                position,
                rect,
                ..
            } = event
                && let Some(window) = tray.app_handle().get_webview_window("main")
            {
                match window.is_visible() {
                    Ok(true) => {
                        let _ = window.hide();
                    }
                    Ok(false) | Err(_) => {
                        #[cfg(target_os = "macos")]
                        let _ = &position;
                        #[cfg(target_os = "macos")]
                        position_window_near_tray_macos(&window, &rect);
                        #[cfg(target_os = "windows")]
                        let _ = &rect;
                        #[cfg(target_os = "windows")]
                        position_window_near_cursor_windows(&window, &position);
                        show_window(&window);
                    }
                }
            }
        })
        .build(app_handle)?;

    Ok(())
}

pub fn run() {
    let credential_registry = Arc::new(credentials::CredentialRegistry::new());
    let widget_sync_store = commands::widget_sync::new_store();
    commands::widget_sync::start_server(Arc::clone(&widget_sync_store));
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(Arc::clone(&credential_registry))
        .manage(widget_sync_store);

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ));
    }

    builder
        .invoke_handler(tauri::generate_handler![
            commands::codex::get_codex_usage,
            commands::claude::get_claude_usage,
            commands::copilot::get_copilot_usage,
            commands::openrouter::get_openrouter_usage,
            commands::kimi::get_kimi_usage,
            commands::connect::run_connect_command,
            commands::connect::run_agent_connect_command,
            commands::openrouter::save_openrouter_key,
            commands::openrouter::clear_openrouter_key,
            commands::openrouter::has_openrouter_key,
            commands::platform::detect_platform,
            commands::connect::inspect_provider_bootstrap,
            commands::tray::set_tray_icon,
            commands::tray::set_tray_labels,
            commands::widget_sync::set_widget_sync_config,
            commands::widget_sync::update_widget_snapshot,
            commands::widget_sync::get_widget_sync_urls,
        ])
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }

            create_tray(app.handle())?;

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_always_on_top(true);
                let _ = window.set_skip_taskbar(true);
            }

            #[cfg(target_os = "macos")]
            if let Some(window) = app.get_webview_window("main") {
                if std::env::var("VITE_WIDGET_SYNC_TOKEN").is_err() {
                    let _ = window.hide();
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            WindowEvent::Focused(false)
                if should_auto_hide_on_blur() && window.label() == "main" =>
            {
                let _ = window.hide();
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
