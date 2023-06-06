use std::sync::Arc;
use std::time::Instant;

use anvil::{eth::EthApi, spawn, NodeConfig, NodeHandle};
use bindings::convex::ShutdownSystemCall;
use ethers::{abi::AbiEncode, prelude::*};
use ndarray::Array1;
use ndarray_stats::QuantileExt;
use std::{env, future::Future};
use std::sync::atomic::{AtomicU8, Ordering};

use crate::{bindings, TRACE_COUNT, GAS, shutdown};



pub async fn spawn_ipc() -> Result<(Arc<Provider<Ipc>>, EthApi, NodeHandle), Box<dyn std::error::Error>>
{
    let ipc_path = env::var("ETH_IPC_PATH").expect("ETH_IPC_PATH not found in .env");
    let mut config = NodeConfig::default()
        .with_eth_ipc_path(Some(ipc_path.to_string()))
        .with_fork_block_number::<u64>(Some(14445961))
        .with_ipc(Some(None))
        .with_steps_tracing(false)
        .with_gas_limit(Some(GAS))
        .no_storage_caching()
        .silent();

    // only set up tracing for the first run
    if TRACE_COUNT.load(Ordering::SeqCst) == 0 {
        config = config.with_tracing(true).with_steps_tracing(true);
        TRACE_COUNT.fetch_add(1, Ordering::SeqCst);
    } else {
        config = config.silent().with_steps_tracing(false);
    }

        spawn_with_ipc_config(config).await
}


pub async fn spawn_ethers_reth(
) -> Result<(Arc<Provider<Ipc>>, EthApi, NodeHandle), Box<dyn std::error::Error>> {
    let ipc_path = env::var("ETH_IPC_PATH").expect("ETH_IPC_PATH not found in .env");
    let db_path = env::var("ETH_DB_PATH").expect("ETH_DB_PATH not found in .env");

    let config = NodeConfig::default()
        .with_eth_ipc_path(Some(ipc_path.to_string()))
        .with_eth_reth_db(Some(db_path.to_string()))
        .with_fork_block_number::<u64>(Some(14445961))
        .with_ipc(Some(None))
        .with_steps_tracing(false)
        .with_gas_limit(Some(GAS))
        .no_storage_caching()
        .silent();

        spawn_with_ipc_config(config).await
}

pub async fn spawn_with_http_config(
    config: NodeConfig,
) -> Result<(Arc<Provider<Http>>, EthApi, NodeHandle), Box<dyn std::error::Error>> {
    let (api, handle) = spawn(config).await;

    api.anvil_auto_impersonate_account(true).await?;

    let provider = Arc::new(handle.http_provider());

    Ok((provider, api, handle))
}

pub async fn spawn_with_ipc_config(
    config: NodeConfig,
) -> Result<(Arc<Provider<Ipc>>, EthApi, NodeHandle), Box<dyn std::error::Error>> {
    let (api, handle) = spawn(config).await;

    api.anvil_auto_impersonate_account(true).await?;

    let provider = Arc::new(Provider::<Ipc>::connect_ipc(handle.ipc_path().unwrap()).await?);

    Ok((provider, api, handle))
}

pub async fn ipc_system_shutdown(provider: Arc<Provider<Ipc>>, api: &EthApi) {
    let convex_sys: H160 = "0xF403C135812408BFbE8713b5A23a04b3D48AAE31".parse().unwrap();
    let owner: H160 = "0x3cE6408F923326f81A7D7929952947748180f1E6".parse().unwrap();

    api.anvil_set_balance(owner, U256::from(1e19 as u64)).await.unwrap();

    let shutdown = ShutdownSystemCall {}.encode().into();

    let nonce = provider.get_transaction_count(owner, None).await.unwrap();
    let gas_price = provider.get_gas_price().await.unwrap();

    let tx = TransactionRequest {
        from: Some(owner),
        to: Some(convex_sys.into()),
        value: None,
        gas_price: Some(gas_price),
        nonce: Some(nonce),
        gas: Some(28_000_000u64.into()),
        data: Some(shutdown),
        chain_id: Some(1.into()),
    };

    let _result = api.call(tx.into(), Some(BlockId::Number(14445961.into())), None).await.unwrap();
}

pub async fn ipc_measure_system_shutdown<Fut>(
    spawn_function: impl Fn() -> Fut,
) -> Result<f64, Box<dyn std::error::Error>>
where
    Fut: Future<
            Output = Result<(Arc<Provider<Ipc>>, EthApi, NodeHandle), Box<dyn std::error::Error>>,
        > + 'static,
{
    let start = Instant::now();
    let (provider, api, handle) = (spawn_function)().await?;
    ipc_system_shutdown(provider.clone(), &api).await;
    let duration = start.elapsed();
    shutdown(api, handle).await;
    Ok(duration.as_secs_f64())
}