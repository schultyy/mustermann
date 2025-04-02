use std::collections::HashMap;

use opentelemetry::propagation::{Extractor, Injector};

pub struct MetadataMap<'a>(&'a mut HashMap<String, String>);

impl Injector for MetadataMap<'_> {
    /// Set a key and value in the MetadataMap.  Does nothing if the key or value are not valid inputs
    fn set(&mut self, key: &str, value: String) {
        let key = key.to_string();
        let value = value.to_string();
        self.0.insert(key, value);
    }
}

impl Extractor for MetadataMap<'_> {
    /// Get a value for a key from the MetadataMap.  If the value can't be converted to &str, returns None
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(|value| value.as_str())
    }

    /// Collect all the keys from the MetadataMap.
    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|key| key.as_str()).collect::<Vec<_>>()
    }
}
