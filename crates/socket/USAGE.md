# Circle Socket Usage Guide

This guide explains how to use the `circle-socket` crate for Unix socket communication in CLI applications.

## Overview

The `circle-socket` crate provides a simple way to implement inter-process communication using Unix domain sockets. This is particularly useful for CLI applications that need to manage background processes or communicate between different command invocations.

## Key Features

- **Generic Payload System**: Type-safe request/response handling
- **Async/Await Support**: Built on Tokio for non-blocking operations
- **Command Routing**: Register multiple handlers for different commands
- **Error Handling**: Comprehensive error types
- **Timeout Protection**: Configurable timeouts for socket operations

## Quick Start

### 1. Basic Server Setup

```rust
use circle_socket::{SocketServer, SocketConfig, SocketPayload, SocketResponse, SocketResult};
use serde::{Deserialize, Serialize};

// Define your request and response types
#[derive(Debug, Serialize, Deserialize)]
struct MyCommand {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct MyResponse {
    pub reply: String,
}

// Create and run the socket server
#[tokio::main]
async fn main() -> SocketResult<()> {
    let config = SocketConfig::from("/tmp/myapp.sock");
    let server = SocketServer::<MyCommand, MyResponse>::new(config);

    // Register command handlers
    server.register_handler("echo", |payload| {
        Ok(SocketResponse::success(payload.request_id, MyResponse {
            reply: payload.data.message,
        }))
    }).await;

    // Run the server (this blocks)
    server.run().await
}
```

### 2. Client Communication

```rust
use circle_socket::{SocketClient, SocketConfig, SocketPayload};

// Create a client
let client = SocketClient::new(SocketConfig::from("/tmp/myapp.sock"));

// Send a request
let payload = SocketPayload::new("echo", MyCommand {
    message: "Hello, Socket!".to_string(),
});

let response = client.send_request(payload).await?;
if response.success {
    println!("Response: {:?}", response.data);
}
```

## Design Patterns

### 1. String-Based Command Routing

For maximum flexibility, you can use strings for payload data and handle serialization/deserialization in your handlers:

```rust
let server = SocketServer::<String, String>::new(config);

server.register_handler("start", |payload| {
    if let Ok(cmd) = serde_json::from_str::<StartCommand>(&payload.data) {
        // Handle start command
        let response = start_process(cmd);
        Ok(SocketResponse::success(payload.request_id,
            serde_json::to_string(&response).unwrap()))
    } else {
        Ok(SocketResponse::error(payload.request_id, "Invalid command"))
    }
}).await;
```

### 2. Type-Safe Generic Approach

For better type safety, you can define a generic payload system:

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Request {
    Start(StartCommand),
    Stop(StopCommand),
    List(ListCommand),
}

// Use Request as your generic payload type
```

## Best Practices

1. **Always cleanup socket files**: Remove socket files when your daemon exits
2. **Use unique socket paths**: Include your application name or user ID
3. **Handle connection errors gracefully**: Daemon might not be running
4. **Use timeouts**: Prevent clients from hanging indefinitely
5. **Log operations**: Debug socket communication issues

## Configuration

```rust
let config = SocketConfig {
    socket_path: PathBuf::from("/tmp/custom.sock"),
    timeout: 30, // seconds
};
```

## Error Handling

The library provides detailed error types:

```rust
match client.send_request(payload).await {
    Ok(response) => {
        if response.success {
            // Handle success
        } else {
            eprintln!("Server error: {}", response.error.unwrap());
        }
    }
    Err(SocketError::ConnectionTimeout) => {
        eprintln!("Connection timed out");
    }
    Err(SocketError::Io(e)) => {
        eprintln!("IO error: {}", e);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

## Testing

The crate includes comprehensive tests. Run them with:

```bash
cargo test -p circle-socket
```

## Integration in CLI Applications

Here's a typical pattern for CLI applications:

```rust
// main.rs
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("daemon") => run_daemon().await,
        Some("start") => start_process(&args[2], &args[3]).await,
        Some("stop") => stop_process(&args[2]).await,
        Some("status") => show_status().await,
        _ => show_help(),
    }
}
```

This pattern allows you to:
- Start a long-running daemon process
- Send commands to control the daemon
- Query the daemon's status
- Stop the daemon cleanly

The Unix socket acts as the communication channel between your CLI commands and the background daemon process.
