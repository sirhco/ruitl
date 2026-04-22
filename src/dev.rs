//! Development server with browser auto-reload.
//!
//! `ruitl dev` runs a file watcher over `.ruitl` templates and serves a tiny
//! HTTP sidecar (default port 35729) with two endpoints:
//!
//! - `GET /ruitl/reload.js` — client JS that subscribes to the SSE stream
//!   and reloads the page on each tick. Scaffolded projects inject this
//!   script tag when `--with-hot-reload` is set.
//! - `GET /ruitl/reload` — Server-Sent Events endpoint. Emits `event: reload`
//!   after each successful template recompile.
//!
//! Why SSE, not WebSocket: SSE needs no extra dependency (one-way text
//! events over plain HTTP), auto-reconnects, and is enough for
//! "reload-the-page" semantics. HMR is out of scope for server-rendered
//! Rust components; the user's binary must be rebuilt + restarted to pick
//! up code changes, which the developer handles separately (e.g.
//! `cargo watch -x run`).

use crate::error::{Result, RuitlError};
use colored::*;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

/// Client script served from `/ruitl/reload.js`. The port placeholder is
/// substituted per-request so the script always connects back to the
/// sidecar the user actually launched.
const RELOAD_JS_TEMPLATE: &str = r#"(() => {
  const es = new EventSource("__RUITL_RELOAD_URL__");
  es.addEventListener("reload", () => window.location.reload());
  es.addEventListener("ping", () => {}); // keep-alive noop
  window.addEventListener("beforeunload", () => es.close());
})();
"#;

/// Configuration for `ruitl dev`. Keep minimal — most defaults are fine
/// for the local-dev loop.
#[derive(Debug, Clone)]
pub struct DevOptions {
    /// Port for the sidecar reload server. 35729 is LiveReload's legacy
    /// default and rarely clashes with app servers.
    pub reload_port: u16,
    /// Verbose logging of every recompile / SSE event.
    pub verbose: bool,
}

impl Default for DevOptions {
    fn default() -> Self {
        Self {
            reload_port: 35729,
            verbose: false,
        }
    }
}

/// Handle to the reload bus. Cloned into the watcher and into each SSE
/// client task so they all observe the same tick stream.
#[derive(Clone)]
struct ReloadBus {
    tx: broadcast::Sender<()>,
}

impl ReloadBus {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(16);
        Self { tx }
    }

    fn subscribe(&self) -> broadcast::Receiver<()> {
        self.tx.subscribe()
    }

    fn fire(&self) {
        // It's fine if no receivers are connected — error just means no
        // browsers have the reload endpoint open yet.
        let _ = self.tx.send(());
    }
}

/// Run the dev loop: watch `src_dir`, recompile on each change, serve SSE
/// ticks on `opts.reload_port`. Blocks the calling task until Ctrl+C.
pub async fn run_dev(src_dir: &Path, opts: DevOptions) -> Result<()> {
    let bus = Arc::new(ReloadBus::new());

    // Initial compile — fail fast if the starting state is broken.
    ruitl_compiler::compile_dir_sibling(src_dir)
        .map_err(|e| RuitlError::generic(format!("Initial compile failed: {}", e)))?;
    println!("{}", "✓ Initial compile OK".green());

    // Spawn the watcher on a blocking worker so the async runtime keeps
    // the HTTP server responsive. The watcher sends a tick into `bus`
    // after each successful recompile.
    #[cfg(feature = "dev")]
    {
        let src_owned = src_dir.to_path_buf();
        let bus_for_watch = Arc::clone(&bus);
        let verbose = opts.verbose;
        tokio::task::spawn_blocking(move || {
            if let Err(e) = run_watcher_blocking(&src_owned, bus_for_watch, verbose) {
                eprintln!("{} watcher failed: {}", "error:".red(), e);
            }
        });
    }
    #[cfg(not(feature = "dev"))]
    {
        return Err(RuitlError::generic(
            "`ruitl dev` requires the 'dev' feature. Rebuild with `cargo build --features dev`.",
        ));
    }

    // Run the HTTP sidecar — serves the reload script + SSE endpoint.
    let addr: SocketAddr = ([127, 0, 0, 1], opts.reload_port).into();
    println!(
        "{} reload server on http://{}",
        "✓".green(),
        addr.to_string().bright_blue()
    );
    println!(
        "  Script tag: {}",
        format!(
            "<script src=\"http://{}/ruitl/reload.js\"></script>",
            addr
        )
        .bright_black()
    );
    println!("  Press Ctrl+C to stop.");

    let bus_for_server = Arc::clone(&bus);
    let make_svc = make_service_fn(move |_| {
        let bus = Arc::clone(&bus_for_server);
        let port = opts.reload_port;
        async move {
            let bus = bus.clone();
            Ok::<_, Infallible>(service_fn(move |req| {
                let bus = bus.clone();
                async move { handle_request(req, bus, port).await }
            }))
        }
    });

    Server::bind(&addr)
        .serve(make_svc)
        .await
        .map_err(|e| RuitlError::generic(format!("Reload server error: {}", e)))?;
    Ok(())
}

#[cfg(feature = "dev")]
fn run_watcher_blocking(
    src_dir: &Path,
    bus: Arc<ReloadBus>,
    verbose: bool,
) -> Result<()> {
    use hotwatch::{Event, Hotwatch};
    use std::path::PathBuf;

    let mut hotwatch = Hotwatch::new_with_custom_delay(Duration::from_millis(150))
        .map_err(|e| RuitlError::generic(format!("Failed to start watcher: {}", e)))?;

    let src_owned = src_dir.to_path_buf();
    hotwatch
        .watch(src_dir, move |event: Event| {
            let changed: Option<&PathBuf> = match &event {
                Event::Create(p)
                | Event::Write(p)
                | Event::Remove(p)
                | Event::Rename(p, _) => Some(p),
                _ => None,
            };
            let Some(path) = changed else { return };
            if path.extension().map(|e| e != "ruitl").unwrap_or(true) {
                return;
            }
            if verbose {
                println!(
                    "{} change in {}",
                    "info:".bright_blue().bold(),
                    path.display()
                );
            }
            match ruitl_compiler::compile_dir_sibling(&src_owned) {
                Ok(_) => {
                    println!("{} recompiled, notifying browsers", "✓".green());
                    bus.fire();
                }
                Err(e) => {
                    eprintln!("{} recompile failed: {}", "error:".red(), e);
                }
            }
        })
        .map_err(|e| RuitlError::generic(format!("Failed to watch '{}': {}", src_dir.display(), e)))?;

    // Park this thread so hotwatch's background thread keeps processing.
    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

async fn handle_request(
    req: Request<Body>,
    bus: Arc<ReloadBus>,
    port: u16,
) -> std::result::Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/ruitl/reload.js") => Ok(reload_js_response(port)),
        (&Method::GET, "/ruitl/reload") => Ok(sse_response(bus.subscribe())),
        _ => Ok(not_found()),
    }
}

fn reload_js_response(port: u16) -> Response<Body> {
    let body = RELOAD_JS_TEMPLATE
        .replace(
            "__RUITL_RELOAD_URL__",
            &format!("http://127.0.0.1:{}/ruitl/reload", port),
        );
    Response::builder()
        .header("content-type", "application/javascript; charset=utf-8")
        // Avoid caching — the dev server is the only consumer.
        .header("cache-control", "no-cache")
        // Allow injection from any origin so an app on a different port
        // can still pull the script.
        .header("access-control-allow-origin", "*")
        .body(Body::from(body))
        .unwrap()
}

/// Wrap a `broadcast::Receiver<()>` as a stream of SSE-formatted frames.
/// Merges the reload channel with a 30s ping ticker (so proxies don't close
/// the connection) into a single HTTP response body.
fn sse_response(rx: broadcast::Receiver<()>) -> Response<Body> {
    use futures::stream::StreamExt;
    use tokio_stream::wrappers::{BroadcastStream, IntervalStream};

    // Map each reload tick into an SSE `reload` frame, dropping lag errors
    // (browser reconnects automatically on close).
    let reloads = BroadcastStream::new(rx).filter_map(|item| async move {
        match item {
            Ok(_) => Some(hyper::body::Bytes::from(
                "event: reload\ndata: \n\n".to_string(),
            )),
            Err(_) => None,
        }
    });

    // Keep-alive pings every 30s so intermediaries don't prune idle
    // connections.
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let pings = IntervalStream::new(interval)
        .map(|_| hyper::body::Bytes::from("event: ping\ndata: \n\n".to_string()));

    // Prime with an immediate hello frame so clients know they connected.
    let hello = futures::stream::once(async {
        hyper::body::Bytes::from(":connected\n\n".to_string())
    });

    let merged = hello
        .chain(futures::stream::select(reloads, pings))
        .map(Ok::<_, Infallible>);

    Response::builder()
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .header("access-control-allow-origin", "*")
        .body(Body::wrap_stream(merged))
        .unwrap()
}

fn not_found() -> Response<Body> {
    let mut r = Response::new(Body::from("not found"));
    *r.status_mut() = StatusCode::NOT_FOUND;
    r
}

/// Return the `<script>` tag snippet to embed in layouts so pages subscribe
/// to reload events. Emitted by the scaffolder when `--with-hot-reload`
/// is set; also useful for documentation / copy-paste.
pub fn reload_script_tag(port: u16) -> String {
    format!(
        r#"<script src="http://127.0.0.1:{}/ruitl/reload.js"></script>"#,
        port
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reload_bus_fires_to_subscribers() {
        let bus = ReloadBus::new();
        let mut rx = bus.subscribe();
        bus.fire();
        // `broadcast::Receiver::try_recv` avoids an async runtime dep.
        assert!(rx.try_recv().is_ok(), "bus tick must reach subscriber");
    }

    #[test]
    fn reload_bus_no_receivers_no_panic() {
        let bus = ReloadBus::new();
        bus.fire(); // no one subscribed — must not panic
    }

    #[test]
    fn script_tag_embeds_port() {
        let t = reload_script_tag(12345);
        assert!(t.contains(":12345/"));
        assert!(t.contains("<script src=\""));
    }
}
