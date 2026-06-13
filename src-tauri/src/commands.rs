use crate::scoring;
use crate::store::{Config, PromptItem, PromptStore};
use enigo::KeyboardControllable;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub store: Mutex<PromptStore>,
    pub config: Mutex<Config>,
}

#[tauri::command]
pub fn get_prompts(state: tauri::State<'_, AppState>) -> Vec<PromptItem> {
    let store = state.store.lock().unwrap();
    store.load_prompts()
}

#[tauri::command]
pub fn get_prompts_sorted(state: tauri::State<'_, AppState>) -> Vec<PromptItem> {
    let store = state.store.lock().unwrap();
    let prompts = store.load_prompts();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    scoring::top_k(&prompts, 8, now)
}

#[tauri::command]
pub fn add_prompt(state: tauri::State<'_, AppState>, name: String, content: String) -> Result<PromptItem, String> {
    let store = state.store.lock().unwrap();
    let mut prompts = store.load_prompts();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let max_order = prompts.iter().map(|p| p.sort_order).max().unwrap_or(0);
    let item = PromptItem {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        content,
        use_count: 0,
        last_used: now,
        sort_order: max_order + 1,
    };
    prompts.push(item.clone());
    store.save_prompts(&prompts);
    log::info!("Added prompt: {}", item.name);
    Ok(item)
}

#[tauri::command]
pub fn update_prompt(state: tauri::State<'_, AppState>, id: String, name: String, content: String) -> Result<(), String> {
    let store = state.store.lock().unwrap();
    let mut prompts = store.load_prompts();
    if let Some(item) = prompts.iter_mut().find(|p| p.id == id) {
        item.name = name;
        item.content = content;
        store.save_prompts(&prompts);
        log::info!("Updated prompt: {}", id);
        Ok(())
    } else {
        Err("Prompt not found".into())
    }
}

#[tauri::command]
pub fn delete_prompt(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    let store = state.store.lock().unwrap();
    let mut prompts = store.load_prompts();
    if let Some(pos) = prompts.iter().position(|p| p.id == id) {
        let removed = prompts.remove(pos);
        store.save_prompts(&prompts);
        let mut backup = store.load_deleted_backup();
        backup.push(removed);
        store.save_deleted_backup(&backup);
        log::info!("Deleted prompt (backed up): {}", id);
        Ok(())
    } else {
        Err("Prompt not found".into())
    }
}

#[tauri::command]
pub fn move_prompt(state: tauri::State<'_, AppState>, id: String, new_order: u32) -> Result<(), String> {
    let store = state.store.lock().unwrap();
    let mut prompts = store.load_prompts();
    if let Some(item) = prompts.iter_mut().find(|p| p.id == id) {
        item.sort_order = new_order;
        store.save_prompts(&prompts);
        Ok(())
    } else {
        Err("Prompt not found".into())
    }
}

#[tauri::command]
pub fn reorder_prompts(state: tauri::State<'_, AppState>, ordered_ids: Vec<String>) -> Result<(), String> {
    let store = state.store.lock().unwrap();
    let mut prompts = store.load_prompts();
    for (i, id) in ordered_ids.iter().enumerate() {
        if let Some(item) = prompts.iter_mut().find(|p| p.id == *id) {
            item.sort_order = i as u32;
        }
    }
    store.save_prompts(&prompts);
    Ok(())
}

#[tauri::command]
pub fn export_prompts(state: tauri::State<'_, AppState>) -> String {
    let store = state.store.lock().unwrap();
    let prompts = store.load_prompts();
    serde_json::to_string_pretty(&prompts).unwrap_or_else(|_| "[]".into())
}

#[tauri::command]
pub fn import_prompts(state: tauri::State<'_, AppState>, json: String) -> Result<usize, String> {
    let items: Vec<PromptItem> = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let count = items.len();
    let store = state.store.lock().unwrap();
    let mut prompts = store.load_prompts();
    let max_order = prompts.iter().map(|p| p.sort_order).max().unwrap_or(0);
    for (i, mut item) in items.into_iter().enumerate() {
        if prompts.iter().any(|p| p.id == item.id) {
            item.id = uuid::Uuid::new_v4().to_string();
        }
        item.sort_order = max_order + i as u32 + 1;
        prompts.push(item);
    }
    store.save_prompts(&prompts);
    log::info!("Imported {} prompts", count);
    Ok(count)
}

#[tauri::command]
pub fn get_config(state: tauri::State<'_, AppState>) -> Config {
    let config = state.config.lock().unwrap();
    config.clone()
}

#[tauri::command]
pub fn update_config(state: tauri::State<'_, AppState>, config: Config) -> Result<(), String> {
    let store = state.store.lock().unwrap();
    store.save_config(&config);
    let mut current = state.config.lock().unwrap();
    *current = config;
    Ok(())
}

#[tauri::command]
pub fn get_shortcut(state: tauri::State<'_, AppState>, key: String) -> Result<String, String> {
    let config = state.config.lock().unwrap();
    config.shortcuts.get(&key).cloned().ok_or("Shortcut not found".into())
}

#[tauri::command]
pub fn update_shortcut(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    key: String,
    new_shortcut: String,
) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
    let store = state.store.lock().unwrap();
    let mut config = store.load_config();
    let old_shortcut = config.shortcuts.get(&key).cloned();
    config.shortcuts.insert(key.clone(), new_shortcut.clone());
    store.save_config(&config);

    if let Some(old) = old_shortcut {
        if let Ok(shortcut) = old.parse::<tauri_plugin_global_shortcut::Shortcut>() {
            let _ = app.global_shortcut().unregister(shortcut);
        }
    }
    if let Ok(shortcut) = new_shortcut.parse::<tauri_plugin_global_shortcut::Shortcut>() {
        let action = key.clone();
        app.global_shortcut()
            .on_shortcut(shortcut, move |a, _sc, ev| {
                if ev.state == ShortcutState::Pressed {
                    crate::lib_shortcut_handler(a, &action);
                }
            })
            .map_err(|e| e.to_string())?;
    }

    let mut current = state.config.lock().unwrap();
    *current = config;
    log::info!("Updated shortcut: {} -> {}", key, new_shortcut);
    Ok(())
}

// ── Insert (clipboard write + Ctrl+V) ──

#[tauri::command]
pub fn insert_text(app: tauri::AppHandle, text: String) -> Result<(), String> {
    // 1. Hide bar so focus returns to editor
    if let Some(window) = app.get_webview_window("quick-bar") {
        let _ = window.hide();
    }

    std::thread::sleep(std::time::Duration::from_millis(80));

    // 2. Write to clipboard
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(&text).map_err(|e| e.to_string())?;

    // 3. Ctrl+V
    std::thread::sleep(std::time::Duration::from_millis(15));
    let mut enigo = enigo::Enigo::new();
    enigo.key_down(enigo::Key::Control);
    enigo.key_click(enigo::Key::Layout('v'));
    enigo.key_up(enigo::Key::Control);

    // 4. Short wait then show bar
    std::thread::sleep(std::time::Duration::from_millis(60));
    if let Some(window) = app.get_webview_window("quick-bar") {
        let _ = window.show();
        crate::window::apply_noactivate(&window);
    }

    Ok(())
}

// ── Selection capture ──

#[tauri::command]
pub fn capture_selection(app: tauri::AppHandle) -> Result<String, String> {
    let saved = arboard::Clipboard::new()
        .ok()
        .and_then(|mut c| c.get_text().ok())
        .unwrap_or_default();

    if let Some(window) = app.get_webview_window("quick-bar") {
        let _ = window.hide();
    }

    std::thread::sleep(std::time::Duration::from_millis(80));

    let mut enigo = enigo::Enigo::new();
    enigo.key_down(enigo::Key::Control);
    enigo.key_click(enigo::Key::Layout('c'));
    enigo.key_up(enigo::Key::Control);

    std::thread::sleep(std::time::Duration::from_millis(80));

    let selection = arboard::Clipboard::new()
        .ok()
        .and_then(|mut c| c.get_text().ok())
        .unwrap_or_default();

    if let Ok(mut c) = arboard::Clipboard::new() {
        let _ = c.set_text(&saved);
    }

    if let Some(window) = app.get_webview_window("quick-bar") {
        let _ = window.show();
        crate::window::apply_noactivate(&window);
    }

    Ok(selection)
}

// ── Quick Save ──

#[tauri::command]
pub fn quick_save(state: tauri::State<'_, AppState>, name: String, content: String) -> Result<PromptItem, String> {
    add_prompt(state, name, content)
}

// ── File-based export/import (avoids needing plugin-fs) ──

#[tauri::command]
pub fn export_to_file(state: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    let store = state.store.lock().unwrap();
    let prompts = store.load_prompts();
    let json = serde_json::to_string_pretty(&prompts).unwrap_or_else(|_| "[]".into());
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_from_file(state: tauri::State<'_, AppState>, path: String) -> Result<usize, String> {
    let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    import_prompts(state, data)
}

// ── Record usage ──

#[tauri::command]
pub fn record_usage(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    let store = state.store.lock().unwrap();
    let mut prompts = store.load_prompts();
    if let Some(item) = prompts.iter_mut().find(|p| p.id == id) {
        item.use_count += 1;
        item.last_used = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        store.save_prompts(&prompts);
        Ok(())
    } else {
        Err("Prompt not found".into())
    }
}

#[tauri::command]
pub fn open_manager_window(app: tauri::AppHandle) {
    crate::window::create_manager(&app);
}
