#[derive(Debug, Clone)]
pub struct Fr(pub BigUint);

impl Fr {
    pub fn from_u8(v: u8) -> Self {
        Fr(BigUint::from(v))
    }
    pub fn from_bigint(v: BigUint) -> Self {
        Fr(v)
    }
    pub fn from_str(s: &str) -> Self {
        Fr(BigUint::parse_bytes(s.as_bytes(), 10).unwrap())
    }
}
