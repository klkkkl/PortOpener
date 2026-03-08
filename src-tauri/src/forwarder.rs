use std::collections::HashMap;
use std::sync::Arc;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::io;
use tokio_util::sync::CancellationToken;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::sync::atomic::{AtomicU64, AtomicI64, Ordering};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RuleStatus {
    Stopped,
    Running,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleStats {
    pub bytes_up: u64,
    pub bytes_down: u64,
    pub connections: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardRule {
    pub id: String,
    #[serde(default)]
    pub order: u64,
    pub name: String,
    pub protocol: Protocol,
    pub listen_addr: String,
    pub target_addr: String,
    pub enabled: bool,
    pub status: RuleStatus,
    #[serde(default)]
    pub stats: RuleStats,
}

impl Default for RuleStats {
    fn default() -> Self {
        Self { bytes_up: 0, bytes_down: 0, connections: 0 }
    }
}

impl ForwardRule {
    pub fn new(order: u64, name: String, protocol: Protocol, listen_addr: String, target_addr: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            order,
            name,
            protocol,
            listen_addr,
            target_addr,
            enabled: false,
            status: RuleStatus::Stopped,
            stats: RuleStats::default(),
        }
    }
}

// Shared atomic counters per rule (not persisted, reset on restart)
pub struct RuleCounters {
    pub bytes_up: Arc<AtomicU64>,
    pub bytes_down: Arc<AtomicU64>,
    pub connections: Arc<AtomicI64>,
}

impl RuleCounters {
    fn new() -> Self {
        Self {
            bytes_up: Arc::new(AtomicU64::new(0)),
            bytes_down: Arc::new(AtomicU64::new(0)),
            connections: Arc::new(AtomicI64::new(0)),
        }
    }
}

struct RunningTask {
    cancel: CancellationToken,
    counters: Arc<RuleCounters>,
}

pub struct ForwarderState {
    pub rules: HashMap<String, ForwardRule>,
    tasks: HashMap<String, RunningTask>,
    pub config_path: Option<PathBuf>,
    pub log_path: Option<PathBuf>,
    next_order: u64,
}

impl ForwarderState {
    pub fn with_config_path(config_path: PathBuf) -> Self {
        let log_path = config_path.parent().map(|p| p.join("portopener.log"));
        Self {
            rules: HashMap::new(),
            tasks: HashMap::new(),
            config_path: Some(config_path),
            log_path,
            next_order: 0,
        }
    }

    pub fn next_order(&mut self) -> u64 {
        let o = self.next_order;
        self.next_order += 1;
        o
    }

    pub fn is_running(&self, id: &str) -> bool {
        self.tasks.contains_key(id)
    }

    pub fn get_live_stats(&self, id: &str) -> Option<RuleStats> {
        self.tasks.get(id).map(|t| RuleStats {
            bytes_up: t.counters.bytes_up.load(Ordering::Relaxed),
            bytes_down: t.counters.bytes_down.load(Ordering::Relaxed),
            connections: t.counters.connections.load(Ordering::Relaxed).max(0) as i64,
        })
    }

    pub fn save_rules(&self) -> Result<(), String> {
        if let Some(path) = &self.config_path {
            // Save only persistent fields (strip runtime stats)
            let rules_vec: Vec<_> = self.rules.values().map(|r| {
                let mut r = r.clone();
                r.stats = RuleStats::default();
                r
            }).collect();
            let json = serde_json::to_string_pretty(&rules_vec)
                .map_err(|e| format!("Serialize error: {e}"))?;
            std::fs::write(path, json)
                .map_err(|e| format!("Write error: {e}"))?;
        }
        Ok(())
    }

    pub fn load_rules(&mut self) -> Result<(), String> {
        if let Some(path) = &self.config_path {
            if path.exists() {
                let json = std::fs::read_to_string(path)
                    .map_err(|e| format!("Read error: {e}"))?;
                let rules_vec: Vec<ForwardRule> = serde_json::from_str(&json)
                    .map_err(|e| format!("Deserialize error: {e}"))?;
                self.rules.clear();
                let mut max_order = 0u64;
                for mut rule in rules_vec {
                    rule.enabled = false;
                    rule.status = RuleStatus::Stopped;
                    rule.stats = RuleStats::default();
                    if rule.order >= max_order {
                        max_order = rule.order + 1;
                    }
                    self.rules.insert(rule.id.clone(), rule);
                }
                self.next_order = max_order;
            }
        }
        Ok(())
    }

    pub fn read_logs(&self, limit: usize) -> Vec<String> {
        if let Some(path) = &self.log_path {
            if let Ok(content) = std::fs::read_to_string(path) {
                let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                let start = lines.len().saturating_sub(limit);
                return lines[start..].to_vec();
            }
        }
        vec![]
    }
}

pub type SharedState = Arc<Mutex<ForwarderState>>;

pub async fn start_rule(state: SharedState, id: String) -> Result<(), String> {
    let (protocol, listen_addr, target_addr, log_path) = {
        let s = state.lock().await;
        let rule = s.rules.get(&id).ok_or("Rule not found")?;
        if s.tasks.contains_key(&id) {
            return Err("Already running".into());
        }
        (rule.protocol.clone(), rule.listen_addr.clone(), rule.target_addr.clone(), s.log_path.clone())
    };

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let state_clone = state.clone();
    let id_clone = id.clone();

    let counters = Arc::new(RuleCounters::new());
    let counters_clone = counters.bytes_up.clone();
    let counters_down = counters.bytes_down.clone();
    let counters_conn = counters.connections.clone();

    let task = match protocol {
        Protocol::Tcp => {
            tokio::spawn(async move {
                let result = run_tcp_forward(
                    listen_addr, target_addr, cancel_clone,
                    counters_clone, counters_down, counters_conn, log_path,
                ).await;
                let mut s = state_clone.lock().await;
                s.tasks.remove(&id_clone);
                if let Some(rule) = s.rules.get_mut(&id_clone) {
                    rule.enabled = false;
                    rule.status = match result {
                        Ok(_) => RuleStatus::Stopped,
                        Err(e) => RuleStatus::Error(e),
                    };
                }
            })
        }
        Protocol::Udp => {
            tokio::spawn(async move {
                let result = run_udp_forward(
                    listen_addr, target_addr, cancel_clone,
                    counters_clone, counters_down, counters_conn, log_path,
                ).await;
                let mut s = state_clone.lock().await;
                s.tasks.remove(&id_clone);
                if let Some(rule) = s.rules.get_mut(&id_clone) {
                    rule.enabled = false;
                    rule.status = match result {
                        Ok(_) => RuleStatus::Stopped,
                        Err(e) => RuleStatus::Error(e),
                    };
                }
            })
        }
    };

    drop(task);

    let mut s = state.lock().await;
    s.tasks.insert(id.clone(), RunningTask { cancel, counters });
    if let Some(rule) = s.rules.get_mut(&id) {
        rule.enabled = true;
        rule.status = RuleStatus::Running;
    }

    Ok(())
}

pub async fn stop_rule(state: SharedState, id: String) -> Result<(), String> {
    let mut s = state.lock().await;
    let task = s.tasks.remove(&id).ok_or("Not running")?;
    task.cancel.cancel();
    if let Some(rule) = s.rules.get_mut(&id) {
        rule.enabled = false;
        rule.status = RuleStatus::Stopped;
    }
    Ok(())
}

fn log_line(log_path: &Option<PathBuf>, msg: &str) {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("[{}] {}", now, msg);
    if let Some(path) = log_path {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(f, "{}", line);
        }
    }
}

fn hex_dump(data: &[u8]) -> String {
    data.chunks(16).map(|row| {
        let hex: Vec<String> = row.iter().map(|b| format!("{:02x}", b)).collect();
        let ascii: String = row.iter().map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' }).collect();
        format!("  {:47}  |{}|", hex.join(" "), ascii)
    }).collect::<Vec<_>>().join("\n")
}

async fn run_tcp_forward(
    listen_addr: String,
    target_addr: String,
    cancel: CancellationToken,
    bytes_up: Arc<AtomicU64>,
    bytes_down: Arc<AtomicU64>,
    connections: Arc<AtomicI64>,
    log_path: Option<PathBuf>,
) -> Result<(), String> {
    let listener = TcpListener::bind(&listen_addr)
        .await
        .map_err(|e| format!("Bind failed: {e}"))?;

    log_line(&log_path, &format!("[TCP] Listening on {}", listen_addr));

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                log_line(&log_path, &format!("[TCP] Stopped {}", listen_addr));
                break;
            }
            result = listener.accept() => {
                match result {
                    Ok((client, client_addr)) => {
                        log_line(&log_path, &format!("[TCP] {} -> {}", client_addr, target_addr));
                        let target = target_addr.clone();
                        let cancel_conn = cancel.clone();
                        let up = bytes_up.clone();
                        let down = bytes_down.clone();
                        let conn = connections.clone();
                        let lp = log_path.clone();
                        tokio::spawn(async move {
                            conn.fetch_add(1, Ordering::Relaxed);
                            if let Err(e) = handle_tcp_connection(client, target, cancel_conn, up, down, lp.clone(), client_addr).await {
                                log_line(&lp, &format!("[TCP] Connection error: {}", e));
                            }
                            conn.fetch_sub(1, Ordering::Relaxed);
                        });
                    }
                    Err(e) => return Err(format!("Accept error: {e}")),
                }
            }
        }
    }
    Ok(())
}

async fn handle_tcp_connection(
    client: TcpStream,
    target_addr: String,
    cancel: CancellationToken,
    bytes_up: Arc<AtomicU64>,
    bytes_down: Arc<AtomicU64>,
    log_path: Option<PathBuf>,
    client_addr: std::net::SocketAddr,
) -> io::Result<()> {
    let target = TcpStream::connect(&target_addr).await?;

    let (mut cr, mut cw) = io::split(client);
    let (mut tr, mut tw) = io::split(target);

    tokio::select! {
        _ = cancel.cancelled() => {}
        _ = async {
            let up = bytes_up.clone();
            let down = bytes_down.clone();
            let lp_up = log_path.clone();
            let lp_down = log_path.clone();
            let ca = client_addr;
            let ta = target_addr.clone();
            tokio::join!(
                async {
                    let mut buf = vec![0u8; 8192];
                    loop {
                        match tokio::io::AsyncReadExt::read(&mut cr, &mut buf).await {
                            Ok(0) | Err(_) => break,
                            Ok(n) => {
                                up.fetch_add(n as u64, Ordering::Relaxed);
                                log_line(&lp_up, &format!("[TCP] DATA {} -> {} {} bytes\n{}", ca, ta, n, hex_dump(&buf[..n])));
                                if tokio::io::AsyncWriteExt::write_all(&mut tw, &buf[..n]).await.is_err() { break; }
                            }
                        }
                    }
                },
                async {
                    let mut buf = vec![0u8; 8192];
                    loop {
                        match tokio::io::AsyncReadExt::read(&mut tr, &mut buf).await {
                            Ok(0) | Err(_) => break,
                            Ok(n) => {
                                down.fetch_add(n as u64, Ordering::Relaxed);
                                log_line(&lp_down, &format!("[TCP] DATA {} <- {} {} bytes\n{}", ca, ta, n, hex_dump(&buf[..n])));
                                if tokio::io::AsyncWriteExt::write_all(&mut cw, &buf[..n]).await.is_err() { break; }
                            }
                        }
                    }
                },
            )
        } => {}
    }
    log_line(&log_path, &format!("[TCP] Closed {} -> {}", client_addr, target_addr));
    Ok(())
}

async fn run_udp_forward(
    listen_addr: String,
    target_addr: String,
    cancel: CancellationToken,
    bytes_up: Arc<AtomicU64>,
    bytes_down: Arc<AtomicU64>,
    connections: Arc<AtomicI64>,
    log_path: Option<PathBuf>,
) -> Result<(), String> {
    let socket = Arc::new(
        UdpSocket::bind(&listen_addr)
            .await
            .map_err(|e| format!("Bind failed: {e}"))?,
    );

    log_line(&log_path, &format!("[UDP] Listening on {}", listen_addr));

    let sessions: Arc<Mutex<HashMap<std::net::SocketAddr, (Arc<UdpSocket>, Instant)>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let sessions_cleanup = sessions.clone();
    let cancel_cleanup = cancel.clone();
    let conn_cleanup = connections.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel_cleanup.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                    let mut map = sessions_cleanup.lock().await;
                    let now = Instant::now();
                    let before = map.len() as i64;
                    map.retain(|_, (_, last_activity)| {
                        now.duration_since(*last_activity).as_secs() <= 60
                    });
                    let removed = before - map.len() as i64;
                    if removed > 0 {
                        conn_cleanup.fetch_sub(removed, Ordering::Relaxed);
                    }
                }
            }
        }
    });

    let mut buf = vec![0u8; 65535];

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                log_line(&log_path, &format!("[UDP] Stopped {}", listen_addr));
                break;
            }
            result = socket.recv_from(&mut buf) => {
                match result {
                    Err(e) => return Err(format!("Recv error: {e}")),
                    Ok((len, client_addr)) => {
                        let data = buf[..len].to_vec();
                        let target = target_addr.clone();
                        let inbound = socket.clone();
                        let sessions = sessions.clone();
                        let up = bytes_up.clone();
                        let down = bytes_down.clone();
                        let conn = connections.clone();
                        let lp = log_path.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_udp_packet(
                                data, client_addr, target, inbound, sessions, up, down, conn, lp,
                            ).await {
                                eprintln!("[UDP] Packet error: {}", e);
                            }
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

async fn handle_udp_packet(
    data: Vec<u8>,
    client_addr: std::net::SocketAddr,
    target_addr: String,
    inbound: Arc<UdpSocket>,
    sessions: Arc<Mutex<HashMap<std::net::SocketAddr, (Arc<UdpSocket>, Instant)>>>,
    bytes_up: Arc<AtomicU64>,
    bytes_down: Arc<AtomicU64>,
    connections: Arc<AtomicI64>,
    log_path: Option<PathBuf>,
) -> io::Result<()> {
    let outbound = {
        let mut map = sessions.lock().await;
        if let Some((s, last_activity)) = map.get_mut(&client_addr) {
            *last_activity = Instant::now();
            s.clone()
        } else {
            let s = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
            s.connect(&target_addr).await?;
            map.insert(client_addr, (s.clone(), Instant::now()));
            connections.fetch_add(1, Ordering::Relaxed);
            log_line(&log_path, &format!("[UDP] New session: {} -> {}", client_addr, target_addr));

            let s_clone = s.clone();
            let inbound_clone = inbound.clone();
            let sessions_clone = sessions.clone();
            let down = bytes_down.clone();
            let lp_down = log_path.clone();
            let ta_down = target_addr.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 65535];
                loop {
                    match s_clone.recv(&mut buf).await {
                        Ok(n) => {
                            if let Some((_, last_activity)) = sessions_clone.lock().await.get_mut(&client_addr) {
                                *last_activity = Instant::now();
                            }
                            down.fetch_add(n as u64, Ordering::Relaxed);
                            log_line(&lp_down, &format!("[UDP] DATA {} <- {} {} bytes\n{}", client_addr, ta_down, n, hex_dump(&buf[..n])));
                            let _ = inbound_clone.send_to(&buf[..n], client_addr).await;
                        }
                        Err(_) => break,
                    }
                }
            });

            s
        }
    };

    let n = data.len();
    bytes_up.fetch_add(n as u64, Ordering::Relaxed);
    log_line(&log_path, &format!("[UDP] DATA {} -> {} {} bytes\n{}", client_addr, target_addr, n, hex_dump(&data)));
    outbound.send(&data).await?;
    Ok(())
}
