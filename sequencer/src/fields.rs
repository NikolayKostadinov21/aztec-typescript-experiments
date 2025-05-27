use num_bigint::BigUint;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fr(pub BigUint);

impl Fr {
    pub fn from_u8(v: u8) -> Self {
        Fr(BigUint::from(v))
    }

    pub fn from_str(s: &str) -> Self {
        Fr(BigUint::parse_bytes(s.as_bytes(), 10).unwrap())
    }

    pub fn from_biguint(b: BigUint) -> Self {
        Fr(b)
    }

    pub fn from_u64(v: u64) -> Self {
        Fr(BigUint::from(v))
    }
}

