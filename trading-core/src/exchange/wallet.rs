// ====
// Hyperliquid Wallet - Custom Implementation
// ====
// EIP-712 Signing für Hyperliquid API
// Keine externen Wallet-Libraries
// ====

use anyhow::{anyhow, Context, Result};
use hex;
use k256::ecdsa::{signature::Signer, Signature, SigningKey};
use k256::SecretKey;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

/// Hyperliquid Wallet
///
/// Custom wallet implementation for Hyperliquid
/// - EIP-712 signing
/// - No external wallet libraries
/// - Direct private key management
pub struct HyperliquidWallet {
    /// Private key
    private_key: SigningKey,
    /// Public address (0x...)
    pub address: String,
}

impl HyperliquidWallet {
    /// Create wallet from private key hex string
    ///
    /// Example:
    /// ```
    /// let wallet = HyperliquidWallet::from_private_key("0x1234...")?;
    /// ```
    pub fn from_private_key(private_key_hex: &str) -> Result<Self> {
        // Remove 0x prefix if present
        let key_hex = private_key_hex.trim_start_matches("0x");

        // Decode hex
        let key_bytes = hex::decode(key_hex).context("Failed to decode private key hex")?;

        // Create secret key
        let secret_key = SecretKey::from_slice(&key_bytes).context("Invalid private key")?;

        // Create signing key
        let signing_key = SigningKey::from(secret_key);

        // Derive address
        let address = Self::derive_address(&signing_key)?;

        Ok(Self {
            private_key: signing_key,
            address,
        })
    }

    /// Derive Ethereum address from signing key
    fn derive_address(signing_key: &SigningKey) -> Result<String> {
        // Get public key
        let verifying_key = signing_key.verifying_key();
        let public_key_bytes = verifying_key.to_encoded_point(false);
        let public_key_bytes = public_key_bytes.as_bytes();

        // Skip first byte (0x04 prefix for uncompressed key)
        let public_key = &public_key_bytes[1..];

        // Keccak256 hash
        let mut hasher = Keccak256::new();
        hasher.update(public_key);
        let hash = hasher.finalize();

        // Take last 20 bytes
        let address_bytes = &hash[12..];

        // Format as 0x...
        Ok(format!("0x{}", hex::encode(address_bytes)))
    }

    /// Sign EIP-712 typed data
    ///
    /// Used for Hyperliquid API requests
    pub fn sign_typed_data(&self, typed_data: &TypedData) -> Result<String> {
        // Encode typed data
        let encoded = typed_data.encode()?;

        // Sign
        let signature: Signature = self.private_key.sign(&encoded);

        // Format signature (r, s, v)
        let sig_bytes = signature.to_bytes();
        let r = &sig_bytes[..32];
        let s = &sig_bytes[32..64];

        // Calculate v (recovery id)
        // For Ethereum, v = 27 + recovery_id
        // We use 27 as default (most common)
        let v = 27u8;

        // Concatenate r + s + v
        let mut full_sig = Vec::with_capacity(65);
        full_sig.extend_from_slice(r);
        full_sig.extend_from_slice(s);
        full_sig.push(v);

        // Return as 0x... hex string
        Ok(format!("0x{}", hex::encode(full_sig)))
    }

    /// Sign message (personal_sign)
    ///
    /// For simple message signing
    pub fn sign_message(&self, message: &str) -> Result<String> {
        // Ethereum signed message prefix
        let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
        let full_message = format!("{}{}", prefix, message);

        // Keccak256 hash
        let mut hasher = Keccak256::new();
        hasher.update(full_message.as_bytes());
        let hash = hasher.finalize();

        // Sign
        let signature: Signature = self.private_key.sign(&hash);

        // Format signature
        let sig_bytes = signature.to_bytes();
        let r = &sig_bytes[..32];
        let s = &sig_bytes[32..64];
        let v = 27u8;

        let mut full_sig = Vec::with_capacity(65);
        full_sig.extend_from_slice(r);
        full_sig.extend_from_slice(s);
        full_sig.push(v);

        Ok(format!("0x{}", hex::encode(full_sig)))
    }
}

/// EIP-712 Domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EIP712Domain {
    pub name: String,
    pub version: String,
    #[serde(rename = "chainId")]
    pub chain_id: u64,
    #[serde(rename = "verifyingContract")]
    pub verifying_contract: String,
}

/// EIP-712 Typed Data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedData {
    pub domain: EIP712Domain,
    #[serde(rename = "primaryType")]
    pub primary_type: String,
    pub types: serde_json::Value,
    pub message: serde_json::Value,
}

impl TypedData {
    /// Encode typed data for signing (EIP-712)
    ///
    /// Returns: keccak256("\x19\x01" ‖ domainSeparator ‖ hashStruct(message))
    pub fn encode(&self) -> Result<Vec<u8>> {
        // Encode domain separator
        let domain_separator = self.hash_domain()?;

        // Encode message
        let message_hash = self.hash_struct(&self.primary_type, &self.message)?;

        // Concatenate: "\x19\x01" ‖ domainSeparator ‖ messageHash
        let mut encoded = Vec::with_capacity(66);
        encoded.push(0x19);
        encoded.push(0x01);
        encoded.extend_from_slice(&domain_separator);
        encoded.extend_from_slice(&message_hash);

        // Final keccak256 hash
        let mut hasher = Keccak256::new();
        hasher.update(&encoded);
        let hash = hasher.finalize();

        Ok(hash.to_vec())
    }

    /// Hash domain separator
    fn hash_domain(&self) -> Result<Vec<u8>> {
        // typeHash = keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)")
        let type_hash = keccak256(
            b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
        );

        // Encode domain fields
        let name_hash = keccak256(self.domain.name.as_bytes());
        let version_hash = keccak256(self.domain.version.as_bytes());
        let chain_id_bytes = self.domain.chain_id.to_be_bytes();
        let contract_bytes = hex::decode(self.domain.verifying_contract.trim_start_matches("0x"))
            .context("Invalid verifying contract address")?;

        // Concatenate: typeHash ‖ nameHash ‖ versionHash ‖ chainId ‖ verifyingContract
        let mut encoded = Vec::new();
        encoded.extend_from_slice(&type_hash);
        encoded.extend_from_slice(&name_hash);
        encoded.extend_from_slice(&version_hash);

        // Pad chain_id to 32 bytes
        let mut chain_id_padded = vec![0u8; 32];
        chain_id_padded[24..].copy_from_slice(&chain_id_bytes);
        encoded.extend_from_slice(&chain_id_padded);

        // Pad contract address to 32 bytes
        let mut contract_padded = vec![0u8; 32];
        contract_padded[12..].copy_from_slice(&contract_bytes);
        encoded.extend_from_slice(&contract_padded);

        // Hash
        Ok(keccak256(&encoded).to_vec())
    }

    /// Hash struct
    fn hash_struct(&self, struct_type: &str, data: &serde_json::Value) -> Result<Vec<u8>> {
        // Get type definition
        let type_def = self
            .types
            .get(struct_type)
            .ok_or_else(|| anyhow!("Type {} not found", struct_type))?;

        // Encode type string
        let type_string = self.encode_type(struct_type, type_def)?;
        let type_hash = keccak256(type_string.as_bytes());

        // Encode data
        let mut encoded = Vec::new();
        encoded.extend_from_slice(&type_hash);

        // Encode each field
        if let Some(fields) = type_def.as_array() {
            for field in fields {
                let field_name = field["name"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Field name missing"))?;
                let field_type = field["type"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Field type missing"))?;

                let field_value = &data[field_name];
                let field_encoded = self.encode_field(field_type, field_value)?;
                encoded.extend_from_slice(&field_encoded);
            }
        }

        // Hash
        Ok(keccak256(&encoded).to_vec())
    }

    /// Encode type string
    fn encode_type(&self, struct_type: &str, type_def: &serde_json::Value) -> Result<String> {
        let mut type_string = format!("{}(", struct_type);

        if let Some(fields) = type_def.as_array() {
            let field_strings: Vec<String> = fields
                .iter()
                .map(|field| {
                    let field_type = field["type"].as_str().unwrap_or("");
                    let field_name = field["name"].as_str().unwrap_or("");
                    format!("{} {}", field_type, field_name)
                })
                .collect();

            type_string.push_str(&field_strings.join(","));
        }

        type_string.push(')');
        Ok(type_string)
    }

    /// Encode field value
    fn encode_field(&self, field_type: &str, value: &serde_json::Value) -> Result<Vec<u8>> {
        match field_type {
            "string" => {
                let s = value.as_str().ok_or_else(|| anyhow!("Expected string"))?;
                Ok(keccak256(s.as_bytes()).to_vec())
            }
            "uint256" | "uint64" | "uint32" | "uint8" => {
                let n = value.as_u64().ok_or_else(|| anyhow!("Expected number"))?;
                let mut bytes = vec![0u8; 32];
                bytes[24..].copy_from_slice(&n.to_be_bytes());
                Ok(bytes)
            }
            "address" => {
                let addr = value.as_str().ok_or_else(|| anyhow!("Expected address"))?;
                let addr_bytes =
                    hex::decode(addr.trim_start_matches("0x")).context("Invalid address")?;
                let mut bytes = vec![0u8; 32];
                bytes[12..].copy_from_slice(&addr_bytes);
                Ok(bytes)
            }
            "bool" => {
                let b = value.as_bool().ok_or_else(|| anyhow!("Expected bool"))?;
                let mut bytes = vec![0u8; 32];
                bytes[31] = if b { 1 } else { 0 };
                Ok(bytes)
            }
            _ => Err(anyhow!("Unsupported field type: {}", field_type)),
        }
    }
}

/// Helper: Keccak256 hash
fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_creation() {
        // Test private key (DO NOT USE IN PRODUCTION)
        let private_key = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let wallet = HyperliquidWallet::from_private_key(private_key).unwrap();

        // Check address format
        assert!(wallet.address.starts_with("0x"));
        assert_eq!(wallet.address.len(), 42); // 0x + 40 hex chars
    }

    #[test]
    fn test_sign_message() {
        let private_key = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let wallet = HyperliquidWallet::from_private_key(private_key).unwrap();

        let signature = wallet.sign_message("Hello, Hyperliquid!").unwrap();

        // Check signature format
        assert!(signature.starts_with("0x"));
        assert_eq!(signature.len(), 132); // 0x + 130 hex chars (65 bytes)
    }
}
