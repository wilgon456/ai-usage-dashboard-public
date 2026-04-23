use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    AppHandle,
};

#[tauri::command]
pub async fn set_tray_icon(
    app: AppHandle,
    bytes: Vec<u8>,
    label: Option<String>,
) -> Result<(), String> {
    let tray = app.tray_by_id("main").ok_or("tray not found")?;
    let image = Image::from_bytes(&bytes).map_err(|error| error.to_string())?;

    tray.set_icon(Some(image)).map_err(|error| error.to_string())?;

    #[cfg(target_os = "macos")]
    {
        tray.set_title(label.as_deref())
            .map_err(|error| error.to_string())?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = label;
    }

    Ok(())
}

#[tauri::command]
pub async fn set_tray_labels(
    app: AppHandle,
    show_dashboard: String,
    go_to_settings: String,
    quit: String,
) -> Result<(), String> {
    let tray = app.tray_by_id("main").ok_or("tray not found")?;
    let show_dashboard_item = MenuItem::with_id(
        &app,
        "show_dashboard",
        show_dashboard,
        true,
        None::<&str>,
    )
    .map_err(|error| error.to_string())?;
    let go_to_settings_item =
        MenuItem::with_id(&app, "go_to_settings", go_to_settings, true, None::<&str>)
            .map_err(|error| error.to_string())?;
    let separator = PredefinedMenuItem::separator(&app).map_err(|error| error.to_string())?;
    let quit_item =
        MenuItem::with_id(&app, "quit", quit, true, None::<&str>).map_err(|error| error.to_string())?;
    let menu = Menu::with_items(
        &app,
        &[
            &show_dashboard_item,
            &go_to_settings_item,
            &separator,
            &quit_item,
        ],
    )
    .map_err(|error| error.to_string())?;

    tray.set_menu(Some(menu)).map_err(|error| error.to_string())?;

    Ok(())
}
