mod commands;
mod scoring;
mod store;
mod window;

use commands::AppState;
use std::sync::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    simple_logger::SimpleLogger::new()
        .with_utc_timestamps()
        .init()
        .ok();

    let store = store::PromptStore::new();
    let config = store.load_config();

    let app_state = AppState {
        store: Mutex::new(store),
        config: Mutex::new(config.clone()),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_prompts,
            commands::get_prompts_sorted,
            commands::add_prompt,
            commands::update_prompt,
            commands::delete_prompt,
            commands::move_prompt,
            commands::reorder_prompts,
            commands::export_prompts,
            commands::import_prompts,
            commands::get_config,
            commands::update_config,
            commands::get_shortcut,
            commands::update_shortcut,
            commands::insert_text,
            commands::capture_selection,
            commands::quick_save,
            commands::record_usage,
            commands::open_manager_window,
        ])
        .setup(move |app| {
            if !check_single_instance() {
                log::info!("Another instance detected, exiting");
                std::process::exit(0);
            }

            let config = app.state::<AppState>().config.lock().unwrap().clone();

            window::create_quick_bar(app.handle());
            register_shortcuts(app.handle(), &config);

            let toggle_bar = MenuItemBuilder::with_id("toggle_bar", "显示/隐藏").build(app)?;
            let sep1 = MenuItemBuilder::with_id("sep1", "──────────").build(app)?;
            let manager_item = MenuItemBuilder::with_id("open_manager", "管理面板").build(app)?;
            let sep2 = MenuItemBuilder::with_id("sep2", "──────────").build(app)?;
            let launch_label = if config.launch_on_startup { "开机自启 ✓" } else { "开机自启" };
            let launch_item = MenuItemBuilder::with_id("launch_startup", launch_label).build(app)?;
            let about_item = MenuItemBuilder::with_id("about", "关于").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&toggle_bar)
                .item(&sep1)
                .item(&manager_item)
                .item(&sep2)
                .item(&launch_item)
                .item(&about_item)
                .item(&quit_item)
                .build()?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Q-Prompt")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "toggle_bar" => window::toggle_quick_bar(app),
                    "open_manager" => window::create_manager(app),
                    "launch_startup" => {
                        let current = {
                            let state = app.state::<AppState>();
                            let c = state.config.lock().unwrap();
                            c.launch_on_startup
                        };
                        set_launch_on_startup(!current);
                        let state = app.state::<AppState>();
                        let store = state.store.lock().unwrap();
                        let mut c = store.load_config();
                        c.launch_on_startup = !current;
                        store.save_config(&c);
                        let mut config_lock = state.config.lock().unwrap();
                        *config_lock = c;
                    }
                    "about" => {
                        use tauri_plugin_dialog::DialogExt;
                        app.dialog()
                            .message("Q-Prompt v0.1.0\n桌面悬浮提示词工具\n快速插入常用提示词到任意编辑器")
                            .title("关于 Q-Prompt")
                            .blocking_show();
                    }
                    "quit" => std::process::exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        window::toggle_quick_bar(tray.app_handle());
                    }
                })
                .build(app)?;

            log::info!("Q-Prompt started successfully");
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "quick-bar" {
                if let tauri::WindowEvent::Moved(pos) = event {
                    window::save_bar_position(window.app_handle(), pos.x as i32, pos.y as i32);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Q-Prompt");
}

// ── Shortcuts ──

fn register_shortcuts(app: &tauri::AppHandle, config: &store::Config) {
    let list: Vec<(&str, String)> = vec![
        ("toggle_bar", config.shortcuts.get("toggle_bar").cloned().unwrap_or("Alt+Q".into())),
        ("insert_1", config.shortcuts.get("insert_1").cloned().unwrap_or("Alt+1".into())),
        ("insert_2", config.shortcuts.get("insert_2").cloned().unwrap_or("Alt+2".into())),
        ("insert_3", config.shortcuts.get("insert_3").cloned().unwrap_or("Alt+3".into())),
        ("insert_4", config.shortcuts.get("insert_4").cloned().unwrap_or("Alt+4".into())),
        ("insert_5", config.shortcuts.get("insert_5").cloned().unwrap_or("Alt+5".into())),
        ("insert_6", config.shortcuts.get("insert_6").cloned().unwrap_or("Alt+6".into())),
        ("insert_7", config.shortcuts.get("insert_7").cloned().unwrap_or("Alt+7".into())),
        ("insert_8", config.shortcuts.get("insert_8").cloned().unwrap_or("Alt+8".into())),
        ("quick_save", config.shortcuts.get("quick_save").cloned().unwrap_or("Alt+S".into())),
    ];

    for (action, shortcut_str) in &list {
        if let Ok(shortcut) = shortcut_str.parse::<tauri_plugin_global_shortcut::Shortcut>() {
            let action_s = action.to_string();
            match app.global_shortcut().on_shortcut(shortcut, move |a, _sc, ev| {
                if ev.state == ShortcutState::Pressed {
                    handle_shortcut(a, &action_s);
                }
            }) {
                Ok(_) => log::info!("Registered: {} -> {}", action, shortcut_str),
                Err(e) => log::warn!("Shortcut failed {} ({}): {:?}", action, shortcut_str, e),
            }
        }
    }
}

fn handle_shortcut(app: &tauri::AppHandle, action: &str) {
    match action {
        "toggle_bar" => window::toggle_quick_bar(app),
        s if s.starts_with("insert_") => {
            let idx: usize = s.strip_prefix("insert_").unwrap().parse().unwrap();
            let _ = app.emit("shortcut-insert", idx - 1);
        }
        "quick_save" => {
            let _ = app.emit("shortcut-quick-save", ());
        }
        _ => {}
    }
}

// ── Public handler ──

pub fn lib_shortcut_handler(app: &tauri::AppHandle, action: &str) {
    handle_shortcut(app, action);
}

// ── Single instance ──

fn check_single_instance() -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        let name: Vec<u16> = OsStr::new("Q-Prompt-SingleInstance-9a4b")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            extern "system" {
                fn CreateMutexW(a: *mut std::ffi::c_void, b: i32, n: *const u16) -> isize;
                fn GetLastError() -> u32;
            }
            CreateMutexW(std::ptr::null_mut(), 0, name.as_ptr());
            GetLastError() != 183
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        true
    }
}

fn set_launch_on_startup(enable: bool) {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        let sub_key: Vec<u16> = OsStr::new("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let val_name: Vec<u16> = OsStr::new("Q-Prompt")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            extern "system" {
                fn RegOpenKeyExW(h: isize, s: *const u16, o: u32, a: u32, r: *mut isize) -> i32;
                fn RegSetValueExW(h: isize, v: *const u16, r: u32, t: u32, d: *const u8, l: u32) -> i32;
                fn RegDeleteValueW(h: isize, v: *const u16) -> i32;
                fn RegCloseKey(h: isize) -> i32;
            }
            let mut hkey: isize = 0;
            if RegOpenKeyExW(-2147483647, sub_key.as_ptr(), 0, 0x0002, &mut hkey) == 0 {
                if enable {
                    let exe = std::env::current_exe().unwrap_or_default();
                    let exe_s = exe.to_string_lossy();
                    let data: Vec<u16> = OsStr::new(&*exe_s)
                        .encode_wide()
                        .chain(std::iter::once(0))
                        .collect();
                    RegSetValueExW(hkey, val_name.as_ptr(), 0, 1, data.as_ptr() as *const u8, (data.len() * 2) as u32);
                } else {
                    RegDeleteValueW(hkey, val_name.as_ptr());
                }
                RegCloseKey(hkey);
            }
        }
    }
}
