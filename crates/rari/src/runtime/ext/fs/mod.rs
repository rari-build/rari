use ::deno_fs::FileSystemRc;
use deno_core::{Extension, extension};
use deno_fs::deno_fs;

use super::ExtensionTrait;

extension!(
    init_fs,
    deps = [rari],
    esm_entry_point = "ext:init_fs/init_fs.ts",
    esm = [ dir "src/runtime/ext/fs", "init_fs.ts" ],
);

impl ExtensionTrait<()> for init_fs {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

impl ExtensionTrait<FileSystemRc> for deno_fs {
    fn init(fs: FileSystemRc) -> Extension {
        Self::init(fs)
    }
}

pub fn extensions(fs: FileSystemRc, is_snapshot: bool) -> Vec<Extension> {
    vec![deno_fs::build(fs, is_snapshot), init_fs::build((), is_snapshot)]
}
