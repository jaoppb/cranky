use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;

/// Represents a binary data buffer with a `Debug` implementation that defaults
/// to omitting raw byte arrays in logs unless `cranky::binary=trace` is enabled.
#[derive(Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BinaryData(#[serde(with = "serde_bytes")] Vec<u8>);

impl BinaryData {
    #[must_use]
    pub fn new(data: impl Into<Vec<u8>>) -> Self {
        Self(data.into())
    }

    #[must_use]
    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Deref for BinaryData {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[u8]> for BinaryData {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for BinaryData {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl From<&[u8]> for BinaryData {
    fn from(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }
}

impl From<BinaryData> for Vec<u8> {
    fn from(binary: BinaryData) -> Self {
        binary.0
    }
}

impl fmt::Debug for BinaryData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if tracing::enabled!(target: "cranky::binary", tracing::Level::TRACE) {
            self.0.fmt(f)
        } else {
            let len = self.0.len();
            write!(f, "<Binary Data ({len} bytes)>")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::layer::SubscriberExt;

    #[test]
    fn test_binary_data_basic_operations() {
        let raw = vec![1, 2, 3, 4, 5];
        let binary = BinaryData::new(raw.clone());
        assert_eq!(binary.len(), 5);
        assert!(!binary.is_empty());
        assert_eq!(binary.as_slice(), &[1, 2, 3, 4, 5]);
        assert_eq!(&*binary, &[1, 2, 3, 4, 5]);
        assert_eq!(binary.as_ref(), &[1, 2, 3, 4, 5]);

        let into_v: Vec<u8> = binary.clone().into_vec();
        assert_eq!(into_v, raw);

        let from_slice: BinaryData = (&raw[..]).into();
        assert_eq!(from_slice, binary);

        let empty = BinaryData::default();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_binary_data_debug_default_omission() {
        let binary = BinaryData::new(vec![10, 20, 30]);
        let formatted = format!("{binary:?}");
        assert_eq!(formatted, "<Binary Data (3 bytes)>");

        let empty = BinaryData::default();
        let formatted_empty = format!("{empty:?}");
        assert_eq!(formatted_empty, "<Binary Data (0 bytes)>");
    }

    #[test]
    fn test_binary_data_debug_with_tracing_filter_enabled() {
        let filter = tracing_subscriber::EnvFilter::new("cranky::binary=trace");
        let subscriber = tracing_subscriber::registry().with(filter);

        let binary = BinaryData::new(vec![1, 2, 3]);

        tracing::subscriber::with_default(subscriber, || {
            let formatted = format!("{binary:?}");
            assert_eq!(formatted, "[1, 2, 3]");
        });
    }

    #[test]
    fn test_binary_data_serde_roundtrip() {
        let original = BinaryData::new(vec![0xDE, 0xAD, 0xBE, 0xEF]);
        let serialized = serde_json::to_string(&original).expect("Serialization failed");
        let deserialized: BinaryData =
            serde_json::from_str(&serialized).expect("Deserialization failed");
        assert_eq!(original, deserialized);
    }
}
