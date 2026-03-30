use zeroize::Zeroize;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum VaultStatus {
    Unlocked,
    Locked,
    NoPassword,
}

#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct DerivedKey {
    pub key_bytes: [u8; 32],
}

impl std::fmt::Debug for DerivedKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DerivedKey(***)")
    }
}
