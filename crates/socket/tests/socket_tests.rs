use circle_socket::{SocketClient, SocketConfig, SocketPayload, SocketResponse, SocketServer};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::time::{sleep, Duration};

#[derive(Debug, Serialize, Deserialize)]
struct TestData {
    value: String,
    number: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct TestResponse {
    result: String,
    doubled: i32,
}

#[tokio::test]
async fn test_start_stop_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = PathBuf::from("/tmp/test_circle.sock");
    let config = SocketConfig::from(&socket_path);

    // Clean up any existing socket
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }

    // Start server in background
    let server_config = config.clone();
    let server_handle = tokio::spawn(async move {
        let server = SocketServer::<TestData, TestResponse>::new(server_config);

        // Register "start" handler
        server
            .register_handler("start", |payload| {
                Ok(SocketResponse::success(payload.request_id, TestResponse {
                    result: format!("Started with value: {}", payload.data.value),
                    doubled: payload.data.number * 2,
                }))
            })
            .await;

        // Register "stop" handler
        server
            .register_handler("stop", |payload| {
                Ok(SocketResponse::success(payload.request_id, TestResponse {
                    result: format!("Stopped with value: {}", payload.data.value),
                    doubled: payload.data.number * 2,
                }))
            })
            .await;

        // Run server for a limited time for testing
        tokio::time::timeout(Duration::from_secs(5), server.run()).await
    });

    // Give server time to start and create socket
    sleep(Duration::from_millis(100)).await;

    // Create client
    let client = SocketClient::new(config);

    // Test start command
    let start_payload = SocketPayload::new(
        "start",
        TestData {
            value: "my-process".to_string(),
            number: 42,
        },
    );

    let response = client.send_request::<TestData, TestResponse>(start_payload).await?;
    assert!(response.success);
    let data = response.data.unwrap();
    assert_eq!(data.result, "Started with value: my-process");
    assert_eq!(data.doubled, 84);

    // Test stop command
    let stop_payload = SocketPayload::new(
        "stop",
        TestData {
            value: "my-process".to_string(),
            number: 21,
        },
    );

    let response = client.send_request::<TestData, TestResponse>(stop_payload).await?;
    assert!(response.success);
    let data = response.data.unwrap();
    assert_eq!(data.result, "Stopped with value: my-process");
    assert_eq!(data.doubled, 42);

    // Abort server
    server_handle.abort();

    // Clean up
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }

    Ok(())
}