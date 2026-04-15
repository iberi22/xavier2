//! Crypto utilities - Simple byte-to-hex encoding and SHA256 hashing
//!
//! Provides lightweight alternatives to the hex crate.

use sha2::{Digest, Sha256};

/// Encode bytes to lowercase hex string
pub fn hex_encode(data: &[u8]) -> String {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut result = Vec::with_capacity(data.len() * 2);
    for &byte in data {
        result.push(HEX_CHARS[(byte >> 4) as usize]);
        result.push(HEX_CHARS[(byte & 0xf) as usize]);
    }
    unsafe { String::from_utf8_unchecked(result) }
}

/// Decode lowercase hex string to bytes
pub fn hex_decode(hex_str: &str) -> anyhow::Result<Vec<u8>> {
    let hex_str = hex_str.trim();
    if !hex_str.len().is_multiple_of(2) {
        anyhow::bail!("hex string must have even length");
    }

    let bytes: Vec<u8> = hex_str
        .as_bytes()
        .chunks(2)
        .map(|chunk| {
            let high = char_to_nibble(chunk[0]).map_err(anyhow::Error::msg)?;
            let low = char_to_nibble(chunk[1]).map_err(anyhow::Error::msg)?;
            Ok::<u8, anyhow::Error>(high << 4 | low)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(bytes)
}

#[inline]
fn char_to_nibble(c: u8) -> Result<u8, &'static str> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err("invalid hex character"),
    }
}

/// Compute SHA256 hash and return as hex string
pub fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    hex_encode(&digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_encode() {
        assert_eq!(hex_encode(b"hello"), "68656c6c6f");
        assert_eq!(hex_encode(b""), "");
        assert_eq!(hex_encode(&[0xab, 0xcd]), "abcd");
    }

    #[test]
    fn test_hex_decode() {
        assert_eq!(hex_decode("68656c6c6f").unwrap(), b"hello");
        assert_eq!(hex_decode("").unwrap(), b"");
        assert_eq!(hex_decode("ABCD").unwrap(), &[0xab, 0xcd]);
    }

    #[test]
    fn test_hex_decode_invalid() {
        assert!(hex_decode("xyz").is_err());
        assert!(hex_decode("123").is_err()); // odd length
    }

    #[test]
    fn test_sha256_hex() {
        let hash = sha256_hex(b"hello");
        assert_eq!(hash.len(), 64);
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}
