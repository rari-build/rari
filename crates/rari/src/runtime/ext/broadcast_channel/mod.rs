use super::ExtensionTrait;
use deno_broadcast_channel::InMemoryBroadcastChannel;
use deno_core::{Extension, extension};

mod wrapper;

extension!(
    init_broadcast_channel,
    deps = [rari],
    esm_entry_point = "ext:init_broadcast_channel/init_broadcast_channel.js",
    esm = [ dir "src/runtime/ext/broadcast_channel", "init_broadcast_channel.js" ],
);
impl ExtensionTrait<()> for init_broadcast_channel {
    fn init((): ()) -> Extension {
        init_broadcast_channel::init()
    }
}
impl ExtensionTrait<InMemoryBroadcastChannel> for deno_broadcast_channel::deno_broadcast_channel {
    fn init(channel: InMemoryBroadcastChannel) -> Extension {
        deno_broadcast_channel::deno_broadcast_channel::init(channel)
    }
}

pub fn extensions(channel: InMemoryBroadcastChannel, is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_broadcast_channel::deno_broadcast_channel::build(channel, is_snapshot),
        init_broadcast_channel::build((), is_snapshot),
    ]
}
