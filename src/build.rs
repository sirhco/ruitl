//! Static-site build pipeline driven by `[[routes]]` entries in
//! `ruitl.toml`.
//!
//! Because component dispatch by name requires knowing the concrete types at
//! the call site, the public entry point takes a user-supplied closure
//! `renderer: Fn(name, props_json) -> Result<String>`. The scaffolder
//! generates a template renderer at project creation time (see
//! `src/cli.rs::generate_site_renderer_content`). Consumers call:
//!
//! ```ignore
//! ruitl::build::render_site(&cfg, &out_dir, |name, props_json| {
//!     match name {
//!         "HomePage" => {
//!             let props: HomePageProps = serde_json::from_str(props_json)?;
//!             let html = HomePage.render(&props, &ComponentContext::new())?;
//!             Ok(html.render())
//!         }
//!         other => Err(ruitl::RuitlError::generic(format!("unknown route: {}", other))),
//!     }
//! });
//! ```

use crate::config::{RouteConfig, RuitlConfig};
use crate::error::{Result, RuitlError};
use std::fs;
use std::path::{Path, PathBuf};

/// Map a URL path to a filesystem path under `out_dir`. `/` and empty paths
/// resolve to `index.html`; every other path becomes `<stripped>/index.html`
/// so it serves at its URL from any plain static-file server.
fn route_to_file(out_dir: &Path, url_path: &str) -> PathBuf {
    let trimmed = url_path.trim_matches('/');
    if trimmed.is_empty() {
        return out_dir.join("index.html");
    }
    out_dir.join(trimmed).join("index.html")
}

/// Render every route listed in the config using the caller-provided
/// dispatcher. On success returns the list of files written.
pub fn render_site<F>(cfg: &RuitlConfig, out_dir: &Path, mut renderer: F) -> Result<Vec<PathBuf>>
where
    F: FnMut(&str, &str) -> Result<String>,
{
    fs::create_dir_all(out_dir)
        .map_err(|e| RuitlError::config(format!("create out dir {}: {}", out_dir.display(), e)))?;

    let mut written = Vec::with_capacity(cfg.routes.len());
    for route in &cfg.routes {
        let output = render_route(route, out_dir, &mut renderer)?;
        written.push(output);
    }
    Ok(written)
}

fn render_route<F>(route: &RouteConfig, out_dir: &Path, renderer: &mut F) -> Result<PathBuf>
where
    F: FnMut(&str, &str) -> Result<String>,
{
    let props_json = fs::read_to_string(&route.props_file).map_err(|e| {
        RuitlError::config(format!(
            "read props file {}: {}",
            route.props_file.display(),
            e
        ))
    })?;
    let html = renderer(&route.component, &props_json)?;
    let target = route_to_file(out_dir, &route.path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            RuitlError::config(format!("create {}: {}", parent.display(), e))
        })?;
    }
    fs::write(&target, html)
        .map_err(|e| RuitlError::config(format!("write {}: {}", target.display(), e)))?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_to_file_maps_root_to_index() {
        let got = route_to_file(Path::new("/dist"), "/");
        assert_eq!(got, PathBuf::from("/dist/index.html"));
    }

    #[test]
    fn route_to_file_preserves_nested_path() {
        let got = route_to_file(Path::new("/dist"), "/blog/post");
        assert_eq!(got, PathBuf::from("/dist/blog/post/index.html"));
    }
}
