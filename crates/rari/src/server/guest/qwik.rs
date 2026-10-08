//! Qwik guest renderer.
//!
//! `@rari/qwik`'s build emits `dist/server/qwik-server-entry.mjs`: a
//! self-contained ESM bundle of the app, Qwik Router's `requestHandler`, and the
//! rari platform shim. Evaluating it installs `globalThis.__rariQwikHandle`, an
//! async function taking the host's [`GuestRequest`] and speaking the protocol
//! in [`super::stream`]. Qwik owns loaders, actions, `server$`, cookies,
//! redirects and rendering; the host [`pipeline`](super::pipeline) owns
//! everything around the request. What is left here is only what is Qwik's:
//! where its build output lives, how its URLs map to pages, and the render call.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use rari_error::RariError;
use tokio::fs;

use super::{
    GuestRenderer, StaticMount,
    pipeline::{GuestRequest, default_page_pathname},
    stream::{GuestStream, run_guest_script},
};
use crate::{
    async_trait::async_trait,
    runtime::{JsExecutionRuntime, factory::JsRuntimeInterface},
    server::config::Framework,
};

/// Server bundle the Qwik build writes, relative to the project root.
pub const SERVER_ENTRY: &str = "dist/server/qwik-server-entry.mjs";
/// Client output the Qwik build writes, relative to the project root.
pub const CLIENT_DIR: &str = "dist/client";
/// Global the server bundle installs.
const HANDLER_GLOBAL: &str = "__rariQwikHandle";

pub struct QwikGuest {
    runtime: Arc<JsExecutionRuntime>,
    /// `file://` URL the bundle is registered under in the module loader.
    entry_specifier: String,
    /// The loader keys registered modules by a component id derived from the
    /// specifier (for `dist/server/<file>` that is the file name); loading and
    /// evaluating goes through that id, as the API-route loader does.
    entry_component_id: String,
    entry_code: String,
    client_dir: PathBuf,
}

impl QwikGuest {
    /// Load the server bundle into every isolate of the pool.
    ///
    /// # Errors
    ///
    /// Fails when the bundle is missing or unreadable, or when evaluating it in
    /// the runtime fails (including when it does not install the handler).
    pub async fn load(
        runtime: Arc<JsExecutionRuntime>,
        project_root: &Path,
    ) -> Result<Self, RariError> {
        let entry = project_root.join(SERVER_ENTRY);
        let canonical = fs::canonicalize(&entry).await.map_err(|err| {
            RariError::configuration(format!(
                "Qwik server entry {} not found ({err}). Build the app with @rari/qwik first.",
                entry.display()
            ))
        })?;
        let entry_code = fs::read_to_string(&canonical).await.map_err(|err| {
            RariError::io(format!(
                "Failed to read Qwik server entry {}: {err}",
                canonical.display()
            ))
        })?;
        let entry_specifier = url::Url::from_file_path(&canonical)
            .map_err(|()| {
                RariError::configuration(format!(
                    "Failed to create a file URL for {}",
                    canonical.display()
                ))
            })?
            .to_string();

        let entry_component_id =
            canonical.file_name().and_then(|name| name.to_str()).map(str::to_string).ok_or_else(
                || {
                    RariError::configuration(format!(
                        "Qwik server entry has no file name: {}",
                        canonical.display()
                    ))
                },
            )?;

        runtime.add_module_to_loader(&entry_specifier, entry_code.clone()).await?;
        runtime.load_and_evaluate_module(&entry_component_id).await?;
        runtime
            .broadcast_script(
                "qwik_guest_check",
                &format!(
                    "if (typeof globalThis.{HANDLER_GLOBAL} !== 'function') \
                     throw new Error('Qwik server entry did not install globalThis.{HANDLER_GLOBAL}')"
                ),
            )
            .await?;

        Ok(Self {
            runtime,
            entry_specifier,
            entry_component_id,
            entry_code,
            client_dir: project_root.join(CLIENT_DIR),
        })
    }
}

/// Qwik Router fetches loader data for a page at `<page>/q-loader-<id>.<hash>.json`;
/// everything else is a page URL. Strip that suffix (and Qwik's trailing slash)
/// so the host matches the page the request belongs to.
fn qwik_page_pathname(path: &str) -> String {
    let (page, last) = match path.rsplit_once('/') {
        Some((page, last)) => (page, last),
        None => (path, ""),
    };
    let page_path =
        if last.starts_with("q-loader-") && last.ends_with(".json") { page } else { path };
    default_page_pathname(page_path)
}

#[async_trait]
impl GuestRenderer for QwikGuest {
    fn framework(&self) -> Framework {
        Framework::Qwik
    }

    fn static_mounts(&self) -> Vec<StaticMount> {
        vec![
            StaticMount::new("/build", self.client_dir.join("build"), true),
            StaticMount::new("/assets", self.client_dir.join("assets"), true),
        ]
    }

    fn public_dir(&self) -> Option<&Path> {
        Some(&self.client_dir)
    }

    fn page_pathname(&self, path: &str) -> String {
        qwik_page_pathname(path)
    }

    /// Qwik Router serves `/.well-known/*` endpoints from `plugin@*.ts`
    /// middleware without a page file for them.
    fn owns_unmatched_path(&self, path: &str) -> bool {
        path == "/.well-known" || path.starts_with("/.well-known/")
    }

    async fn resync_slot(&self, runtime: &Arc<dyn JsRuntimeInterface>) -> Result<(), RariError> {
        runtime.add_module_to_loader(&self.entry_specifier, self.entry_code.clone()).await?;
        let module_id = runtime.load_es_module(&self.entry_component_id).await?;
        runtime.evaluate_module(module_id).await?;
        Ok(())
    }

    async fn render(&self, request: &GuestRequest) -> Result<GuestStream, RariError> {
        let script = format!("globalThis.{HANDLER_GLOBAL}({})", request.to_json()?);
        run_guest_script(&self.runtime, request.stream_id.clone(), "qwik_render", script).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qwik_internal_suffixes_and_trailing_slashes_map_to_the_page() {
        assert_eq!(qwik_page_pathname("/"), "/");
        assert_eq!(qwik_page_pathname("/about/"), "/about");
        assert_eq!(qwik_page_pathname("/blog/hello/"), "/blog/hello");
        assert_eq!(qwik_page_pathname("/blog/hello/q-loader-abc.4vn0l9.json"), "/blog/hello");
        assert_eq!(qwik_page_pathname("/q-loader-abc.4vn0l9.json"), "/");
        assert_eq!(qwik_page_pathname("/q-loader-not-json"), "/q-loader-not-json");
    }
}
