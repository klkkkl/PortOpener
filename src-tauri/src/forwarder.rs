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
pub struct ForwardRule {
    pub id: String,
    pub name: String,
    pub protocol: Protocol,
    pub listen_addr: String,
    pub target_addr: String,
    pub enabled: bool,
    pub status: RuleStatus,
}

impl ForwardRule {
    pub fn new(name: String, protocol: Protocol, listen_addr: String, target_addr: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            protocol,
            listen_addr,
            target_addr,
            enabled: false,
            status: RuleStatus::Stopped,
        }
    }
}

struct RunningTask {
    cancel: CancellationToken,
}

pub struct ForwarderState {
    pub rules: HashMap<String, ForwardRule>,
    tasks: HashMap<String, RunningTask>,
    config_path: Option<PathBuf>,
}

impl ForwarderState {
    pub fn with_config_path(config_path: PathBuf) -> Self {
        Self {
            rules: HashMap::new(),
            tasks: HashMap::new(),
            config_path: Some(config_path),
        }
    }

    pub fn is_running(&self, id: &str) -> bool {
        self.tasks.contains_key(id)
    }

    pub fn save_rules(&self) -> Result<(), String> {
        if let Some(path) = &self.config_path {
            let rules_vec: Vec<_> = self.rules.values().cloned().collect();
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
                for mut rule in rules_vec {
                    // Reset runtime state on load
                    rule.enabled = false;
                    rule.status = RuleStatus::Stopped;
                    self.rules.insert(rule.id.clone(), rule);
                }
            }
        }
        Ok(())
    }
}

pub type SharedState = Arc<Mutex<ForwarderState>>;

pub async fn start_rule(state: SharedState, id: String) -> Result<(), String> {
    let (protocol, listen_addr, target_addr) = {
        let s = state.lock().await;
        let rule = s.rules.get(&id).ok_or("Rule not found")?;
        if s.tasks.contains_key(&id) {
            return Err("Already running".into());
        }
        (rule.protocol.clone(), rule.listen_addr.clone(), rule.target_addr.clone())
    };

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let state_clone = state.clone();
    let id_clone = id.clone();

    let task = match protocol {
        Protocol::Tcp => {
            tokio::spawn(async move {
                let result = run_tcp_forward(listen_addr, target_addr, cancel_clone).await;
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
                let result = run_udp_forward(listen_addr, target_addr, cancel_clone).await;
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

    // Drop the JoinHandle — task runs independently
    drop(task);

    let mut s = state.lock().await;
    s.tasks.insert(id.clone(), RunningTask { cancel });
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

async fn run_tcp_forward(
    listen_addr: String,
    target_addr: String,
    cancel: CancellationToken,
) -> Result<(), String> {
    let listener = TcpListener::bind(&listen_addr)
        .await
        .map_err(|e| format!("Bind failed: {e}"))?;

    println!("[TCP] Listening on {}", listen_addr);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                println!("[TCP] Stopped listening on {}", listen_addr);
                break;
            }
            result = listener.accept() => {
                match result {
                    Ok((client, client_addr)) => {
                        println!("[TCP] New connection from {} -> {}", client_addr, target_addr);
                        let target = target_addr.clone();
                        let cancel_conn = cancel.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_tcp_connection(client, target, cancel_conn).await {
                                eprintln!("[TCP] Connection error: {}", e);
                            }
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
) -> io::Result<()> {
    let target = TcpStream::connect(&target_addr).await?;

    let (mut cr, mut cw) = io::split(client);
    let (mut tr, mut tw) = io::split(target);

    tokio::select! {
        _ = cancel.cancelled() => {}
        _ = async {
            tokio::join!(
                io::copy(&mut cr, &mut tw),
                io::copy(&mut tr, &mut cw),
            )
        } => {}
    }
    Ok(())
}

async fn run_udp_forward(
    listen_addr: String,
    target_addr: String,
    cancel: CancellationToken,
) -> Result<(), String> {
    let socket = Arc::new(
        UdpSocket::bind(&listen_addr)
            .await
            .map_err(|e| format!("Bind failed: {e}"))?,
    );

    println!("[UDP] Listening on {}", listen_addr);

    // client_addr -> (outbound socket, last activity time)
    let sessions: Arc<Mutex<HashMap<std::net::SocketAddr, (Arc<UdpSocket>, Instant)>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // Spawn cleanup task for idle sessions (60s timeout)
    let sessions_cleanup = sessions.clone();
    let cancel_cleanup = cancel.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel_cleanup.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                    let mut map = sessions_cleanup.lock().await;
                    let now = Instant::now();
                    map.retain(|addr, (_, last_activity)| {
                        let idle = now.duration_since(*last_activity).as_secs();
                        if idle > 60 {
                            println!("[UDP] Cleaning up idle session: {}", addr);
                            false
                        } else {
                            true
                        }
                    });
                }
            }
        }
    });

    let mut buf = vec![0u8; 65535];

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                println!("[UDP] Stopped listening on {}", listen_addr);
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

                        tokio::spawn(async move {
                            if let Err(e) = handle_udp_packet(
                                data, client_addr, target, inbound, sessions
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

            // Spawn reverse path: target -> client
            let s_clone = s.clone();
            let inbound_clone = inbound.clone();
            let sessions_clone = sessions.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 65535];
                loop {
                    match s_clone.recv(&mut buf).await {
                        Ok(n) => {
                            // Update last activity time
                            if let Some((_, last_activity)) = sessions_clone.lock().await.get_mut(&client_addr) {
                                *last_activity = Instant::now();
                            }
                            let _ = inbound_clone.send_to(&buf[..n], client_addr).await;
                        }
                        Err(_) => break,
                    }
                }
            });

            s
        }
    };

    outbound.send(&data).await?;
    Ok(())
}
