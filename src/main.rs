mod bindings;
use std::sync::Arc;
use std::time::Instant;

use anvil::{eth::EthApi, spawn, NodeConfig, NodeHandle};
use anvil_benchmarks::{spawn_http_local, spawn_http_external, spawn_ethers_reth, spawn_ipc, http_measure_system_shutdown, ipc_measure_system_shutdown};
use bindings::convex::ShutdownSystemCall;
use ethers::{abi::AbiEncode, prelude::*};
use ndarray::Array1;
use ndarray_stats::QuantileExt;
use std::{env, future::Future};
use std::sync::atomic::{AtomicU8, Ordering};



#[tokio::main]
async fn main() {
    const NUM_ITERATIONS: usize = 10;

    let durations_http_local = collect_duration_http(NUM_ITERATIONS, spawn_http_local).await;
    print_statistics("http local fork", &durations_http_local);

    let durations_ipc = collect_duration_ipc(NUM_ITERATIONS, spawn_ipc).await;
    print_statistics("Ipc fork", &durations_ipc);

    let durations_ethers_reth = collect_duration_ipc(NUM_ITERATIONS, spawn_ethers_reth).await;
    print_statistics("Ipc ethers_reth fork", &durations_ethers_reth);

}

pub async fn collect_duration_http<F, Fut>(num_iterations: usize, spawn_function: F) -> Vec<f64>
where
    F: Fn() -> Fut,
    Fut: Future<
            Output = Result<(Arc<Provider<Http>>, EthApi, NodeHandle), Box<dyn std::error::Error>>,
        > + 'static,
{
    let mut durations = vec![];
    for _ in 0..num_iterations {
        match http_measure_system_shutdown(&spawn_function).await {
            Ok(duration) => durations.push(duration),
            Err(e) => eprintln!("Error while measuring system shutdown: {}", e),
        }
    }
    durations
}

pub async fn collect_duration_ipc<F, Fut>(num_iterations: usize, spawn_function: F) -> Vec<f64>
where
    F: Fn() -> Fut,
    Fut: Future<
            Output = Result<(Arc<Provider<Ipc>>, EthApi, NodeHandle), Box<dyn std::error::Error>>,
        > + 'static,
{
    let mut durations = vec![];
    for _ in 0..num_iterations {
        match ipc_measure_system_shutdown(&spawn_function).await {
            Ok(duration) => durations.push(duration),
            Err(e) => eprintln!("Error while measuring system shutdown: {}", e),
        }
    }
    durations
}



pub fn print_statistics(label: &str, durations: &Vec<f64>) {
    let array_durations: Array1<f64> = Array1::from(durations.clone());

    let mean_duration = array_durations.mean().unwrap();
    let min_duration = array_durations.min().unwrap();
    let max_duration = array_durations.max().unwrap();

    let sum: f64 = durations.iter().map(|&x| (x - mean_duration).powi(2)).sum();
    let std_dev_duration = (sum / (durations.len() - 1) as f64).sqrt();

    println!("Mean eth_call duration via {}: {} seconds", label, mean_duration);
    println!("Std Dev of eth_call duration via {}: {} seconds", label, std_dev_duration);
    println!("Min eth_call duration via {}: {} seconds", label, min_duration);
    println!("Max eth_call duration via {}: {} seconds", label, max_duration);
}



