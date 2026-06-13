use crate::store::Position;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

pub fn create_quick_bar(app: &tauri::AppHandle) {
    let config = {
        let state = app.state::<crate::commands::AppState>();
        let c = state.config.lock().unwrap();
        c.clone()
    };

    let pos = if config.bar_position.x >= 0 && config.bar_position.y >= 0 {
        (config.bar_position.x, config.bar_position.y)
    } else {
        default_bar_position(app)
    };

    if app.get_webview_window("quick-bar").is_some() {
        return;
    }

    let builder = WebviewWindowBuilder::new(app, "quick-bar", WebviewUrl::App("/".into()))
        .title("Q-Prompt")
        .inner_size(700.0, 36.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false)
        .position(pos.0 as f64, pos.1 as f64);

    let window = builder.build().expect("Failed to create quick-bar window");
    apply_noactivate(&window);
    log::info!("Quick bar created at ({}, {})", pos.0, pos.1);
}

pub fn create_manager(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("manager") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    let builder = WebviewWindowBuilder::new(app, "manager", WebviewUrl::App("/manager.html".into()))
        .title("Q-Prompt 管理面板")
        .inner_size(620.0, 480.0)
        .decorations(true)
        .resizable(true)
        .center();

    builder.build().expect("Failed to create manager window");
    log::info!("Manager window created");
}

pub fn toggle_quick_bar(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("quick-bar") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
            log::info!("Quick bar hidden");
        } else {
            let _ = window.show();
            apply_noactivate(&window);
            log::info!("Quick bar shown");
        }
    } else {
        create_quick_bar(app);
        if let Some(w) = app.get_webview_window("quick-bar") {
            let _ = w.show();
        }
    }
}

pub fn save_bar_position(app: &tauri::AppHandle, x: i32, y: i32) {
    let state = app.state::<crate::commands::AppState>();
    let store = state.store.lock().unwrap();
    let mut config = store.load_config();
    config.bar_position = Position { x, y };
    store.save_config(&config);
    let mut current = state.config.lock().unwrap();
    *current = config;
}

// ── Internal ──

pub fn apply_noactivate(window: &tauri::WebviewWindow) {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;
        extern "system" {
            fn SetWindowLongPtrW(hwnd: *mut c_void, nIndex: i32, dwNewLong: isize) -> isize;
            fn GetWindowLongPtrW(hwnd: *mut c_void, nIndex: i32) -> isize;
        }
        // Get HWND from window
        if let Ok(hwnd) = window.hwnd() {
            unsafe {
                let h = hwnd.0 as *mut c_void;
                let mut ex = GetWindowLongPtrW(h, -20);
                ex &= !0x00040000_isize;
                ex |= 0x00000080_isize;
                ex |= 0x08000000_isize;
                SetWindowLongPtrW(h, -20, ex);
            }
        }
    }
}

fn default_bar_position(app: &tauri::AppHandle) -> (i32, i32) {
    if let Some(window) = app.get_webview_window("quick-bar") {
        let monitors = window.available_monitors().unwrap_or_default();
        if let Some(primary) = monitors.first() {
            let size = primary.size();
            let scale = primary.scale_factor();
            let w = 700.0;
            let w_f64 = size.width as f64 / scale;
            let x = ((w_f64 - w) / 2.0) as i32;
            let y = ((size.height as f64 / scale) - 80.0) as i32;
            return (x.max(0), y.max(0));
        }
    }
    (640, 1360)
}
