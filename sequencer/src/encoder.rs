use num_bigint::BigUint;
use serde::Deserialize;
use serde_json::{json, Value};
use std::env;
use std::time::Duration;
use tokio::time::sleep;
use sha3::{Digest, Keccak256};
use hex;

use crate::fields::Fr;

#[derive(Debug, Clone)]
pub struct ContractArtifact {
    pub name: String,
    pub functions: Vec<FunctionArtifact>,
    // TODO: Check if they are needed
    // WARNING: ---> omitted non_dispatch_public_functions, outputs, storage_layout, notes, file_map
}

#[derive(Debug, Clone)]
pub struct FunctionArtifact {
    pub name: String,
    pub parameters: Vec<AbiParameter>,
    pub bytecode: Vec<u8>,
    pub verification_key: Option<String>,
    pub debug_symbols: String,
    pub debug: Option<FunctionDebugMetadata>,
}

#[derive(Debug, Clone)]
pub struct AbiParameter {
    pub name: String,
    pub abi_type: AbiType,
}

impl ToString for AbiType {
    fn to_string(&self) -> String {
        match self {
            AbiType::Field => "field".to_string(),
            AbiType::Boolean => "bool".to_string(),
            AbiType::Array(inner, len) => format!("{}[{}]", inner.to_string(), len),
            AbiType::String(len) => format!("string[{}]", len),
            AbiType::Struct(_) => "struct".to_string(), // Simplify or expand as needed
            AbiType::Integer { signed, width } => {
                if *signed {
                    format!("i{}", width)
                } else {
                    format!("u{}", width)
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionDebugMetadata {}

#[derive(Debug, Clone)]
pub struct FunctionSelector(pub String);

impl FunctionSelector {
    pub fn from_name_and_parameters(name: &str, params: &[AbiParameter]) -> Self {
        let signature = format!(
            "{}({})",
            name,
            params.iter()
                .map(|p| p.abi_type.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        let mut hasher = Keccak256::new();
        hasher.update(signature.as_bytes());
        let hash = hasher.finalize();
        FunctionSelector(hex::encode(&hash[..4]))
    }
}

pub fn get_function_artifact(
    artifact: &ContractArtifact,
    function_name_or_selector: &str,
) -> Result<FunctionArtifact, String> {
    // Match by name
    if let Some(fn_artifact) = artifact.functions.iter().find(|f| f.name == function_name_or_selector) {
        return Ok(fn_artifact.clone());
    }

    // Match by selector
    for f in &artifact.functions {
        let selector = FunctionSelector::from_name_and_parameters(&f.name, &f.parameters);
        if selector.0 == function_name_or_selector {
            return Ok(f.clone());
        }
    }

    Err(format!("Unknown function '{}'.", function_name_or_selector))
}


#[derive(Debug, Clone)]
pub struct FunctionParameter {
    pub name: String,
    pub param_type: AbiType,
}

#[derive(Debug, Clone)]
pub struct FunctionAbi {
    pub name: String,
    pub parameters: Vec<FunctionParameter>,
}

pub struct ArgumentEncoder {
    abi: FunctionAbi,
    args: Vec<Value>,
    pub flattened: Vec<Fr>,
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

impl ArgumentEncoder {
    pub fn new(abi: FunctionAbi, args: Vec<Value>) -> Self {
        Self {
            abi,
            args,
            flattened: Vec::new(),
        }
    }

    pub fn encode(&mut self) -> Result<Vec<Fr>, String> {
        let params = self.abi.parameters.clone();
        let args = self.args.clone();
    
        for (i, param) in params.iter().enumerate() {
            self.encode_argument(&param.param_type, &args[i], Some(&param.name))?;
        }
    
        Ok(self.flattened.clone())
    }

    fn encode_argument(&mut self, abi_type: &AbiType, arg: &Value, name: Option<&str>) -> Result<(), String> {
        match abi_type {
            AbiType::Field => {
                if arg.is_number() {
                    let num = arg.as_u64().ok_or("Invalid number")?;
                    self.flattened.push(Fr::from_u8(num as u8));
                } else if arg.is_string() {
                    let s = arg.as_str().unwrap();
                    self.flattened.push(Fr::from_str(s));
                } else if arg.is_boolean() {
                    self.flattened.push(Fr::from_u8(if arg.as_bool().unwrap() { 1 } else { 0 }));
                } else {
                    return Err(format!("Unsupported Field arg: {:?}", arg));
                }
            }
            AbiType::Boolean => {
                self.flattened.push(Fr::from_u8(if arg.as_bool().unwrap() { 1 } else { 0 }));
            }
            AbiType::Array(inner_type, len) => {
                let arr = arg.as_array().ok_or("Expected array")?;
                if arr.len() != *len {
                    return Err(format!("Array length mismatch for {}", name.unwrap_or("unknown")));
                }

                for (i, elem) in arr.iter().enumerate() {
                    self.encode_argument(inner_type, elem, Some(&format!("{}[{}]", name.unwrap_or("arr"), i)))?;
                }
            }
            AbiType::String(len) => {
                let string = arg.as_str().ok_or("Expected string")?;
                for i in 0..*len {
                    let char = string.chars().nth(i).unwrap_or('\0');
                    self.flattened.push(Fr::from_u8(char as u8));
                }
            }
            AbiType::Struct(fields) => {
                let obj = arg.as_object().ok_or("Expected object for struct")?;
                for field in fields {
                    let field_val = obj.get(&field.name).ok_or("Missing struct field")?;
                    self.encode_argument(&field.field_type, field_val, Some(&field.name))?;
                }
            }
            AbiType::Integer { signed: _, width: _ } => {
                if arg.is_string() {
                    let val = BigUint::parse_bytes(arg.as_str().unwrap().as_bytes(), 10)
                        .ok_or("Invalid string bigint")?;
                    self.flattened.push(Fr(val));
                } else if arg.is_number() {
                    self.flattened.push(Fr::from_u8(arg.as_u64().unwrap() as u8));
                } else {
                    return Err("Unsupported integer input".into());
                }
            }
        }
        Ok(())
    }
}

pub fn encode_arguments(abi: FunctionAbi, args: Vec<Value>) -> Result<Vec<Fr>, String> {
    ArgumentEncoder::new(abi, args).encode()
}
