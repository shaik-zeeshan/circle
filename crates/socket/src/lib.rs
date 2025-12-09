use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Errors that can occur during socket operations
#[derive(Error, Debug)]
pub enum SocketError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Socket already exists at path: {0}")]
    AlreadyExists(PathBuf),
    #[error("Connection timed out")]
    ConnectionTimeout,
    #[error("Request handler not found for command: {0}")]
    HandlerNotFound(String),
    #[error("Invalid request format")]
    InvalidRequest,
}

/// Result type for socket operations
pub type SocketResult<T> = Result<T, SocketError>;

/// Generic socket payload that can be used for any command communication
#[derive(Debug, Clone)]
pub struct SocketPayload<T, R> {
    /// Unique identifier for this request
    pub request_id: String,
    /// Command type (e.g., "start", "stop", "status")
    pub command: String,
    /// The actual payload data
    pub data: T,
    /// Expected response type marker
    _phantom: std::marker::PhantomData<R>,
}

impl<T, R> serde::Serialize for SocketPayload<T, R>
where
    T: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SocketPayload", 3)?;
        state.serialize_field("request_id", &self.request_id)?;
        state.serialize_field("command", &self.command)?;
        state.serialize_field("data", &self.data)?;
        state.end()
    }
}

impl<'de, T, R> serde::Deserialize<'de> for SocketPayload<T, R>
where
    T: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct SocketPayloadData<T> {
            request_id: String,
            command: String,
            data: T,
        }

        let data = SocketPayloadData::<T>::deserialize(deserializer)?;
        Ok(SocketPayload {
            request_id: data.request_id,
            command: data.command,
            data: data.data,
            _phantom: std::marker::PhantomData,
        })
    }
}

impl<T, R> SocketPayload<T, R> {
    /// Create a new socket payload
    pub fn new(command: impl Into<String>, data: T) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            command: command.into(),
            data,
            _phantom: std::marker::PhantomData,
        }
    }
}

/// Response sent back through the socket
#[derive(Debug, Clone)]
pub struct SocketResponse<R> {
    /// Corresponds to the original request ID
    pub request_id: String,
    /// Whether the operation was successful
    pub success: bool,
    /// The response data
    pub data: Option<R>,
    /// Error message if any
    pub error: Option<String>,
}

impl<R> serde::Serialize for SocketResponse<R>
where
    R: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SocketResponse", 4)?;
        state.serialize_field("request_id", &self.request_id)?;
        state.serialize_field("success", &self.success)?;
        state.serialize_field("data", &self.data)?;
        state.serialize_field("error", &self.error)?;
        state.end()
    }
}

impl<'de, R> serde::Deserialize<'de> for SocketResponse<R>
where
    R: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct SocketResponseData<R> {
            request_id: String,
            success: bool,
            data: Option<R>,
            error: Option<String>,
        }

        let data = SocketResponseData::<R>::deserialize(deserializer)?;
        Ok(SocketResponse {
            request_id: data.request_id,
            success: data.success,
            data: data.data,
            error: data.error,
        })
    }
}

impl<R> SocketResponse<R> {
    /// Create a successful response
    pub fn success(request_id: impl Into<String>, data: R) -> Self {
        Self {
            request_id: request_id.into(),
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(request_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            success: false,
            data: None,
            error: Some(error.into()),
        }
    }
}

/// Configuration for socket connections
#[derive(Debug, Clone)]
pub struct SocketConfig {
    /// Path to the Unix socket file
    pub socket_path: PathBuf,
    /// Timeout for connections in seconds
    pub timeout: u64,
}

impl Default for SocketConfig {
    fn default() -> Self {
        Self {
            socket_path: PathBuf::from("/tmp/circle.sock"),
            timeout: 30,
        }
    }
}

impl<P> From<P> for SocketConfig where P: AsRef<Path> {
    fn from(path: P) -> Self {
        Self {
            socket_path: path.as_ref().to_path_buf(),
            timeout: 30,
        }
    }
}

/// A handler function for processing socket requests
pub type RequestHandler<T, R> = Arc<dyn Fn(SocketPayload<T, R>) -> SocketResult<SocketResponse<R>> + Send + Sync>;

/// Unix socket server for handling incoming requests
pub struct SocketServer<T, R> {
    config: SocketConfig,
    handlers: Arc<RwLock<std::collections::HashMap<String, RequestHandler<T, R>>>>,
}

impl<T, R> SocketServer<T, R>
where
    T: Send + Sync + 'static + serde::Serialize + for<'de> serde::Deserialize<'de>,
    R: Send + Sync + 'static + serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug,
{
    /// Create a new socket server
    pub fn new(config: SocketConfig) -> Self {
        Self {
            config,
            handlers: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Register a handler for a specific command
    pub async fn register_handler<F>(&self, command: impl Into<String>, handler: F)
    where
        F: Fn(SocketPayload<T, R>) -> SocketResult<SocketResponse<R>> + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().await;
        handlers.insert(command.into(), Arc::new(handler));
    }

    /// Start the socket server
    pub async fn run(self) -> SocketResult<()> {
        let socket_path = &self.config.socket_path;

        // Remove existing socket file if it exists
        if socket_path.exists() {
            std::fs::remove_file(socket_path)?;
        }

        let listener = UnixListener::bind(socket_path)?;
        info!("Socket server listening on: {:?}", socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let handlers = Arc::clone(&self.handlers);
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, handlers).await {
                            error!("Error handling connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                }
            }
        }
    }

    async fn handle_connection(
        mut stream: UnixStream,
        handlers: Arc<RwLock<std::collections::HashMap<String, RequestHandler<T, R>>>>,
    ) -> SocketResult<()> {
        // Read the request
        let mut buffer = vec![0u8; 8192];
        let n = stream.read(&mut buffer).await?;
        if n == 0 {
            warn!("Empty connection received");
            return Ok(());
        }

        let request_str = String::from_utf8_lossy(&buffer[..n]);
        debug!("Received request: {}", request_str);

        // Parse the payload
        let payload: SocketPayload<T, R> = serde_json::from_str(&request_str)
            .map_err(|_| SocketError::InvalidRequest)?;

        // Store request_id before moving payload
        let request_id = payload.request_id.clone();
        let command = payload.command.clone();

        // Find and execute the handler
        let handlers = handlers.read().await;
        if let Some(handler) = handlers.get(&payload.command) {
            match handler(payload) {
                Ok(response) => {
                    let response_json = serde_json::to_string(&response)?;
                    stream.write_all(response_json.as_bytes()).await?;
                    debug!("Sent response for request ID: {}", response.request_id);
                }
                Err(e) => {
                    let error_response = SocketResponse::<R>::error(&request_id, e.to_string());
                    let response_json = serde_json::to_string(&error_response)?;
                    stream.write_all(response_json.as_bytes()).await?;
                    warn!("Error handling request: {}", e);
                }
            }
        } else {
            let error_response = SocketResponse::<R>::error(
                &request_id,
                format!("No handler for command: {}", command),
            );
            let response_json = serde_json::to_string(&error_response)?;
            stream.write_all(response_json.as_bytes()).await?;
        }

        Ok(())
    }
}

/// Unix socket client for sending requests
pub struct SocketClient {
    config: SocketConfig,
}

impl SocketClient {
    /// Create a new socket client
    pub fn new(config: SocketConfig) -> Self {
        Self { config }
    }

    /// Send a request and wait for response
    pub async fn send_request<T, R>(&self, payload: SocketPayload<T, R>) -> SocketResult<SocketResponse<R>>
    where
        T: serde::Serialize,
        R: for<'de> serde::Deserialize<'de> + std::fmt::Debug,
    {
        let mut stream = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout),
            UnixStream::connect(&self.config.socket_path),
        )
        .await
        .map_err(|_| SocketError::ConnectionTimeout)??;

        let request_json = serde_json::to_string(&payload)?;
        stream.write_all(request_json.as_bytes()).await?;
        stream.shutdown().await?;

        // Read response
        let mut buffer = vec![0u8; 8192];
        let n = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout),
            stream.read(&mut buffer),
        )
        .await
        .map_err(|_| SocketError::ConnectionTimeout)??;

        if n == 0 {
            return Err(SocketError::InvalidRequest);
        }

        let response_str = String::from_utf8_lossy(&buffer[..n]);
        let response: SocketResponse<R> = serde_json::from_str(&response_str)?;
        debug!("Received response: {:?}", response);

        Ok(response)
    }

    /// Send a request without waiting for response (fire and forget)
    pub async fn send_request_no_response<T>(&self, payload: SocketPayload<T, ()>) -> SocketResult<()>
    where
        T: serde::Serialize,
    {
        let mut stream = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout),
            UnixStream::connect(&self.config.socket_path),
        )
        .await
        .map_err(|_| SocketError::ConnectionTimeout)??;

        let request_json = serde_json::to_string(&payload)?;
        stream.write_all(request_json.as_bytes()).await?;
        stream.shutdown().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tokio::time::{sleep, Duration};

    #[derive(Debug, Serialize, Deserialize)]
    struct StartCommand {
        pub process_id: String,
        pub command: Vec<String>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct StartResponse {
        pub started: bool,
        pub pid: u32,
    }

    
    #[tokio::test]
    async fn test_socket_communication() {
        let socket_path = "/tmp/test_circle_socket.sock";
        let config = SocketConfig::from(socket_path);

        // Start server in background
        let server_config = config.clone();
        let server_handle = tokio::spawn(async move {
            let server = SocketServer::<StartCommand, StartResponse>::new(server_config);

            // Register start handler
            server.register_handler("start", |payload| {
                Ok(SocketResponse::success(payload.request_id, StartResponse {
                    started: true,
                    pid: 12345,
                }))
            }).await;

            // This would normally run forever, but we'll timeout for the test
            tokio::time::timeout(Duration::from_secs(1), server.run()).await
        });

        // Give server time to start
        sleep(Duration::from_millis(100)).await;

        // Send a request
        let client = SocketClient::new(config);
        let payload = SocketPayload::new("start", StartCommand {
            process_id: "test_process".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
        });

        let response = client.send_request::<StartCommand, StartResponse>(payload).await;
        assert!(response.is_ok());

        let resp = response.unwrap();
        assert!(resp.success);
        assert!(resp.data.is_some());
        assert_eq!(resp.data.unwrap().pid, 12345);

        server_handle.abort();

        // Clean up
        if Path::new(socket_path).exists() {
            std::fs::remove_file(socket_path).ok();
        }
    }
}