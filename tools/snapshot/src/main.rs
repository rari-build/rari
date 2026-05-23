use std::path::PathBuf;
use std::rc::Rc;

use deno_core::{ModuleCodeString, ModuleName, SourceMapData};
use deno_error::JsErrorBox;

type Transpiler = dyn Fn(
    ModuleName,
    ModuleCodeString,
) -> Result<(ModuleCodeString, Option<SourceMapData>), JsErrorBox>;

fn main() {
    let output_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("crates/rari/snapshots/RARI_SNAPSHOT.bin"));

    println!("Building V8 snapshot...");

    let ext_options = rari::runtime::ext::ExtensionOptions::default();
    let extensions = rari::runtime::ext::extensions(&ext_options, false);

    let transpiler: Rc<Transpiler> = Rc::new(|name, source| {
        rari::runtime::utils::transpile::maybe_transpile_source(name, source)
    });

    let output = deno_core::snapshot::create_snapshot(
        deno_core::snapshot::CreateSnapshotOptions {
            cargo_manifest_dir: env!("CARGO_MANIFEST_DIR"),
            startup_snapshot: None,
            skip_op_registration: false,
            extensions,
            extension_transpiler: Some(transpiler),
            with_runtime_cb: None,
        },
        None,
    )
    .expect("Failed to create V8 snapshot");

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create output directory");
    }

    std::fs::write(&output_path, &output.output).expect("Failed to write snapshot file");

    println!(
        "Snapshot written to {} ({:.2} MB)",
        output_path.display(),
        output.output.len() as f64 / 1024.0 / 1024.0
    );
    println!("Files loaded during snapshot: {}", output.files_loaded_during_snapshot.len());
}
