//! Zed extension entry point for RUITL.
//!
//! Compiles to WASM via the `zed_extension_api` crate. Zed loads the
//! extension, calls `language_server_command` whenever it needs to spawn
//! the LSP for a `.ruitl` file, and we return the `ruitl-lsp` binary
//! (which the user installs separately via `cargo install --path ruitl_lsp`).

use zed_extension_api::{self as zed, Command, LanguageServerId, Result, Worktree};

struct RuitlExtension;

impl zed::Extension for RuitlExtension {
    fn new() -> Self {
        Self
    }

    /// Locate `ruitl-lsp` on PATH and launch it. Zed talks to the server
    /// over stdio — no extra args.
    fn language_server_command(
        &mut self,
        _server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        let path = worktree
            .which("ruitl-lsp")
            .ok_or_else(|| {
                "`ruitl-lsp` not found on PATH. Install with `cargo install \
                 --path ruitl_lsp` from the RUITL repo."
                    .to_string()
            })?;
        Ok(Command {
            command: path,
            args: vec![],
            env: Default::default(),
        })
    }
}

zed::register_extension!(RuitlExtension);
