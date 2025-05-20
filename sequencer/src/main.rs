use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::time::Duration;
use tokio::time::sleep;
use std::str::FromStr;
use num_bigint::BigUint;
use std::fs;

mod aztec_rpc_client;
mod contract_function_interaction;
mod update_feeds;

use aztec_rpc_client::{AztecRpcClient, setup_sandbox};
use crate::contract_function_interaction::ContractFunctionInteraction;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pxe = setup_sandbox().await?;
    println!("Hello, world!");
    let block = pxe.get_block_number().await?;
    println!("Current PXE block: {}", block);
    let contract_metadata = pxe.get_contract_metadata().await?;
    println!("contract_metadata: {:x?}", contract_metadata);
    Ok(())
}
