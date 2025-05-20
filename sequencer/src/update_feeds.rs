use serde_json::{json, Value};

fn get_contract_by_address(contract_address: String, wallet: String) {
    // return MainContract.at(contractAddress, wallet);
}

struct ContractArtifact {
    pub value: Vec<Value>
}

pub struct Contract {
    pub instance: String,
    pub wallet: String,
    pub artifact: ContractArtifact,
}

// address: AztecAddress, artifact: ContractArtifact, wallet: Wallet
fn at(address: String, artifact: ContractArtifact, wallet: String) {

}

