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
    let mut rules: Vec<_> = s.rules.values().cloned().collect();
    rules.sort_by_key(|r| r.order);
    Ok(rules)
}

#[tauri::command]
async fn add_rule(
    state: State<'_, SharedState>,
    req: CreateRuleRequest,
) -> Result<ForwardRule, String> {
    let mut s = state.lock().await;
    let order = s.next_order();
    let rule = ForwardRule::new(order, req.name, req.protocol, req.listen_addr, req.target_addr);
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
        rule.id = uuid::Uuid::new_v4().to_string();
        rule.order = s.next_order();
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

#[tauri::command]
async fn get_log_path(state: State<'_, SharedState>) -> Result<String, String> {
    let s = state.lock().await;
    s.log_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or("Log path not set".into())
}

#[tauri::command]
async fn clear_logs(state: State<'_, SharedState>) -> Result<(), String> {
    let s = state.lock().await;
    if let Some(path) = &s.log_path {
        std::fs::write(path, "").map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct UpdateRuleRequest {
    id: String,
    name: String,
    protocol: Protocol,
    listen_addr: String,
    target_addr: String,
}

#[tauri::command]
async fn update_rule(
    state: State<'_, SharedState>,
    req: UpdateRuleRequest,
) -> Result<(), String> {
    let mut s = state.lock().await;
    if s.is_running(&req.id) {
        return Err("Stop the rule before editing it".into());
    }
    let rule = s.rules.get_mut(&req.id).ok_or("Rule not found")?;
    rule.name = req.name;
    rule.protocol = req.protocol;
    rule.listen_addr = req.listen_addr;
    rule.target_addr = req.target_addr;
    s.save_rules()?;
    Ok(())
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
            get_log_path,
            clear_logs,
            update_rule,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
