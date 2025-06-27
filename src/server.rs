//! Development server for RUITL projects
//!
//! This module provides a development server with hot reload, file watching,
//! and live development features.

use crate::config::{DevConfig, RuitlConfig};
use crate::error::{Result, ResultExt, RuitlError};
use crate::render::{
    RenderContext, RenderOptions, RenderTarget, Renderer, RendererConfig, UniversalRenderer,
};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::convert::Infallible;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::sleep;
use tokio_stream::StreamExt;
use walkdir::WalkDir;

/// Development server with hot reload capabilities
pub struct DevServer {
    config: DevConfig,
    project_config: RuitlConfig,
    renderer: Arc<RwLock<UniversalRenderer>>,
    file_watcher: Option<FileWatcher>,
    clients: Arc<RwLock<Vec<ClientConnection>>>,
}

/// File watcher for hot reload
struct FileWatcher {
    watcher: notify::RecommendedWatcher,
    receiver: mpsc::Receiver<notify::Result<Event>>,
}

/// Client connection for Server-Sent Events
struct ClientConnection {
    id: String,
    sender: mpsc::UnboundedSender<String>,
}

/// HTTP request handler
struct RequestHandler {
    renderer: Arc<RwLock<UniversalRenderer>>,
    config: RuitlConfig,
    static_files: HashMap<String, Vec<u8>>,
    clients: Arc<RwLock<Vec<ClientConnection>>>,
}

/// Server-Sent Events message
#[derive(Debug, Clone)]
enum SseMessage {
    Reload,
    FileChanged(String),
    Error(String),
}

impl DevServer {
    /// Create a new development server
    pub fn new(config: DevConfig, project_config: RuitlConfig) -> Result<Self> {
        let renderer_config = RendererConfig::default();
        let renderer = Arc::new(RwLock::new(UniversalRenderer::new(renderer_config)));

        Ok(Self {
            config,
            project_config,
            renderer,
            file_watcher: None,
            clients: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Start the development server
    pub async fn start(&mut self) -> Result<()> {
        // Setup file watcher if hot reload is enabled
        if self.config.hot_reload {
            self.setup_file_watcher().await?;
        }

        // Create HTTP service
        let handler = RequestHandler::new(
            self.renderer.clone(),
            self.project_config.clone(),
            self.clients.clone(),
        )
        .await?;

        let make_svc = make_service_fn(move |_conn| {
            let handler = handler.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let handler = handler.clone();
                    async move { handler.handle_request(req).await }
                }))
            }
        });

        // Start server
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .map_err(|e| RuitlError::server(format!("Invalid server address: {}", e)))?;

        let server = Server::bind(&addr).serve(make_svc);

        println!("🚀 Development server running on http://{}", addr);

        if self.config.open {
            self.open_browser(&format!("http://{}", addr)).await?;
        }

        // Start file watching task
        if let Some(watcher) = self.file_watcher.take() {
            let clients = self.clients.clone();
            let renderer = self.renderer.clone();
            let project_config = self.project_config.clone();

            tokio::spawn(async move {
                Self::watch_files(watcher, clients, renderer, project_config).await;
            });
        }

        // Run server
        server.await.server_context("Server error")?;

        Ok(())
    }

    /// Setup file watcher for hot reload
    async fn setup_file_watcher(&mut self) -> Result<()> {
        let (tx, rx) = mpsc::channel(1000);

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                futures::executor::block_on(async {
                    tx.send(res).await.ok();
                })
            },
            Config::default(),
        )
        .map_err(|e| RuitlError::server(format!("Failed to create file watcher: {}", e)))?;

        // Watch source directories
        for pattern in &self.config.watch {
            // Simple pattern matching - in production, you'd use a proper glob library
            if pattern.contains("**") {
                let base_path = pattern.split("**").next().unwrap_or(".");
                if Path::new(base_path).exists() {
                    watcher
                        .watch(Path::new(base_path), RecursiveMode::Recursive)
                        .map_err(|e| {
                            RuitlError::server(format!("Failed to watch directory: {}", e))
                        })?;
                }
            } else if Path::new(pattern).exists() {
                watcher
                    .watch(Path::new(pattern), RecursiveMode::Recursive)
                    .map_err(|e| RuitlError::server(format!("Failed to watch path: {}", e)))?;
            }
        }

        self.file_watcher = Some(FileWatcher {
            watcher,
            receiver: rx,
        });

        Ok(())
    }

    /// Watch for file changes and trigger hot reload
    async fn watch_files(
        mut watcher: FileWatcher,
        clients: Arc<RwLock<Vec<ClientConnection>>>,
        renderer: Arc<RwLock<UniversalRenderer>>,
        config: RuitlConfig,
    ) {
        while let Some(event_result) = watcher.receiver.recv().await {
            match event_result {
                Ok(event) => {
                    match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) => {
                            if let Some(path) = event.paths.first() {
                                println!("📝 File changed: {}", path.display());

                                // Reload templates and components
                                if let Err(e) =
                                    Self::reload_project(&renderer, &config, &path).await
                                {
                                    eprintln!("❌ Reload failed: {}", e);
                                    Self::broadcast_to_clients(
                                        &clients,
                                        SseMessage::Error(e.to_string()),
                                    )
                                    .await;
                                } else {
                                    Self::broadcast_to_clients(
                                        &clients,
                                        SseMessage::FileChanged(path.to_string_lossy().to_string()),
                                    )
                                    .await;
                                }
                            }
                        }
                        EventKind::Remove(_) => {
                            if let Some(path) = event.paths.first() {
                                println!("🗑️  File removed: {}", path.display());
                                Self::broadcast_to_clients(
                                    &clients,
                                    SseMessage::FileChanged(path.to_string_lossy().to_string()),
                                )
                                .await;
                            }
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    eprintln!("❌ File watcher error: {}", e);
                }
            }
        }
    }

    /// Reload project components and templates
    async fn reload_project(
        renderer: &Arc<RwLock<UniversalRenderer>>,
        config: &RuitlConfig,
        changed_path: &Path,
    ) -> Result<()> {
        let renderer = renderer.write().await;

        // Check if it's a template file
        if changed_path.starts_with(&config.build.template_dir) {
            if let Some(file_name) = changed_path.file_stem() {
                let template_name = file_name.to_string_lossy();
                let content = fs::read_to_string(changed_path)?;
                renderer.register_template(&template_name, &content).await?;
                println!("🔄 Reloaded template: {}", template_name);
            }
        }

        // Check if it's a component file
        if changed_path.starts_with(&config.components.dirs[0]) {
            // In a real implementation, you'd recompile the component
            println!("🔄 Component changed: {}", changed_path.display());
        }

        Ok(())
    }

    /// Broadcast message to all connected clients
    async fn broadcast_to_clients(
        clients: &Arc<RwLock<Vec<ClientConnection>>>,
        message: SseMessage,
    ) {
        let clients = clients.read().await;
        for client in clients.iter() {
            let _ = client.sender.send(Self::format_sse_message(&message));
        }
    }

    /// Format Server-Sent Events message
    fn format_sse_message(message: &SseMessage) -> String {
        match message {
            SseMessage::Reload => "event: reload\ndata: {\"type\":\"reload\"}\n\n".to_string(),
            SseMessage::FileChanged(path) => {
                format!(
                    "event: file-changed\ndata: {{\"type\":\"file-changed\",\"path\":\"{}\"}}\n\n",
                    path
                )
            }
            SseMessage::Error(error) => {
                format!(
                    "event: error\ndata: {{\"type\":\"error\",\"message\":\"{}\"}}\n\n",
                    error
                )
            }
        }
    }

    /// Open browser automatically
    async fn open_browser(&self, url: &str) -> Result<()> {
        println!("🌐 Opening browser...");

        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(&["/c", "start", url])
                .spawn()
                .server_context("Failed to open browser")?;
        }

        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(url)
                .spawn()
                .server_context("Failed to open browser")?;
        }

        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open")
                .arg(url)
                .spawn()
                .server_context("Failed to open browser")?;
        }

        Ok(())
    }
}

impl RequestHandler {
    /// Create a new request handler
    async fn new(
        renderer: Arc<RwLock<UniversalRenderer>>,
        config: RuitlConfig,
        clients: Arc<RwLock<Vec<ClientConnection>>>,
    ) -> Result<Self> {
        let static_files = Self::load_static_files(&config.build.static_dir).await?;

        Ok(Self {
            renderer,
            config,
            static_files,
            clients,
        })
    }

    /// Load static files into memory for serving
    async fn load_static_files(static_dir: &Path) -> Result<HashMap<String, Vec<u8>>> {
        let mut static_files = HashMap::new();

        if !static_dir.exists() {
            return Ok(static_files);
        }

        for entry in WalkDir::new(static_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();
                let relative_path = path.strip_prefix(static_dir).unwrap();
                let key = format!("/{}", relative_path.to_string_lossy().replace('\\', "/"));
                let content = fs::read(path)?;
                static_files.insert(key, content);
            }
        }

        Ok(static_files)
    }

    /// Handle HTTP request
    async fn handle_request(
        &self,
        req: Request<Body>,
    ) -> std::result::Result<Response<Body>, Infallible> {
        let response = match self.handle_request_internal(req).await {
            Ok(response) => response,
            Err(e) => {
                eprintln!("Request error: {}", e);
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!("Internal Server Error: {}", e)))
                    .unwrap()
            }
        };

        Ok(response)
    }

    /// Internal request handling
    async fn handle_request_internal(&self, req: Request<Body>) -> Result<Response<Body>> {
        let path = req.uri().path();
        let method = req.method();

        // Handle Server-Sent Events endpoint for hot reload
        if path == "/__ruitl_sse" {
            return self.handle_sse_connection(req).await;
        }

        // Handle static files
        if let Some(content) = self.static_files.get(path) {
            let content_type = self.guess_content_type(path);
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", content_type)
                .body(Body::from(content.clone()))?);
        }

        // Handle dynamic routes
        match method {
            &Method::GET => self.handle_get_request(path).await,
            &Method::POST => self.handle_post_request(req).await,
            _ => Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body(Body::from("Method Not Allowed"))?),
        }
    }

    /// Handle GET requests
    async fn handle_get_request(&self, path: &str) -> Result<Response<Body>> {
        // Create render context
        let context = RenderContext::new()
            .with_path(path)
            .with_target(RenderTarget::Development);

        // Create render options
        let mut options = RenderOptions::new().pretty();

        // Add hot reload script if enabled
        if self.config.dev.hot_reload {
            options = options.with_head_element(crate::html::Html::raw(HOT_RELOAD_SCRIPT));
        }

        // Render page
        let renderer = self.renderer.read().await;
        let html = renderer.render(&context, &options).await?;

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/html; charset=utf-8")
            .body(Body::from(html))?)
    }

    /// Handle POST requests
    async fn handle_post_request(&self, req: Request<Body>) -> Result<Response<Body>> {
        // Handle API endpoints, form submissions, etc.
        let path = req.uri().path();

        match path {
            "/api/reload" => {
                // Trigger manual reload
                Self::broadcast_to_clients(&self.clients, SseMessage::Reload).await;
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from("{\"status\":\"ok\"}"))?)
            }
            _ => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not Found"))?),
        }
    }

    /// Handle Server-Sent Events connection
    async fn handle_sse_connection(&self, _req: Request<Body>) -> Result<Response<Body>> {
        let (tx, rx) = mpsc::unbounded_channel();

        // Generate client ID
        let client_id = format!("client_{}", uuid::Uuid::new_v4());

        // Add client to list
        {
            let mut clients = self.clients.write().await;
            clients.push(ClientConnection {
                id: client_id.clone(),
                sender: tx,
            });
        }

        // Create response stream
        let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx)
            .map(|item| Ok::<_, hyper::Error>(item));
        let body = Body::wrap_stream(stream);

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .header("connection", "keep-alive")
            .header("access-control-allow-origin", "*")
            .body(body)?)
    }

    /// Guess content type from file extension
    fn guess_content_type(&self, path: &str) -> &'static str {
        if let Some(ext) = Path::new(path).extension().and_then(|s| s.to_str()) {
            match ext {
                "html" => "text/html; charset=utf-8",
                "css" => "text/css",
                "js" => "application/javascript",
                "json" => "application/json",
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "svg" => "image/svg+xml",
                "ico" => "image/x-icon",
                "woff" => "font/woff",
                "woff2" => "font/woff2",
                "ttf" => "font/ttf",
                "eot" => "application/vnd.ms-fontobject",
                _ => "application/octet-stream",
            }
        } else {
            "text/plain"
        }
    }

    /// Broadcast message to all clients
    async fn broadcast_to_clients(
        clients: &Arc<RwLock<Vec<ClientConnection>>>,
        message: SseMessage,
    ) {
        let clients = clients.read().await;
        for client in clients.iter() {
            let _ = client.sender.send(DevServer::format_sse_message(&message));
        }
    }
}

impl Clone for RequestHandler {
    fn clone(&self) -> Self {
        Self {
            renderer: self.renderer.clone(),
            config: self.config.clone(),
            static_files: self.static_files.clone(),
            clients: self.clients.clone(),
        }
    }
}

/// Hot reload script injected into HTML pages
const HOT_RELOAD_SCRIPT: &str = r#"
<script>
(function() {
    if (typeof EventSource === 'undefined') {
        console.warn('RUITL: Hot reload not supported (EventSource not available)');
        return;
    }

    const eventSource = new EventSource('/__ruitl_sse');

    eventSource.onmessage = function(event) {
        const data = JSON.parse(event.data);
        console.log('RUITL:', data);

        if (data.type === 'reload' || data.type === 'file-changed') {
            console.log('RUITL: Reloading page...');
            window.location.reload();
        } else if (data.type === 'error') {
            console.error('RUITL Error:', data.message);
        }
    };

    eventSource.onerror = function(event) {
        console.error('RUITL: Hot reload connection error');
    };

    eventSource.onopen = function(event) {
        console.log('RUITL: Hot reload connected');
    };

    window.addEventListener('beforeunload', function() {
        eventSource.close();
    });
})();
</script>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_dev_server_creation() {
        let dev_config = DevConfig::default();
        let project_config = RuitlConfig::default();

        let server = DevServer::new(dev_config, project_config);
        assert!(server.is_ok());
    }

    #[test]
    fn test_sse_message_formatting() {
        let reload_msg = DevServer::format_sse_message(&SseMessage::Reload);
        assert!(reload_msg.contains("event: reload"));

        let file_msg =
            DevServer::format_sse_message(&SseMessage::FileChanged("test.rs".to_string()));
        assert!(file_msg.contains("event: file-changed"));
        assert!(file_msg.contains("test.rs"));

        let error_msg = DevServer::format_sse_message(&SseMessage::Error("test error".to_string()));
        assert!(error_msg.contains("event: error"));
        assert!(error_msg.contains("test error"));
    }

    #[tokio::test]
    async fn test_static_file_loading() {
        let temp_dir = tempdir().unwrap();
        let static_dir = temp_dir.path().join("static");
        fs::create_dir_all(&static_dir).unwrap();

        // Create test file
        let test_file = static_dir.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let static_files = RequestHandler::load_static_files(&static_dir)
            .await
            .unwrap();
        assert!(static_files.contains_key("/test.txt"));
        assert_eq!(static_files.get("/test.txt").unwrap(), b"test content");
    }

    #[test]
    fn test_content_type_guessing() {
        let handler = RequestHandler {
            renderer: Arc::new(RwLock::new(UniversalRenderer::new(
                RendererConfig::default(),
            ))),
            config: RuitlConfig::default(),
            static_files: HashMap::new(),
            clients: Arc::new(RwLock::new(Vec::new())),
        };

        assert_eq!(
            handler.guess_content_type("/test.html"),
            "text/html; charset=utf-8"
        );
        assert_eq!(handler.guess_content_type("/style.css"), "text/css");
        assert_eq!(
            handler.guess_content_type("/script.js"),
            "application/javascript"
        );
        assert_eq!(handler.guess_content_type("/image.png"), "image/png");
        assert_eq!(handler.guess_content_type("/unknown"), "text/plain");
    }
}
