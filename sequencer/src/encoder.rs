use num_bigint::BigUint;
use serde_json::{json, Value};
use sha3::{Digest, Keccak256};
use crate::fields::Fr;

#[derive(Debug, Clone)]
pub struct ContractArtifact {
    pub name: String,
    pub functions: Vec<FunctionArtifact>,
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

#[derive(Debug, Clone)]
pub struct FunctionDebugMetadata;

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
    if let Some(fn_artifact) = artifact.functions.iter().find(|f| f.name == function_name_or_selector) {
        return Ok(fn_artifact.clone());
    }

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

impl ToString for AbiType {
    fn to_string(&self) -> String {
        match self {
            AbiType::Field => "field".to_string(),
            AbiType::Boolean => "bool".to_string(),
            AbiType::Array(inner, len) => format!("{}[{}]", inner.to_string(), len),
            AbiType::String(len) => format!("string[{}]", len),
            AbiType::Struct(_) => "struct".to_string(),
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
                    self.flattened.push(Fr(BigUint::from(num)));
                } else if arg.is_string() {
                    let s = arg.as_str().unwrap();
                    let num = BigUint::parse_bytes(s.as_bytes(), 10).ok_or("Invalid field string")?;
                    self.flattened.push(Fr(num));
                } else if arg.is_boolean() {
                    self.flattened.push(Fr(BigUint::from(if arg.as_bool().unwrap() { 1u8 } else { 0u8 })));
                } else {
                    return Err(format!("Unsupported Field arg: {:?}", arg));
                }
            }
            AbiType::Boolean => {
                self.flattened.push(Fr(BigUint::from(if arg.as_bool().unwrap() { 1u8 } else { 0u8 })));
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
                    self.flattened.push(Fr(BigUint::from(char as u8)));
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
                    self.flattened.push(Fr(BigUint::from(arg.as_u64().unwrap())));
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


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_set_just_field_encoding() {
        use num_bigint::BigUint;
    
        let function_abi = FunctionAbi {
            name: "set_just_field".to_string(),
            parameters: vec![
                FunctionParameter {
                    name: "value".to_string(),
                    param_type: AbiType::Field,
                },
            ],
        };
    
        let args = vec![json!(123456789u64)];
    
        let encoded = encode_arguments(function_abi, args).expect("Encoding should succeed");
    
        assert_eq!(encoded.len(), 1);
        assert_eq!(encoded[0], Fr(BigUint::from(123456789u64)));
    }

    #[test]
    fn test_nested_struct_encoding() {
        let abi = FunctionAbi {
            name: "nested_struct".to_string(),
            parameters: vec![
                FunctionParameter {
                    name: "nested".to_string(),
                    param_type: AbiType::Struct(vec![
                        StructField {
                            name: "a".to_string(),
                            field_type: AbiType::Field,
                        },
                        StructField {
                            name: "b".to_string(),
                            field_type: AbiType::Struct(vec![
                                StructField {
                                    name: "x".to_string(),
                                    field_type: AbiType::Field,
                                },
                                StructField {
                                    name: "y".to_string(),
                                    field_type: AbiType::Field,
                                },
                            ]),
                        },
                    ]),
                }
            ],
        };

        let args = vec![json!({
            "a": 123123,
            "b": {
                "x": 456456,
                "y": 789789
            }
        })];

        let encoded = encode_arguments(abi, args).unwrap();
        assert_eq!(encoded[0], Fr::from_u64(123123));
        assert_eq!(encoded[1], Fr::from_u64(456456));
        assert_eq!(encoded[2], Fr::from_u64(789789));
    }

    #[test]
    fn test_integer_in_struct() {
        let abi = FunctionAbi {
            name: "integer_struct".to_string(),
            parameters: vec![FunctionParameter {
                name: "value".to_string(),
                param_type: AbiType::Struct(vec![
                    StructField {
                        name: "int".to_string(),
                        field_type: AbiType::Integer {
                            signed: false,
                            width: 64,
                        },
                    },
                ]),
            }],
        };

        let args = vec![json!({ "int": "9876543210" })];
        let encoded = encode_arguments(abi, args).unwrap();
        assert_eq!(encoded[0], Fr::from_str("9876543210"));
    }

    #[test]
    fn test_string_encoding() {
        let abi = FunctionAbi {
            name: "set_name".to_string(),
            parameters: vec![FunctionParameter {
                name: "name".to_string(),
                param_type: AbiType::String(5),
            }],
        };

        let args = vec![json!("Bob")];
        let encoded = encode_arguments(abi, args).unwrap();
        assert_eq!(encoded[0], Fr::from_u8(b'B'));
        assert_eq!(encoded[1], Fr::from_u8(b'o'));
        assert_eq!(encoded[2], Fr::from_u8(b'b'));
        assert_eq!(encoded[3], Fr::from_u8(0));
        assert_eq!(encoded[4], Fr::from_u8(0));
    }

    #[test]
    fn test_array_of_structs() {
        let abi = FunctionAbi {
            name: "update_points".to_string(),
            parameters: vec![FunctionParameter {
                name: "points".to_string(),
                param_type: AbiType::Array(Box::new(AbiType::Struct(vec![
                    StructField {
                        name: "x".to_string(),
                        field_type: AbiType::Field,
                    },
                    StructField {
                        name: "y".to_string(),
                        field_type: AbiType::Field,
                    },
                ])), 2),
            }],
        };

        let args = vec![json!([
            { "x": 10, "y": 20 },
            { "x": 30, "y": 40 }
        ])];

        let encoded = encode_arguments(abi, args).unwrap();
        assert_eq!(encoded[0], Fr::from_u64(10));
        assert_eq!(encoded[1], Fr::from_u64(20));
        assert_eq!(encoded[2], Fr::from_u64(30));
        assert_eq!(encoded[3], Fr::from_u64(40));
    }

    #[test]
    fn test_function_selector() {
        let fn_params = vec![
            FunctionParameter {
                name: "flag".to_string(),
                param_type: AbiType::Boolean,
            },
            FunctionParameter {
                name: "value".to_string(),
                param_type: AbiType::Field,
            },
        ];
    
        let abi_params: Vec<AbiParameter> = fn_params
            .iter()
            .map(|p| AbiParameter {
                name: p.name.clone(),
                abi_type: p.param_type.clone(),
            })
            .collect();
    
        let selector = FunctionSelector::from_name_and_parameters("do_action", &abi_params);
        assert_eq!(selector.0.len(), 8);
    }

    #[test]
    fn test_encode_single_field_u64() {
        let abi = FunctionAbi {
            name: "test_fn".to_string(),
            parameters: vec![FunctionParameter {
                name: "value".to_string(),
                param_type: AbiType::Field,
            }],
        };
        let args = vec![json!(42)];
        let encoded = encode_arguments(abi, args).unwrap();
        assert_eq!(encoded.len(), 1);
        assert_eq!(encoded[0], Fr::from_u8(42));
    }

    #[test]
    fn test_encode_boolean() {
        let abi = FunctionAbi {
            name: "test_bool".to_string(),
            parameters: vec![FunctionParameter {
                name: "flag".to_string(),
                param_type: AbiType::Boolean,
            }],
        };
        let args = vec![json!(true)];
        let encoded = encode_arguments(abi, args).unwrap();
        assert_eq!(encoded.len(), 1);
        assert_eq!(encoded[0], Fr::from_u8(1));
    }

    #[test]
    fn test_encode_array() {
        let abi = FunctionAbi {
            name: "test_array".to_string(),
            parameters: vec![FunctionParameter {
                name: "arr".to_string(),
                param_type: AbiType::Array(Box::new(AbiType::Field), 3),
            }],
        };
        let args = vec![json!([1, 2, 3])];
        let encoded = encode_arguments(abi, args).unwrap();
        assert_eq!(encoded.len(), 3);
        assert_eq!(encoded[0], Fr::from_u8(1));
        assert_eq!(encoded[1], Fr::from_u8(2));
        assert_eq!(encoded[2], Fr::from_u8(3));
    }

    #[test]
    fn test_encode_struct() {
        let abi = FunctionAbi {
            name: "test_struct".to_string(),
            parameters: vec![FunctionParameter {
                name: "data".to_string(),
                param_type: AbiType::Struct(vec![
                    StructField {
                        name: "a".to_string(),
                        field_type: AbiType::Field,
                    },
                    StructField {
                        name: "b".to_string(),
                        field_type: AbiType::Boolean,
                    },
                ]),
            }],
        };
        let args = vec![json!({ "a": 7, "b": false })];
        let encoded = encode_arguments(abi, args).unwrap();
        assert_eq!(encoded.len(), 2);
        assert_eq!(encoded[0], Fr::from_u8(7));
        assert_eq!(encoded[1], Fr::from_u8(0));
    }

    #[test]
    fn test_encode_integer_from_string() {
        let abi = FunctionAbi {
            name: "test_integer".to_string(),
            parameters: vec![FunctionParameter {
                name: "int_val".to_string(),
                param_type: AbiType::Integer {
                    signed: false,
                    width: 32,
                },
            }],
        };
        let args = vec![json!("12345678901234567890")];
        let encoded = encode_arguments(abi, args).unwrap();
        assert_eq!(encoded.len(), 1);
        assert_eq!(encoded[0], Fr::from_str("12345678901234567890"));
    }
}
