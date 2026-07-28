use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha2::{Digest, Sha512};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrityStatus {
    Verified,
    Mismatch,
    Absent,
    Unsupported,
}

pub fn verify_sha512_integrity(integrity: Option<&str>, artifact_bytes: &[u8]) -> IntegrityStatus {
    let Some(integrity) = integrity else {
        return IntegrityStatus::Absent;
    };

    let Some(encoded_digest) = integrity.strip_prefix("sha512-") else {
        return IntegrityStatus::Unsupported;
    };

    if encoded_digest.is_empty() || encoded_digest.contains(char::is_whitespace) {
        return IntegrityStatus::Unsupported;
    }

    let Ok(expected_digest) = STANDARD.decode(encoded_digest) else {
        return IntegrityStatus::Unsupported;
    };

    if expected_digest.len() != 64 {
        return IntegrityStatus::Unsupported;
    }

    let computed_digest = Sha512::digest(artifact_bytes);

    if expected_digest.as_slice() == computed_digest.as_slice() {
        IntegrityStatus::Verified
    } else {
        IntegrityStatus::Mismatch
    }
}

pub fn compute_sha512_hex(bytes: &[u8]) -> String {
    lowercase_hex(&Sha512::digest(bytes))
}

fn lowercase_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }

    output
}

#[cfg(test)]
mod tests;
