# Circle Socket

A Rust library for Unix socket communication in CLI applications, designed for inter-process communication between commands.

## Features

- **Generic Payload System**: Type-safe request/response handling with custom payload types
- **Async/Await Support**: Built on Tokio for non-blocking operations
- **Command Routing**: Register multiple handlers for different command types
- **Timeout Protection**: Configurable timeouts for socket operations
- **Error Handling**: Comprehensive error types with detailed error messages

## Use Case Example

This library is particularly useful for CLI applications that need to communicate between different command invocations. For example:

- Start a long-running background process with `mycli start --daemon`
- Check its status with `mycli status`
- Stop it with `mycli stop`

## Quick Start

### Server Side (Background Process)

```rust
use circle_socket::{SocketServer, SocketConfig, SocketPayload, SocketResponse, SocketResult};
use serde::{Deserialize, Serialize};

// Define your request and response types
#[derive(Debug, Serialize, Deserialize)]
struct StartCommand {
    process_id: String,
    command: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StartResponse {
    process_id: String,
    pid: u32,
    status: String,
}

// Create and run the socket server
#[tokio::main]
async fn main() -> SocketResult<()> {
    let config = SocketConfig::from("/tmp/myapp.sock");
    let server = SocketServer::<StartCommand, StartResponse>::new(config);

    // Register command handlers
    server.register_handler("start", |payload| {
        // Process the start command
        Ok(SocketResponse::success(payload.request_id, StartResponse {
            process_id: payload.data.process_id,
            pid: 12345,
            status: "running".to_string(),
        }))
    }).await;

    // Run the server (this blocks)
    server.run().await
}
```

### Client Side (CLI Commands)

```rust
use circle_socket::{SocketClient, SocketConfig, SocketPayload};

// Create a client
let client = SocketClient::new(SocketConfig::from("/tmp/myapp.sock"));

// Send a request
let payload = SocketPayload::new("start", StartCommand {
    process_id: "my-process".to_string(),
    command: vec!["myapp".to_string(), "--daemon".to_string()],
});

let response = client.send_request(payload).await?;
if response.success {
    println!("Process started: {:?}", response.data);
}
```

## Core Components

### SocketPayload<T, R>
A generic payload structure for sending requests:
- `T`: The request data type
- `R`: The response data type (phantom data marker)
- `request_id`: Unique UUID for tracking
- `command`: Command type string
- `data`: The actual payload data

### SocketResponse<R>
Response structure:
- `request_id`: Matches the original request
- `success`: Boolean indicating success/failure
- `data`: Response data (if successful)
- `error`: Error message (if failed)

### SocketServer<T, R>
Server for handling incoming socket connections:
- Register handlers for different commands
- Handles concurrent connections
- Type-safe request/response handling

### SocketClient
Client for sending requests:
- Send requests and wait for responses
- Send fire-and-forget messages
- Configurable timeouts

## Configuration

```rust
use circle_socket::SocketConfig;

let config = SocketConfig {
    socket_path: PathBuf::from("/tmp/custom.sock"),
    timeout: 30, // seconds
};
```

Or create from a path:
```rust
let config = SocketConfig::from("/tmp/myapp.sock");
```

## Running the Example

The `socket_example` demonstrates a complete use case with process management:

```bash
cd crates/circle-socket
cargo run --example socket_example
```

This example shows:
- Starting a daemon process
- Managing background processes via socket commands
- Starting processes with `start <name> <command>`
- Listing running processes with `list`
- Stopping processes with `stop <name>`

### Example Workflow

1. Terminal 1: Start the daemon
   ```bash
   cargo run --example socket_example -- daemon
   ```

2. Terminal 2: Manage processes
   ```bash
   # Start a process
   cargo run --example socket_example -- start web "python -m http.server 8080"

   # List processes
   cargo run --example socket_example -- list

   # Stop a process
   cargo run --example socket_example -- stop web
   ```

## Error Handling

The library provides comprehensive error handling through the `SocketError` enum:

- `Io`: I/O errors
- `Serialization`: JSON serialization errors
- `AlreadyExists`: Socket file already exists
- `ConnectionTimeout`: Connection timed out
- `HandlerNotFound`: No handler for the command
- `InvalidRequest`: Malformed request

## Testing

Run the test suite:

```bash
cargo test
```

The tests include basic socket communication tests with temporary socket files.