use ::deno_fs::FileSystemRc;
use deno_core::{Extension, ExtensionArguments, extension};
use deno_fs::deno_fs;

use super::{ExtensionTrait, lazy};

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
    const LAZY_INIT: bool = true;

    fn init(fs: FileSystemRc) -> Extension {
        Self::init(fs)
    }

    fn lazy_init() -> Extension {
        Self::lazy_init()
    }

    fn lazy_args(fs: FileSystemRc) -> ExtensionArguments {
        Self::args(fs)
    }
}

pub fn extensions(
    fs: FileSystemRc,
    is_snapshot: bool,
) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();
    lazy::register::<FileSystemRc, deno_fs>(fs, is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), init_fs>((), is_snapshot, &mut extensions, &mut lazy_args);
    (extensions, lazy_args)
}
