//! Simple example demonstrating circle-socket for CLI background process management
//! Shows the start/stop pattern for managing long-running commands

use circle_socket::{SocketClient, SocketConfig, SocketPayload, SocketResponse, SocketResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// Request/Response types
#[derive(Debug, Serialize, Deserialize)]
struct ProcessRequest {
    pub command: String,  // "start", "stop", or "list"
    pub name: String,     // process name
    pub payload: String,  // command to run (for start) or empty
}

#[derive(Debug, Serialize, Deserialize)]
struct ProcessResponse {
    pub success: bool,
    pub message: String,
    pub processes: Option<HashMap<String, String>>, // name -> status
}

// In-memory process store
struct ProcessStore {
    processes: Arc<Mutex<HashMap<String, String>>>,
}

impl ProcessStore {
    fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn handle_request(&self, req: ProcessRequest) -> ProcessResponse {
        let mut processes = self.processes.lock().unwrap();

        match req.command.as_str() {
            "start" => {
                if processes.contains_key(&req.name) {
                    ProcessResponse {
                        success: false,
                        message: format!("Process '{}' already running", req.name),
                        processes: None,
                    }
                } else {
                    processes.insert(req.name.clone(), req.payload.clone());
                    println!("[Daemon] Started process: {} -> {}", req.name, req.payload);
                    ProcessResponse {
                        success: true,
                        message: format!("Process '{}' started", req.name),
                        processes: None,
                    }
                }
            }
            "stop" => {
                match processes.remove(&req.name) {
                    Some(_) => {
                        println!("[Daemon] Stopped process: {}", req.name);
                        ProcessResponse {
                            success: true,
                            message: format!("Process '{}' stopped", req.name),
                            processes: None,
                        }
                    }
                    None => ProcessResponse {
                        success: false,
                        message: format!("Process '{}' not found", req.name),
                        processes: None,
                    },
                }
            }
            "list" => {
                let list = processes.clone();
                ProcessResponse {
                    success: true,
                    message: format!("{} running processes", list.len()),
                    processes: Some(list),
                }
            }
            _ => ProcessResponse {
                success: false,
                message: format!("Unknown command: {}", req.command),
                processes: None,
            },
        }
    }
}

// Run daemon in background
async fn run_daemon(socket_path: &PathBuf) -> SocketResult<()> {
    println!("Starting daemon at {:?}", socket_path);

    let store = Arc::new(ProcessStore::new());
    let config = SocketConfig::from(socket_path);

    // Use string payloads for simplicity
    let server = circle_socket::SocketServer::<String, String>::new(config.clone());

    // Register handler for all requests
    let store_clone = Arc::clone(&store);
    server.register_handler("request", move |payload| {
        if let Ok(req) = serde_json::from_str::<ProcessRequest>(&payload.data) {
            let response = store_clone.handle_request(req);
            let response_str = serde_json::to_string(&response).unwrap();
            Ok(SocketResponse::success(payload.request_id, response_str))
        } else {
            Ok(SocketResponse::error(payload.request_id, "Invalid request format"))
        }
    }).await;

    println!("Daemon ready. Use another terminal to send commands.");
    server.run().await
}

// Send command to daemon
async fn send_command(socket_path: &PathBuf, command: &str, name: &str, payload: &str) -> SocketResult<()> {
    let client = SocketClient::new(SocketConfig::from(socket_path));

    let req = ProcessRequest {
        command: command.to_string(),
        name: name.to_string(),
        payload: payload.to_string(),
    };

    let payload = SocketPayload::new("request", serde_json::to_string(&req)?);
    let response = client.send_request::<String, String>(payload).await?;

    if response.success {
        if let Ok(resp) = serde_json::from_str::<ProcessResponse>(&response.data.unwrap()) {
            if resp.success {
                println!("✓ {}", resp.message);
                if let Some(processes) = resp.processes {
                    if processes.is_empty() {
                        println!("  No running processes");
                    } else {
                        println!("  Running processes:");
                        for (name, cmd) in processes {
                            println!("    - {}: {}", name, cmd);
                        }
                    }
                }
            } else {
                println!("✗ {}", resp.message);
            }
        }
    } else {
        println!("✗ Error: {}", response.error.unwrap());
    }

    Ok(())
}

#[tokio::main]
async fn main() -> SocketResult<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let socket_path = PathBuf::from("/tmp/circle_example.sock");

    if args.is_empty() {
        println!("Circle Socket - Simple CLI Process Manager");
        println!();
        println!("Usage:");
        println!("  cargo run --example socket_example -- daemon");
        println!("  cargo run --example socket_example -- start <name> <command>");
        println!("  cargo run --example socket_example -- stop <name>");
        println!("  cargo run --example socket_example -- list");
        println!();
        println!("Example:");
        println!("  Terminal 1: cargo run --example socket_example -- daemon");
        println!("  Terminal 2: cargo run --example socket_example -- start web 'python -m http.server 8080'");
        println!("  Terminal 2: cargo run --example socket_example -- list");
        println!("  Terminal 2: cargo run --example socket_example -- stop web");
        return Ok(());
    }

    match args[0].as_str() {
        "daemon" => run_daemon(&socket_path).await,
        "start" => {
            if args.len() < 3 {
                eprintln!("Usage: start <name> <command>");
                return Ok(());
            }
            send_command(&socket_path, "start", &args[1], &args[2]).await
        }
        "stop" => {
            if args.len() < 2 {
                eprintln!("Usage: stop <name>");
                return Ok(());
            }
            send_command(&socket_path, "stop", &args[1], "").await
        }
        "list" => send_command(&socket_path, "list", "", "").await,
        _ => {
            eprintln!("Unknown command: {}", args[0]);
            Ok(())
        }
    }
}