use num_bigint::BigUint;

#[derive(Debug, Clone)]
pub struct Fr(pub BigUint);

impl Fr {
    pub fn from_u8(v: u8) -> Self {
        Fr(BigUint::from(v))
    }

    pub fn from_str(s: &str) -> Self {
        Fr(BigUint::parse_bytes(s.as_bytes(), 10).expect("Invalid number string"))
    }

    pub fn from_biguint(b: BigUint) -> Self {
        Fr(b)
    }
}
