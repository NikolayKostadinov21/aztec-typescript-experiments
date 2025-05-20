use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::time::Duration;
use tokio::time::sleep;
use num_bigint::BigUint;
use num_bigint::ParseBigIntError;

#[derive(Debug, Deserialize)]
pub struct RpcResponse<T> {
    pub jsonrpc: String,
    pub id: u32,
    pub result: Option<T>,
    pub error: Option<serde_json::Value>,
}

const PXE_URL: &str = "http://localhost:8080";

#[derive(Debug)]
pub struct AztecRpcClient {
    host: String,
    namespace: Option<String>,
    client: reqwest::Client,
}

pub async fn setup_sandbox() -> Result<AztecRpcClient, Box<dyn std::error::Error>> {
    let pxe_url = env::var("PXE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let pxe = AztecRpcClient::new(pxe_url, Some("pxe".to_string()));

    wait_for_pxe(
        || async {
            let _: serde_json::Value = pxe.request("getNodeInfo", vec![]).await?;
            Ok(())
        },
        10,
        Duration::from_secs(2),
    )
    .await?;

    Ok(pxe)
}

pub async fn wait_for_pxe<F, Fut>(
    check_fn: F,
    max_attempts: u32,
    delay: Duration,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
    for attempt in 1..=max_attempts {
        match check_fn().await {
            Ok(_) => {
                println!("PXE is online!");
                return Ok(());
            }
            Err(err) => {
                println!("Attempt {}/{}: PXE not ready ({})", attempt, max_attempts, err);
                sleep(delay).await;
            }
        }
    }

    Err("PXE did not respond in time".into())
}

impl AztecRpcClient {
    pub fn new(host: impl Into<String>, namespace: Option<String>) -> Self {
        AztecRpcClient {
            host: host.into(),
            namespace,
            client: reqwest::Client::new(),
        }
    }

    pub async fn request<T: for<'de> serde::Deserialize<'de> + std::fmt::Debug>(
        &self,
        method: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let full_method = if let Some(ns) = &self.namespace {
            format!("{}_{}", ns, method)
        } else {
            method.to_string()
        };

        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": full_method,
            "params": params,
        });

        let client = &self.client;
        let response = client.post(&self.host).json(&payload).send().await?;
        let text = response.text().await?;

        // println!("RPC raw response: {}", text);

        let rpc_response: RpcResponse<T> = serde_json::from_str(&text)?;

        if let Some(err) = rpc_response.error {
            return Err(format!("PXE returned error: {}", err).into());
        }

        rpc_response.result.ok_or("Missing `result` field in RPC response".into())
    }
    

    pub async fn get_block_number(&self) -> Result<u64, Box<dyn std::error::Error>> {
        self.request("getBlockNumber", vec![]).await
    }

    pub async fn get_contracts(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        self.request("getContracts", vec![]).await
    }

    pub async fn get_contract_metadata(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let value: Value = self.request(
            "getContractMetadata",
            vec![json!("0x04bfd3ad859c1da7e45740d58ef55bd2195c20a63a383b460369f813ecfc1a24")]
        ).await?;
    
        Ok(value)
    }

    pub async fn simulate_tx_set_feeds(
        &self,
        from_address: &str,
        to_contract_address: &str,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let payload = json!({
            "from": from_address,
            "to": to_contract_address,
            "functionName": function_name,
            "args": args
        });
        println!("goes to here");

        let result: Value = self.request("simulateTx", vec![payload]).await?;

        Ok(result)
    }
}
