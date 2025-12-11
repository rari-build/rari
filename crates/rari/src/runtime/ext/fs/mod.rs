use super::ExtensionTrait;
use deno_core::{Extension, extension};
use deno_fs::FileSystemRc;

extension!(
    init_fs,
    deps = [rari],
    esm_entry_point = "ext:init_fs/init_fs.js",
    esm = [ dir "src/runtime/ext/fs", "init_fs.js" ],
);

impl ExtensionTrait<()> for init_fs {
    fn init((): ()) -> Extension {
        init_fs::init()
    }
}

impl ExtensionTrait<FileSystemRc> for deno_fs::deno_fs {
    fn init(fs: FileSystemRc) -> Extension {
        deno_fs::deno_fs::init(fs)
    }
}

pub fn extensions(fs: FileSystemRc, is_snapshot: bool) -> Vec<Extension> {
    vec![deno_fs::deno_fs::build(fs, is_snapshot), init_fs::build((), is_snapshot)]
}
