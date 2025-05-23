use crate::aztec_rpc_client::AztecRpcClient;
use serde_json::{json, Value};
use std::error::Error;

use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

#[derive(Debug, Serialize, Deserialize)]
pub struct TxExecutionRequest {
    pub origin: String,

    #[serde(rename = "functionSelector")]
    pub function_selector: String,

    pub args: Vec<String>,

    #[serde(rename = "firstCallArgsHash")]
    pub first_call_args_hash: String,

    #[serde(rename = "txContext")]
    pub tx_context: Value,

    #[serde(rename = "authWitnesses")]
    pub auth_witnesses: Vec<Value>,

    #[serde(rename = "argsOfCalls")]
    pub args_of_calls: Vec<Value>,

    pub capsules: Vec<Value>,
}

impl<'a> ContractFunctionInteraction<'a> {
    pub async fn create_tx_execution_request(
        &self,
    ) -> Result<TxExecutionRequest, Box<dyn std::error::Error>> {
        println!("goes into create_tx_execution_request");
        use hex;
        use sha3::{Digest, Keccak256};

        println!("self.function_name: {:#}", self.function_name);
        println!("self.args: {:x?}", self.args);
        let function_signature = format!("{}({:x?})", self.function_name, self.args);
        println!("proba {:#}", function_signature);
        let mut hasher = Keccak256::new();
        hasher.update(function_signature.as_bytes());
        let function_selector = hex::encode(&hasher.finalize()[..4]);

        let dummy_hash =
            "0x0000000000000000000000000000000000000000000000000000000000000000".to_string();
        let tx_context = json!({
            "chainId": "0x1",
            "version": "0x1",
            "isSimulation": true,
            "isFeePaying": false,
            "gasSettings": {
                "feePerGas": "0x00",
                "gasLimit": "0x100000",

                "gasLimits": {
                    "tx": "0x1000",
                    "call": "0x1000",
                    "constructor": "0x1000",
                    "daGas": "0x1000",
                    "l2Gas": "0x1000"
                },

                "teardownGasLimits": {
                    "unconstrained": "0x1000",
                    "verification": "0x1000",
                    "daGas": "0x1000",
                    "l2Gas": "0x1000"
                },

                "maxFeesPerGas": {
                    "tx": "0x00",
                    "call": "0x00",
                    "constructor": "0x00",
                    "feePerDaGas": "0x00",
                    "feePerL2Gas": "0x00"
                },

                "maxPriorityFeesPerGas": {
                    "tx": "0x00",
                    "call": "0x00",
                    "constructor": "0x00",
                    "feePerDaGas": "0x00",
                    "feePerL2Gas": "0x00"
                }
            }
        });

        let tx = TxExecutionRequest {
            origin: self.wallet_address.clone(),
            function_selector: "0x5959152a".to_string(),
            args: self.args.iter().map(|v| v.to_string()).collect(),
            first_call_args_hash: dummy_hash,
            tx_context,
            auth_witnesses: vec![],
            args_of_calls: vec![],
            capsules: vec![],
        };

        Ok(tx)
    }
}

pub struct ContractFunctionInteraction<'a> {
    pub wallet_address: String,
    pub contract_address: String,
    pub function_name: String,
    pub args: Vec<Value>,
    pub pxe: &'a AztecRpcClient,
}

impl<'a> ContractFunctionInteraction<'a> {
    pub async fn send(&self) -> Result<Value, Box<dyn Error>> {
        // Step 1: Construct the txRequest
        let tx_request = json!({
            "to": self.contract_address,
            "from": self.wallet_address,
            "functionName": self.function_name,
            "args": self.args,
        });

        let tx_request = self.create_tx_execution_request().await?;
        let tx_json = serde_json::to_value(&tx_request)?;
        println!("tx_json: {:?}", tx_json);

        let simulation_args = vec![
            tx_json,
            json!(true),
            Value::Null,
            json!(false),
            json!(false),
            Value::Null,
        ];

        println!("before simulation_result");
        println!("simulateTx args: {:?}", simulation_args);
        tracing::info!("simulateTx args: {:?}", simulation_args);
        let simulation_result: Value = self.pxe.request("simulateTx", simulation_args).await?;
        tracing::info!("simulation_result: {:?}", simulation_result);
        println!("simulation_result: {:#}", simulation_result);
        println!("!!!!!");

        let private_execution_result = simulation_result["privateExecutionResult"].clone();
        println!("private_execution_result: {:#}", private_execution_result);
        println!("!!!!!");

        // Step 3: Prove the transaction
        let proving_payload = json!({
            "txRequest": tx_request,
            "privateExecutionResult": private_execution_result,
        });

        let proving_result: Value = self.pxe.request("proveTx", vec![proving_payload]).await?;
        println!("proving_result: {:#}", proving_result);
        println!("!!!!!");
        let tx = proving_result["tx"].clone();
        println!("tx: {:#}", tx);
        println!("!!!!!");

        // Step 4: Send the transaction
        let send_result: Value = self.pxe.request("sendTx", vec![tx]).await?;
        println!("send_result: {:#}", send_result);
        println!("!!!!!");

        Ok(send_result)
    }
}
