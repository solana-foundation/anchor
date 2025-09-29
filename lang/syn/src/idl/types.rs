use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IdlSeed {
    Const(IdlSeedConst),
    Arg(IdlSeedArg),
    Account(IdlSeedAccount),
    Int(IdlSeedInt),
    Signer(IdlSeedSigner),
    String(IdlSeedString),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlSeedConst {
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlSeedArg {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlSeedAccount {
    pub path: String,
    pub account: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlSeedInt {
    pub value: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlSeedSigner {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlSeedString {
    pub value: String,
}
