use zeroize::Zeroize;

/// A String wrapper that zeroizes its contents on drop.
#[derive(Clone)]
#[allow(dead_code)]
pub struct SecureString {
    inner: String,
}

#[allow(dead_code)]
impl SecureString {
    pub fn new(s: String) -> Self {
        Self { inner: s }
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.inner.as_bytes()
    }
}

impl Drop for SecureString {
    fn drop(&mut self) {
        self.inner.zeroize();
    }
}

impl std::fmt::Debug for SecureString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecureString(***)")
    }
}
