use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::Deserialize;
use num_bigint::BigUint;
use serde_json::{json, Value};
use sha3::{Digest, Keccak256};
use crate::fields::Fr;


#[derive(Debug, Clone, Deserialize)]
pub struct ContractArtifact {
    pub name: String,
    pub functions: Vec<FunctionArtifact>,
    #[serde(rename = "nonDispatchPublicFunctions")]
    pub non_dispatch_public_functions: Vec<FunctionAbi>,
    #[serde(rename = "storageLayout")]
    pub storage_layout: HashMap<String, FieldLayout>,
    pub notes: HashMap<String, ContractNote>,
    #[serde(rename = "fileMap")]
    pub file_map: DebugFileMap,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FunctionArtifact {
    pub name: String,
    pub parameters: Vec<AbiParameter>,
    pub bytecode: String,
    #[serde(rename = "verificationKey")]
    pub verification_key: Option<String>,
    #[serde(rename = "debugSymbols")]
    pub debug_symbols: String,
    pub debug: Option<FunctionDebugMetadata>,
    #[serde(rename = "functionType")]
    pub function_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FunctionDebugMetadata {}

#[derive(Debug, Clone, Deserialize)]
pub struct AbiParameter {
    pub name: String,
    #[serde(rename = "type")]
    pub abi_type: AbiType,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FieldLayout {
    pub slot: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContractNote {
    pub id: String,
    #[serde(rename = "type")]
    pub typ: String,
    pub fields: Vec<NoteField>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NoteField {
    pub name: String,
    pub index: usize,
    pub nullable: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DebugFileMap(pub HashMap<String, DebugFile>);

#[derive(Debug, Clone, Deserialize)]
pub struct DebugFile {
    pub source: String,
    pub path: String,
}

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

pub fn get_function_artifact<'a>(
    artifact: &'a ContractArtifact,
    name_or_selector: &str,
) -> Result<&'a FunctionArtifact, String> {
    if let Some(f) = artifact.functions.iter().find(|f| f.name == name_or_selector) {
        return Ok(f);
    }

    for f in &artifact.functions {
        let selector = FunctionSelector::from_name_and_parameters(&f.name, &f.parameters);
        if selector.0 == name_or_selector {
            return Ok(f);
        }
    }

    Err(format!("Unknown function '{}'.", name_or_selector))
}

pub fn load_contract_artifact<P: AsRef<Path>>(path: P) -> Result<ContractArtifact, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let artifact: ContractArtifact = serde_json::from_str(&contents)?;
    Ok(artifact)
}

#[derive(Debug, Clone)]
pub struct FunctionParameter {
    pub name: String,
    pub param_type: AbiType,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FunctionAbi {
    pub name: String,
    #[serde(rename = "functionType")]
    pub function_type: String,
    pub isInternal: bool,
    pub isStatic: bool,
    pub isInitializer: bool,
    pub parameters: Vec<AbiParameter>,
    #[serde(rename = "returnTypes")]
    pub return_types: Vec<AbiType>,
    pub errorTypes: Option<Value>,
}

pub struct ArgumentEncoder {
    abi: FunctionAbi,
    args: Vec<Value>,
    pub flattened: Vec<Fr>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind")]
pub enum AbiType {
    #[serde(rename = "field")]
    Field,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "array")]
    Array { r#type: Box<AbiType>, length: usize },
    #[serde(rename = "string")]
    String { length: usize },
    #[serde(rename = "struct")]
    Struct { fields: Vec<AbiStructField>, path: String },
    #[serde(rename = "integer")]
    Integer { sign: String, width: usize },
}

#[derive(Debug, Clone, Deserialize)]
pub struct AbiStructField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: AbiType,
}

impl ToString for AbiType {
    fn to_string(&self) -> String {
        match self {
            AbiType::Field => "field".to_string(),
            AbiType::Boolean => "bool".to_string(),
            AbiType::Array { r#type, length } => format!("{}[{}]", r#type.to_string(), length),
            AbiType::String { length } => format!("string[{}]", length),
            AbiType::Struct { .. } => "struct".to_string(),
            AbiType::Integer { sign, width } => {
                format!("{}{}", if sign == "unsigned" { "u" } else { "i" }, width)
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Outputs {
    pub structs: HashMap<String, Vec<AbiType>>,
    pub globals: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StructField {
    pub name: String,
    #[serde(rename = "type")]
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
        let parameters = std::mem::take(&mut self.abi.parameters);
        let args = std::mem::take(&mut self.args);
    
        for (i, param) in parameters.into_iter().enumerate() {
            self.encode_argument(&param.abi_type, &args[i], Some(&param.name))?;
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
            AbiType::Array { r#type, length } => {
                let arr = arg.as_array().ok_or("Expected array")?;
                if arr.len() != *length {
                    return Err(format!("Array length mismatch for {}", name.unwrap_or("unknown")));
                }
        
                for (i, elem) in arr.iter().enumerate() {
                    self.encode_argument(r#type, elem, Some(&format!("{}[{}]", name.unwrap_or("arr"), i)))?;
                }
            }
            AbiType::String { length } => {
                let string = arg.as_str().ok_or("Expected string")?;
                for i in 0..*length {
                    let char = string.chars().nth(i).unwrap_or('\0');
                    self.flattened.push(Fr::from_u8(char as u8));
                }
            }
            AbiType::Struct { fields, .. } => {
                let obj = arg.as_object().ok_or("Expected object for struct")?;
                for field in fields {
                    let field_val = obj.get(&field.name).ok_or("Missing struct field")?;
                    self.encode_argument(&field.field_type, field_val, Some(&field.name))?;
                }
            }
            AbiType::Integer { sign: _, width: _ } => {
                if arg.is_string() {
                    let val = BigUint::parse_bytes(arg.as_str().unwrap().as_bytes(), 10)
                        .ok_or("Invalid string bigint")?;
                    self.flattened.push(Fr::from_biguint(val));
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



#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn dummy_contract_artifact(functions: Vec<FunctionArtifact>) -> ContractArtifact {
        ContractArtifact {
            name: "TestContract".to_string(),
            functions,
            non_dispatch_public_functions: vec![],
            storage_layout: HashMap::new(),
            notes: HashMap::new(),
            file_map: DebugFileMap(HashMap::new()),
        }
    }

    fn dummy_function_artifact(name: &str, parameters: Vec<AbiParameter>) -> FunctionArtifact {
        FunctionArtifact {
            name: name.to_string(),
            parameters,
            bytecode: "".to_string(),
            verification_key: None,
            debug_symbols: "".to_string(),
            debug: None,
            function_type: "private".to_string(),
        }
    }

    #[test]
    fn test_encode_single_field_argument() {
        let abi = FunctionAbi {
            name: "set_value".to_string(),
            function_type: "private".to_string(),
            isInternal: false,
            isStatic: false,
            isInitializer: false,
            parameters: vec![AbiParameter {
                name: "value".to_string(),
                abi_type: AbiType::Field,
            }],
            return_types: vec![],
            errorTypes: None,
        };

        let args = vec![json!(42)];
        let encoded = encode_arguments(abi, args).unwrap();
        assert_eq!(encoded[0], Fr::from_u8(42));
    }

    #[test]
    fn test_encode_boolean_argument() {
        let abi = FunctionAbi {
            name: "toggle".to_string(),
            function_type: "private".to_string(),
            isInternal: false,
            isStatic: false,
            isInitializer: false,
            parameters: vec![AbiParameter {
                name: "flag".to_string(),
                abi_type: AbiType::Boolean,
            }],
            return_types: vec![],
            errorTypes: None,
        };

        let args = vec![json!(true)];
        let encoded = encode_arguments(abi, args).unwrap();
        assert_eq!(encoded[0], Fr::from_u8(1));
    }

    #[test]
    fn test_encode_array_argument() {
        let abi = FunctionAbi {
            name: "fill_array".to_string(),
            function_type: "private".to_string(),
            isInternal: false,
            isStatic: false,
            isInitializer: false,
            parameters: vec![AbiParameter {
                name: "arr".to_string(),
                abi_type: AbiType::Array {
                    r#type: Box::new(AbiType::Field),
                    length: 3,
                },
            }],
            return_types: vec![],
            errorTypes: None,
        };

        let args = vec![json!([1, 2, 3])];
        let encoded = encode_arguments(abi, args).unwrap();
        assert_eq!(encoded[0], Fr::from_u8(1));
        assert_eq!(encoded[1], Fr::from_u8(2));
        assert_eq!(encoded[2], Fr::from_u8(3));
    }

    #[test]
    fn test_encode_string_argument() {
        let abi = FunctionAbi {
            name: "set_name".to_string(),
            function_type: "private".to_string(),
            isInternal: false,
            isStatic: false,
            isInitializer: false,
            parameters: vec![AbiParameter {
                name: "name".to_string(),
                abi_type: AbiType::String { length: 5 },
            }],
            return_types: vec![],
            errorTypes: None,
        };

        let args = vec![json!("Rust")];
        let encoded = encode_arguments(abi, args).unwrap();
        assert_eq!(encoded[0], Fr::from_u8(b'R'));
        assert_eq!(encoded[1], Fr::from_u8(b'u'));
        assert_eq!(encoded[2], Fr::from_u8(b's'));
        assert_eq!(encoded[3], Fr::from_u8(b't'));
        assert_eq!(encoded[4], Fr::from_u8(0)); // Padding
    }

    #[test]
    fn test_encode_integer_argument() {
        let abi = FunctionAbi {
            name: "set_int".to_string(),
            function_type: "private".to_string(),
            isInternal: false,
            isStatic: false,
            isInitializer: false,
            parameters: vec![AbiParameter {
                name: "int_val".to_string(),
                abi_type: AbiType::Integer {
                    sign: "unsigned".to_string(),
                    width: 32,
                },
            }],
            return_types: vec![],
            errorTypes: None,
        };

        let args = vec![json!("123456789")];
        let encoded = encode_arguments(abi, args).unwrap();
        assert_eq!(encoded[0], Fr::from_str("123456789"));
    }

    #[test]
    fn test_encode_single_field_u64() {
        let abi = FunctionAbi {
            name: "test_fn".to_string(),
            function_type: "public".to_string(),
            isInternal: false,
            isStatic: false,
            isInitializer: false,
            parameters: vec![AbiParameter {
                name: "value".to_string(),
                abi_type: AbiType::Field,
            }],
            return_types: vec![],
            errorTypes: None,
        };
        let args = vec![json!(42)];
        let encoded = encode_arguments(abi, args).unwrap();
        assert_eq!(encoded.len(), 1);
        assert_eq!(encoded[0], Fr::from_u8(42));
    }

    #[test]
    fn test_set_just_field_encoding() {
        let abi = FunctionAbi {
            name: "set_just_field".to_string(),
            function_type: "public".to_string(),
            isInternal: false,
            isStatic: false,
            isInitializer: false,
            parameters: vec![AbiParameter {
                name: "value".to_string(),
                abi_type: AbiType::Field,
            }],
            return_types: vec![],
            errorTypes: None,
        };

        let args = vec![json!(123456789u64)];

        let encoded = encode_arguments(abi, args).expect("Encoding should succeed");

        assert_eq!(encoded.len(), 1);
        assert_eq!(encoded[0], Fr(BigUint::from(123456789u64)));
    }

    #[test]
    fn test_encode_boolean() {
        let abi = FunctionAbi {
            name: "test_bool".to_string(),
            function_type: "public".to_string(),
            isInternal: false,
            isStatic: false,
            isInitializer: false,
            parameters: vec![AbiParameter {
                name: "flag".to_string(),
                abi_type: AbiType::Boolean,
            }],
            return_types: vec![],
            errorTypes: None,
        };
        let args = vec![json!(true)];
        let encoded = encode_arguments(abi, args).unwrap();
        assert_eq!(encoded.len(), 1);
        assert_eq!(encoded[0], Fr::from_u8(1));
    }

    #[test]
    fn test_function_selector_from_name_and_parameters() {
        let params = vec![
            AbiParameter {
                name: "value".to_string(),
                abi_type: AbiType::Field,
            },
        ];
        let selector = FunctionSelector::from_name_and_parameters("set_just_field", &params);
        assert_eq!(selector.0.len(), 8);
    }

    #[test]
    fn test_get_function_artifact_by_name() {
        let func = FunctionArtifact {
            name: "set_just_field".to_string(),
            parameters: vec![AbiParameter {
                name: "value".to_string(),
                abi_type: AbiType::Field,
            }],
            bytecode: "".to_string(),
            verification_key: None,
            debug_symbols: String::new(),
            debug: None,
            function_type: "public".to_string(),
        };

        let artifact = ContractArtifact {
            name: "MyContract".to_string(),
            functions: vec![func.clone()],
            non_dispatch_public_functions: vec![],
            storage_layout: Default::default(),
            notes: Default::default(),
            file_map: DebugFileMap(Default::default()),
        };

        let resolved = get_function_artifact(&artifact, "set_just_field").unwrap();
        assert_eq!(resolved.name, "set_just_field");
    }

    #[test]
    fn test_get_function_artifact_by_selector() {
        let func = FunctionArtifact {
            name: "set_just_field".to_string(),
            parameters: vec![AbiParameter {
                name: "value".to_string(),
                abi_type: AbiType::Field,
            }],
            bytecode: "".to_string(),
            verification_key: None,
            debug_symbols: String::new(),
            debug: None,
            function_type: "public".to_string(),
        };

        let artifact = ContractArtifact {
            name: "MyContract".to_string(),
            functions: vec![func.clone()],
            non_dispatch_public_functions: vec![],
            storage_layout: Default::default(),
            notes: Default::default(),
            file_map: DebugFileMap(Default::default()),
        };

        let selector = FunctionSelector::from_name_and_parameters(&func.name, &func.parameters);
        let resolved = get_function_artifact(&artifact, &selector.0).unwrap();
        assert_eq!(resolved.name, "set_just_field");
    }

    #[test]
    fn test_nested_struct_encoding() {
        let abi = FunctionAbi {
            name: "nested_struct".to_string(),
            function_type: "public".to_string(),
            isInternal: false,
            isStatic: false,
            isInitializer: false,
            parameters: vec![AbiParameter {
                name: "nested".to_string(),
                abi_type: AbiType::Struct {
                    path: "MyContract::nested".to_string(),
                    fields: vec![
                        AbiStructField {
                            name: "a".to_string(),
                            field_type: AbiType::Field,
                        },
                        AbiStructField {
                            name: "b".to_string(),
                            field_type: AbiType::Struct {
                                path: "MyContract::nested.b".to_string(),
                                fields: vec![
                                    AbiStructField {
                                        name: "x".to_string(),
                                        field_type: AbiType::Field,
                                    },
                                    AbiStructField {
                                        name: "y".to_string(),
                                        field_type: AbiType::Field,
                                    },
                                ]
                            },
                        },
                    ],
                },
            }],
            return_types: vec![],
            errorTypes: None,
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
            function_type: "public".to_string(),
            isInternal: false,
            isStatic: false,
            isInitializer: false,
            parameters: vec![AbiParameter {
                name: "value".to_string(),
                abi_type: AbiType::Struct {
                    path: "MyContract::intStruct".to_string(),
                    fields: vec![AbiStructField {
                        name: "int".to_string(),
                        field_type: AbiType::Integer {
                            sign: "unsigned".to_string(),
                            width: 64,
                        },
                    }],
                },
            }],
            return_types: vec![],
            errorTypes: None,
        };

        let args = vec![json!({ "int": "9876543210" })];
        let encoded = encode_arguments(abi, args).unwrap();
        assert_eq!(encoded[0], Fr::from_str("9876543210"));
    }

    #[test]
    fn test_string_encoding() {
        let abi = FunctionAbi {
            name: "set_name".to_string(),
            function_type: "public".to_string(),
            isInternal: false,
            isStatic: false,
            isInitializer: false,
            parameters: vec![AbiParameter {
                name: "name".to_string(),
                abi_type: AbiType::String { length: 5 },
            }],
            return_types: vec![],
            errorTypes: None,
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
            function_type: "public".to_string(),
            isInternal: false,
            isStatic: false,
            isInitializer: false,
            parameters: vec![AbiParameter {
                name: "points".to_string(),
                abi_type: AbiType::Array {
                    r#type: Box::new(AbiType::Struct {
                        path: "MyContract::Point".to_string(),
                        fields: vec![
                            AbiStructField {
                                name: "x".to_string(),
                                field_type: AbiType::Field,
                            },
                            AbiStructField {
                                name: "y".to_string(),
                                field_type: AbiType::Field,
                            },
                        ],
                    }),
                    length: 2,
                },
            }],
            return_types: vec![],
            errorTypes: None,
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
    fn test_function_selector_mixed_params() {
        let abi_params = vec![
            AbiParameter {
                name: "flag".to_string(),
                abi_type: AbiType::Boolean,
            },
            AbiParameter {
                name: "value".to_string(),
                abi_type: AbiType::Field,
            },
        ];

        let selector = FunctionSelector::from_name_and_parameters("do_action", &abi_params);
        assert_eq!(selector.0.len(), 8);
    }

    #[test]
    fn test_encode_struct() {
        let abi = FunctionAbi {
            name: "test_struct".to_string(),
            function_type: "public".to_string(),
            isInternal: false,
            isStatic: false,
            isInitializer: false,
            parameters: vec![AbiParameter {
                name: "data".to_string(),
                abi_type: AbiType::Struct {
                    path: "MyContract::Data".to_string(),
                    fields: vec![
                        AbiStructField {
                            name: "a".to_string(),
                            field_type: AbiType::Field,
                        },
                        AbiStructField {
                            name: "b".to_string(),
                            field_type: AbiType::Boolean,
                        },
                    ],
                },
            }],
            return_types: vec![],
            errorTypes: None,
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
            function_type: "public".to_string(),
            isInternal: false,
            isStatic: false,
            isInitializer: false,
            parameters: vec![AbiParameter {
                name: "int_val".to_string(),
                abi_type: AbiType::Integer {
                    sign: "unsigned".to_string(),
                    width: 32,
                },
            }],
            return_types: vec![],
            errorTypes: None,
        };
        let args = vec![json!("12345678901234567890")];
        let encoded = encode_arguments(abi, args).unwrap();
        assert_eq!(encoded.len(), 1);
        assert_eq!(encoded[0], Fr::from_str("12345678901234567890"));
    }
}
