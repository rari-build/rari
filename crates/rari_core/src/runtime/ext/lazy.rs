use deno_core::{Extension, ExtensionArguments};

pub fn merge(
    extensions: &mut Vec<Extension>,
    lazy_args: &mut Vec<ExtensionArguments>,
    batch: (Vec<Extension>, Vec<ExtensionArguments>),
) {
    let (exts, args) = batch;
    extensions.extend(exts);
    lazy_args.extend(args);
}

pub fn register<A, T: super::ExtensionTrait<A>>(
    options: A,
    is_snapshot: bool,
    extensions: &mut Vec<Extension>,
    lazy_args: &mut Vec<ExtensionArguments>,
) where
    A: Clone,
{
    T::register(options, is_snapshot, extensions, lazy_args);
}
