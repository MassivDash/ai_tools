//! Metadata conversion utilities
//!
//! This module provides utilities for converting between different metadata formats
//! used by ChromaDB and our application.

use chroma::types::{Metadata, MetadataValue};
use serde_json::Value;
use std::collections::HashMap;

/// Convert our application's metadata format to ChromaDB's Metadata format
pub fn to_chromadb_metadata(metadata: Option<HashMap<String, String>>) -> Option<Metadata> {
    metadata.map(|m| {
        m.into_iter()
            .map(|(k, v)| (k, MetadataValue::Str(v)))
            .collect()
    })
}

/// Convert a single metadata HashMap to ChromaDB format
pub fn hashmap_to_chromadb_metadata(metadata: HashMap<String, String>) -> Metadata {
    metadata
        .into_iter()
        .map(|(k, v)| (k, MetadataValue::Str(v)))
        .collect()
}

/// Convert a vector of metadata HashMaps to ChromaDB format
pub fn vec_to_chromadb_metadata(metadatas: Vec<HashMap<String, String>>) -> Vec<Option<Metadata>> {
    metadatas
        .into_iter()
        .map(|meta| Some(hashmap_to_chromadb_metadata(meta)))
        .collect()
}

/// Convert ChromaDB's MetadataValue to a String representation
pub fn metadata_value_to_string(value: &MetadataValue) -> String {
    match value {
        MetadataValue::Str(s) => s.clone(),
        MetadataValue::Int(i) => i.to_string(),
        MetadataValue::Float(f) => f.to_string(),
        MetadataValue::Bool(b) => b.to_string(),
        MetadataValue::SparseVector(_) => "SparseVector".to_string(),
    }
}

/// Convert ChromaDB's Metadata to our application's HashMap format
pub fn chromadb_metadata_to_hashmap(metadata: &Metadata) -> HashMap<String, String> {
    metadata
        .iter()
        .map(|(k, v)| (k.clone(), metadata_value_to_string(v)))
        .collect()
}

/// Convert ChromaDB's MetadataValue to serde_json::Value
pub fn metadata_value_to_json(value: &MetadataValue) -> Value {
    match value {
        MetadataValue::Str(s) => Value::String(s.clone()),
        MetadataValue::Int(i) => Value::Number((*i).into()),
        MetadataValue::Float(f) => {
            Value::Number(serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from(0)))
        }
        MetadataValue::Bool(b) => Value::Bool(*b),
        MetadataValue::SparseVector(_) => Value::String("SparseVector".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_value_to_string() {
        assert_eq!(
            metadata_value_to_string(&MetadataValue::Str("test".to_string())),
            "test"
        );
        assert_eq!(metadata_value_to_string(&MetadataValue::Int(42)), "42");
        assert_eq!(
            metadata_value_to_string(&MetadataValue::Float(3.15)), // Using 3.15 for testing (avoid clippy approx_constant)
            "3.15"
        );
        assert_eq!(metadata_value_to_string(&MetadataValue::Bool(true)), "true");
    }

    #[test]
    fn test_hashmap_to_chromadb_metadata() {
        let mut input = HashMap::new();
        input.insert("key1".to_string(), "value1".to_string());
        input.insert("key2".to_string(), "value2".to_string());

        let result = hashmap_to_chromadb_metadata(input);
        assert_eq!(result.len(), 2);
        assert_eq!(
            result.get("key1"),
            Some(&MetadataValue::Str("value1".to_string()))
        );
    }

    #[test]
    fn test_to_chromadb_metadata() {
        let mut input = HashMap::new();
        input.insert("test".to_string(), "value".to_string());

        let result = to_chromadb_metadata(Some(input));
        assert!(result.is_some());
        let metadata = result.unwrap();
        assert_eq!(
            metadata.get("test"),
            Some(&MetadataValue::Str("value".to_string()))
        );
    }

    #[test]
    fn test_to_chromadb_metadata_none() {
        let result = to_chromadb_metadata(None);
        assert!(result.is_none());
    }

    #[test]
    fn test_metadata_value_to_json() {
        assert_eq!(
            metadata_value_to_json(&MetadataValue::Str("test".to_string())),
            Value::String("test".to_string())
        );
        assert_eq!(
            metadata_value_to_json(&MetadataValue::Bool(true)),
            Value::Bool(true)
        );
    }
}
