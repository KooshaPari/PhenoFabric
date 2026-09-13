//! Built-in frame transforms: base64, gzip, sha256, msgpack encode/decode.

use std::io::{Read, Write};

/// Errors during transform execution.
#[derive(Debug, Clone)]
pub enum TransformError {
    UnknownTransform(String),
    Encoding(String),
    Decoding(String),
}

impl std::fmt::Display for TransformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTransform(name) => write!(f, "unknown transform: {name}"),
            Self::Encoding(e) => write!(f, "encoding error: {e}"),
            Self::Decoding(e) => write!(f, "decoding error: {e}"),
        }
    }
}

impl std::error::Error for TransformError {}

/// A frame transform specification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransformSpec {
    pub name: String,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

/// Apply a chain of transforms to input bytes.
pub fn apply_transforms(
    input: &[u8],
    transforms: &[TransformSpec],
) -> Result<Vec<u8>, TransformError> {
    let mut data = input.to_vec();
    for spec in transforms {
        data = apply_single(&data, &spec.name)?;
    }
    Ok(data)
}

fn apply_single(input: &[u8], name: &str) -> Result<Vec<u8>, TransformError> {
    match name {
        "identity" => Ok(input.to_vec()),
        "base64_encode" => {
            use base64::Engine;
            Ok(base64::engine::general_purpose::STANDARD
                .encode(input)
                .into_bytes())
        }
        "base64_decode" => {
            use base64::Engine;
            let s =
                std::str::from_utf8(input).map_err(|e| TransformError::Decoding(e.to_string()))?;
            base64::engine::general_purpose::STANDARD
                .decode(s)
                .map_err(|e| TransformError::Decoding(e.to_string()))
        }
        "gzip_compress" => {
            use flate2::write::GzEncoder;
            use flate2::Compression;
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder
                .write_all(input)
                .map_err(|e| TransformError::Encoding(e.to_string()))?;
            encoder
                .finish()
                .map_err(|e| TransformError::Encoding(e.to_string()))
        }
        "gzip_decompress" => {
            use flate2::read::GzDecoder;
            let mut decoder = GzDecoder::new(input);
            let mut output = Vec::new();
            decoder
                .read_to_end(&mut output)
                .map_err(|e| TransformError::Decoding(e.to_string()))?;
            Ok(output)
        }
        "sha256" => {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(input);
            Ok(hasher.finalize().to_vec())
        }
        "msgpack_encode" => {
            rmp_serde::to_vec(input).map_err(|e| TransformError::Encoding(e.to_string()))
        }
        "msgpack_decode" => {
            rmp_serde::from_slice(input).map_err(|e| TransformError::Decoding(e.to_string()))
        }
        other => Err(TransformError::UnknownTransform(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_roundtrip() {
        let data = b"hello world";
        let result = apply_transforms(
            data,
            &[TransformSpec {
                name: "identity".into(),
                config: None,
            }],
        )
        .unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn base64_roundtrip() {
        let data = b"hello world";
        let transforms = vec![
            TransformSpec {
                name: "base64_encode".into(),
                config: None,
            },
            TransformSpec {
                name: "base64_decode".into(),
                config: None,
            },
        ];
        let result = apply_transforms(data, &transforms).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn gzip_roundtrip() {
        let data = b"hello world! this is test data for compression";
        let transforms = vec![
            TransformSpec {
                name: "gzip_compress".into(),
                config: None,
            },
            TransformSpec {
                name: "gzip_decompress".into(),
                config: None,
            },
        ];
        let result = apply_transforms(data, &transforms).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn sha256_produces_hash() {
        let data = b"test";
        let result = apply_transforms(
            data,
            &[TransformSpec {
                name: "sha256".into(),
                config: None,
            }],
        )
        .unwrap();
        assert_eq!(result.len(), 32);
    }

    #[test]
    fn unknown_transform_errors() {
        let result = apply_transforms(
            b"test",
            &[TransformSpec {
                name: "nonexistent".into(),
                config: None,
            }],
        );
        assert!(result.is_err());
    }

    #[test]
    fn chain_compress_then_encode() {
        let data = b"compress me then base64 encode";
        let transforms = vec![
            TransformSpec {
                name: "gzip_compress".into(),
                config: None,
            },
            TransformSpec {
                name: "base64_encode".into(),
                config: None,
            },
        ];
        let encoded = apply_transforms(data, &transforms).unwrap();
        let decode_transforms = vec![
            TransformSpec {
                name: "base64_decode".into(),
                config: None,
            },
            TransformSpec {
                name: "gzip_decompress".into(),
                config: None,
            },
        ];
        let decoded = apply_transforms(&encoded, &decode_transforms).unwrap();
        assert_eq!(decoded, data);
    }
}
