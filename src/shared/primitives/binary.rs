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
    pub(crate) fn new(data: impl Into<Vec<u8>>) -> Self {
        Self(data.into())
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn into_vec(self) -> Vec<u8> {
        self.0
    }

    #[must_use]
    pub(crate) fn as_slice(&self) -> &[u8] {
        &self.0
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn len(&self) -> usize {
        self.0.len()
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn is_empty(&self) -> bool {
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
    use crate::features::module_runtime::test_support::capture_logs;

    #[test]
    fn test_binary_data_basic_operations() {
        let binary = BinaryData::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(binary.len(), 5);
        assert!(!binary.is_empty());
        assert_eq!(binary.as_slice(), &[1, 2, 3, 4, 5]);
        assert_eq!(&*binary, &[1, 2, 3, 4, 5]);
        assert_eq!(binary.as_ref(), &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_binary_data_converts_to_and_from_bytes() {
        let raw = vec![1, 2, 3, 4, 5];
        let binary = BinaryData::new(raw.clone());

        let into_v: Vec<u8> = binary.clone().into_vec();
        assert_eq!(into_v, raw);

        let from_slice: BinaryData = (&raw[..]).into();
        assert_eq!(from_slice, binary);
    }

    #[test]
    fn test_binary_data_default_is_empty() {
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
        let binary = BinaryData::new(vec![1, 2, 3]);

        // `capture_logs` enables every level on this thread only, so `cranky::binary` traces.
        let mut formatted = String::new();
        capture_logs(|| formatted = format!("{binary:?}"));
        assert_eq!(formatted, "[1, 2, 3]");
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
