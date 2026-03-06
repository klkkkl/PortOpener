mod forwarder;

use forwarder::{ForwardRule, Protocol, SharedState};
use serde::Deserialize;
use tauri::State;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Deserialize)]
pub struct CreateRuleRequest {
    name: String,
    protocol: Protocol,
    listen_addr: String,
    target_addr: String,
}


#[tauri::command]
async fn list_rules(state: State<'_, SharedState>) -> Result<Vec<ForwardRule>, String> {
    let s = state.lock().await;
    Ok(s.rules.values().cloned().collect())
}

#[tauri::command]
async fn add_rule(
    state: State<'_, SharedState>,
    req: CreateRuleRequest,
) -> Result<ForwardRule, String> {
    let rule = ForwardRule::new(req.name, req.protocol, req.listen_addr, req.target_addr);
    let mut s = state.lock().await;
    s.rules.insert(rule.id.clone(), rule.clone());
    s.save_rules()?;
    Ok(rule)
}

#[tauri::command]
async fn remove_rule(state: State<'_, SharedState>, id: String) -> Result<(), String> {
    let mut s = state.lock().await;
    if s.is_running(&id) {
        return Err("Stop the rule before removing it".into());
    }
    s.rules.remove(&id).ok_or("Rule not found")?;
    s.save_rules()?;
    Ok(())
}

#[tauri::command]
async fn start_rule(state: State<'_, SharedState>, id: String) -> Result<(), String> {
    forwarder::start_rule(state.inner().clone(), id).await
}

#[tauri::command]
async fn stop_rule(state: State<'_, SharedState>, id: String) -> Result<(), String> {
    forwarder::stop_rule(state.inner().clone(), id).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize state with config path
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap())
        .join("portopener");

    std::fs::create_dir_all(&config_dir).ok();
    let config_path = config_dir.join("rules.json");

    let mut state = forwarder::ForwarderState::with_config_path(config_path);
    if let Err(e) = state.load_rules() {
        eprintln!("Failed to load rules: {}", e);
    }

    let shared_state = Arc::new(Mutex::new(state));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(shared_state)
        .invoke_handler(tauri::generate_handler![
            list_rules,
            add_rule,
            remove_rule,
            start_rule,
            stop_rule,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
