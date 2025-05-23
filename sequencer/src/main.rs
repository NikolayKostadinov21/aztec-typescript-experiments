use num_bigint::BigUint;
use serde_json::Value;

mod aztec_rpc_client;
mod utils;
// mod contract_function_interaction;
// mod update_feeds;

use crate::contract_function_interaction::ContractFunctionInteraction;
use aztec_rpc_client::{setup_sandbox, AztecRpcClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pxe = setup_sandbox().await?;
    println!("Hello, world!");
    let block = pxe.get_block_number().await?;
    println!("Current PXE block: {}", block);
    let contract_metadata = pxe.get_contract_metadata().await?;
    println!("contract_metadata: {:x?}", contract_metadata);

    println!("===============================");
    println!("===============================");
    println!("===============================");

    let just_field = 1u32;
    let _contract_metadata = pxe
        .send_tx_set_feeds(
            "0x154307e2c5e6b146106ad12642a7a1abef01990b0bc68b21c0de67267a705344",
            "0x12d8f70092c1d4b2bf3ddd60af8e47c1a10d90f3f31fe4c874d4b91f58442ede",
            "set_just_field",
            vec![Value::String(just_field.to_string())],
        )
        .await?;
    Ok(())
}
