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
    let mut s = state.lock().await;
    // Inject live stats into each running rule
    let ids: Vec<String> = s.rules.keys().cloned().collect();
    for id in ids {
        if let Some(stats) = s.get_live_stats(&id) {
            if let Some(rule) = s.rules.get_mut(&id) {
                rule.stats = stats;
            }
        }
    }
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

#[tauri::command]
async fn export_rules(state: State<'_, SharedState>) -> Result<String, String> {
    let s = state.lock().await;
    let rules: Vec<_> = s.rules.values().cloned().collect();
    serde_json::to_string_pretty(&rules).map_err(|e| e.to_string())
}

#[tauri::command]
async fn import_rules(state: State<'_, SharedState>, json: String) -> Result<usize, String> {
    let incoming: Vec<ForwardRule> = serde_json::from_str(&json)
        .map_err(|e| format!("Invalid JSON: {e}"))?;
    let count = incoming.len();
    let mut s = state.lock().await;
    for mut rule in incoming {
        // Reset runtime state, generate new id to avoid conflicts
        rule.id = uuid::Uuid::new_v4().to_string();
        rule.enabled = false;
        rule.status = forwarder::RuleStatus::Stopped;
        rule.stats = forwarder::RuleStats::default();
        s.rules.insert(rule.id.clone(), rule);
    }
    s.save_rules()?;
    Ok(count)
}

#[tauri::command]
async fn get_logs(state: State<'_, SharedState>, limit: usize) -> Result<Vec<String>, String> {
    let s = state.lock().await;
    Ok(s.read_logs(limit))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
            export_rules,
            import_rules,
            get_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
