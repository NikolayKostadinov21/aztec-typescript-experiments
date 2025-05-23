use num_bigint::BigUint;
use serde::Deserialize;
use serde_json::{json, Value};
use std::env;
use std::time::Duration;
use tokio::time::sleep;
use crate::fields::Fr;

#[derive(Debug, Deserialize)]
pub struct RpcResponse<T> {
    pub jsonrpc: String,
    pub id: u32,
    pub result: Option<T>,
    pub error: Option<serde_json::Value>,
}

struct ArgumentEncoder {
    pub flattened: Vec<Fr>,
}

struct Argument {
    _type: String,
    value: Fr
}

#[derive(Debug, Clone)]
pub enum AbiType {
    Field,
    Boolean,
    Array(Box<AbiType>, usize),
    String(usize),
    Struct(Vec<StructField>),
    Integer { signed: bool, width: usize },
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub field_type: AbiType,
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
                println!(
                    "Attempt {}/{}: PXE not ready ({})",
                    attempt, max_attempts, err
                );
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

        rpc_response
            .result
            .ok_or("Missing `result` field in RPC response".into())
    }

    pub async fn get_block_number(&self) -> Result<u64, Box<dyn std::error::Error>> {
        self.request("getBlockNumber", vec![]).await
    }

    pub async fn get_contracts(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        self.request("getContracts", vec![]).await
    }

    pub async fn get_contract_metadata(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let value: Value = self
            .request(
                "getContractMetadata",
                vec![json!(
                    "0x12d8f70092c1d4b2bf3ddd60af8e47c1a10d90f3f31fe4c874d4b91f58442ede"
                )],
            )
            .await?;

        if let Some(contract_instance) = value.get("contractInstance").and_then(|v| v.as_object()) {
            println!("Contract address: {}", contract_instance["address"]);
            println!(
                "Contract class ID: {}",
                contract_instance["currentContractClassId"]
            );
        } else {
            println!("Could not extract contractInstance.");
        }

        Ok(value)
    }

    pub async fn send_tx_set_feeds(
        &self,
        _from_address: &str,
        _to_contract_address: &str,
        _function_name: &str,
        _args: Vec<Value>,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let _gas_limits_da_gas = 1000000000;
        let _gas_limits_l2_gas = 1000000000;

        let _teardown_limits_da_gas = 6000000;
        let _teardown_limits_l2_gas = 6000000;

        let tx_execution_request = json!({
          "jsonrpc": "2.0",
          "id": 9,
          "method": "pxe_simulateTx",
          "params": [
            {
              "origin": "0x154307e2c5e6b146106ad12642a7a1abef01990b0bc68b21c0de67267a705344",
              "functionSelector": "0x27e740b2",
              "firstCallArgsHash": "0x11f1fc3d3ffa64fccd5dc340dd3991395969b30b08306a563e42e2085138abda",
              "txContext": {
                "gasSettings": {
                  "gasLimits": { "daGas": 1000000000, "l2Gas": 1000000000 },
                  "teardownGasLimits": { "daGas": 6000000, "l2Gas": 6000000 },
                  "maxFeesPerGas": {
                    "feePerDaGas": "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "feePerL2Gas": "0x0000000000000000000000000000000000000000000000000000000000002aa8"
                  },
                  "maxPriorityFeesPerGas": {
                    "feePerDaGas": "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "feePerL2Gas": "0x0000000000000000000000000000000000000000000000000000000000000000"
                  }
                },
                "chainId": "0x0000000000000000000000000000000000000000000000000000000000007a69",
                "version": "0x00000000000000000000000000000000000000000000000000000000b2da7e95"
              },
              "argsOfCalls": [
                {
                  "values": [
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000"
                  ],
                  "hash": "0x2ff90a8a1f6c3253957f7864dfdf12ec0eef9006c26cdf58dab6a170b5b7dd1c"
                },
                {
                  "values": [
                    "0x0000000000000000000000000000000000000000000000000000000017f12888"
                  ],
                  "hash": "0x0825a9b29181eef01b503945a4268c1d9f7714782fd4d8383a9c6257066df693"
                },
                {
                  "values": [
                    "0x0000000000000000000000000000000000000000000000000000000000000000"
                  ],
                  "hash": "0x2032c19437941846a704c8b191e823c8074b38114a03a22e93020ef6f7688b4d"
                },
                {
                  "values": [
                    "0x0000000000000000000000000000000000000000000000000000000000000000"
                  ],
                  "hash": "0x2032c19437941846a704c8b191e823c8074b38114a03a22e93020ef6f7688b4d"
                },
                {
                  "values": [
                    "0x0000000000000000000000000000000000000000000000000000000000000000"
                  ],
                  "hash": "0x2032c19437941846a704c8b191e823c8074b38114a03a22e93020ef6f7688b4d"
                },
                {
                  "values": [
                    "0x0000000000000000000000000000000000000000000000000000000000000000"
                  ],
                  "hash": "0x2032c19437941846a704c8b191e823c8074b38114a03a22e93020ef6f7688b4d"
                },
                {
                  "values": [
                    "0x2ff90a8a1f6c3253957f7864dfdf12ec0eef9006c26cdf58dab6a170b5b7dd1c",
                    "0x0000000000000000000000000000000000000000000000000000000000c02957",
                    "0x0000000000000000000000000000000000000000000000000000000000000002",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0825a9b29181eef01b503945a4268c1d9f7714782fd4d8383a9c6257066df693",
                    "0x0000000000000000000000000000000000000000000000000000000017f12888",
                    "0x044b9be988489338e14b0ab349a6d6b5e47b329b0fd2cc9a0a373ba2ddd676b2",
                    "0x0000000000000000000000000000000000000000000000000000000000000001",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x2032c19437941846a704c8b191e823c8074b38114a03a22e93020ef6f7688b4d",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000001",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x2032c19437941846a704c8b191e823c8074b38114a03a22e93020ef6f7688b4d",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000001",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x084691ec849079122dbf0b59d4831ca107e46d444270f9fe80355efc37ec5a74",
                    "0x2032c19437941846a704c8b191e823c8074b38114a03a22e93020ef6f7688b4d",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000001",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x2032c19437941846a704c8b191e823c8074b38114a03a22e93020ef6f7688b4d",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x0000000000000000000000000000000000000000000000000000000000000001",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "0x2c1dbbf61cd800fc996d6bf52dd4acb34e659a2d09946dc5e9721ca3b97a067d",
                    "0x0000000000000000000000000000000000000000000000000000000000000001",
                    "0x0000000000000000000000000000000000000000000000000000000000000000"
                  ],
                  "hash": "0x11f1fc3d3ffa64fccd5dc340dd3991395969b30b08306a563e42e2085138abda"
                }
              ],
              "authWitnesses": [
                "0x239041351450551a45e86e62eadc39d99960e37b07c7ef9b2a08de24f860efc500000040000000000000000000000000000000000000000000000000000000000000002e000000000000000000000000000000000000000000000000000000000000008d000000000000000000000000000000000000000000000000000000000000007e000000000000000000000000000000000000000000000000000000000000003e00000000000000000000000000000000000000000000000000000000000000f1000000000000000000000000000000000000000000000000000000000000008700000000000000000000000000000000000000000000000000000000000000cd00000000000000000000000000000000000000000000000000000000000000a200000000000000000000000000000000000000000000000000000000000000cc000000000000000000000000000000000000000000000000000000000000003900000000000000000000000000000000000000000000000000000000000000bc00000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000003c00000000000000000000000000000000000000000000000000000000000000e300000000000000000000000000000000000000000000000000000000000000b600000000000000000000000000000000000000000000000000000000000000ae00000000000000000000000000000000000000000000000000000000000000140000000000000000000000000000000000000000000000000000000000000065000000000000000000000000000000000000000000000000000000000000002400000000000000000000000000000000000000000000000000000000000000b800000000000000000000000000000000000000000000000000000000000000fc000000000000000000000000000000000000000000000000000000000000006d000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000af00000000000000000000000000000000000000000000000000000000000000ab00000000000000000000000000000000000000000000000000000000000000940000000000000000000000000000000000000000000000000000000000000053000000000000000000000000000000000000000000000000000000000000008b00000000000000000000000000000000000000000000000000000000000000a40000000000000000000000000000000000000000000000000000000000000013000000000000000000000000000000000000000000000000000000000000005b0000000000000000000000000000000000000000000000000000000000000036000000000000000000000000000000000000000000000000000000000000003400000000000000000000000000000000000000000000000000000000000000ab00000000000000000000000000000000000000000000000000000000000000dc00000000000000000000000000000000000000000000000000000000000000a5000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000a500000000000000000000000000000000000000000000000000000000000000f4000000000000000000000000000000000000000000000000000000000000007d00000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000000b100000000000000000000000000000000000000000000000000000000000000d90000000000000000000000000000000000000000000000000000000000000056000000000000000000000000000000000000000000000000000000000000009d00000000000000000000000000000000000000000000000000000000000000ea000000000000000000000000000000000000000000000000000000000000000f00000000000000000000000000000000000000000000000000000000000000ed00000000000000000000000000000000000000000000000000000000000000d60000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000005b000000000000000000000000000000000000000000000000000000000000005e00000000000000000000000000000000000000000000000000000000000000a2000000000000000000000000000000000000000000000000000000000000004200000000000000000000000000000000000000000000000000000000000000f0000000000000000000000000000000000000000000000000000000000000003800000000000000000000000000000000000000000000000000000000000000b500000000000000000000000000000000000000000000000000000000000000bc0000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000005c000000000000000000000000000000000000000000000000000000000000005200000000000000000000000000000000000000000000000000000000000000b900000000000000000000000000000000000000000000000000000000000000d10000000000000000000000000000000000000000000000000000000000000097"
              ],
              "capsules": []
            },
            true,
            null,
            true,
            null
          ]
        });
        let _payload = json!({
            "txRequest": tx_execution_request,
            "simulatePublic": true,
            "msgSender": "0x154307e2c5e6b146106ad12642a7a1abef01990b0bc68b21c0de67267a705344",
            "skipTxValidation": false,
            "skipFeeEnforcement": false,
            "scopes": [],
        });
        println!("goes to here");

        println!(
            "tx_execution_request: {:?} ",
            tx_execution_request["params"].as_array().unwrap().to_vec()
        );
        let result: Value = self
            .request(
                "simulateTx",
                tx_execution_request["params"].as_array().unwrap().to_vec(),
            )
            .await?;

        Ok(result)
    }
}

impl ArgumentEncoder {
    pub fn encode_argument(
        &mut self,
        abi_type: &AbiType,
        arg: &Value,
        name: Option<&str>,
    ) -> Result<(), String> {
        match abi_type {
            AbiType::Field => {
                if let Some(num) = arg.as_u64() {
                    self.flattened.push(Fr::from_u8(num as u8));
                } else if let Some(s) = arg.as_str() {
                    self.flattened.push(Fr::from_str(s));
                } else if let Some(b) = arg.as_bool() {
                    self.flattened.push(Fr::from_u8(if b { 1 } else { 0 }));
                } else {
                    return Err(format!("Unsupported Field arg: {:?}", arg));
                }
            }

            AbiType::Boolean => {
                let b = arg.as_bool().ok_or("Expected boolean")?;
                self.flattened.push(Fr::from_u8(if b { 1 } else { 0 }));
            }

            AbiType::Array(inner_type, len) => {
                let arr = arg.as_array().ok_or("Expected array")?;
                if arr.len() != *len {
                    return Err(format!(
                        "Array length mismatch for {}",
                        name.unwrap_or("unknown")
                    ));
                }
                for (i, elem) in arr.iter().enumerate() {
                    self.encode_argument(inner_type, elem, Some(&format!("{}[{}]", name.unwrap_or("array"), i)))?;
                }
            }

            AbiType::String(len) => {
                let string = arg.as_str().ok_or("Expected string")?;
                for i in 0..*len {
                    let ch = string.chars().nth(i).unwrap_or('\0');
                    self.flattened.push(Fr::from_u8(ch as u8));
                }
            }

            AbiType::Struct(fields) => {
                let obj = arg.as_object().ok_or("Expected object for struct")?;
                for field in fields {
                    let field_val = obj
                        .get(&field.name)
                        .ok_or_else(|| format!("Missing field {}", field.name))?;
                    self.encode_argument(&field.field_type, field_val, Some(&field.name))?;
                }
            }

            AbiType::Integer { signed: _, width: _ } => {
                if let Some(s) = arg.as_str() {
                    let val = BigUint::parse_bytes(s.as_bytes(), 10)
                        .ok_or("Invalid string bigint")?;
                    self.flattened.push(Fr::from_biguint(val));
                } else if let Some(n) = arg.as_u64() {
                    self.flattened.push(Fr::from_u8(n as u8));
                } else {
                    return Err("Unsupported integer input".into());
                }
            }
        }
        Ok(())
    }
}
