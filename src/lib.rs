use std::sync::atomic::AtomicU8;

use anvil::{eth::EthApi, NodeHandle};

pub mod ipc;
pub use ipc::*;

pub mod http;
pub use http::*;

mod bindings;


static TRACE_COUNT: AtomicU8 = AtomicU8::new(0);
const GAS: u64 = 28_000_000;

pub async fn shutdown(api: EthApi, handle: NodeHandle) {
    // If fork exists, flush the cache
    if let Some(fork) = api.get_fork().clone() {
        fork.database().read().await.flush_cache();
    }
    handle.server.abort();
    handle.node_service.abort();

    drop(api);
    drop(handle);
}