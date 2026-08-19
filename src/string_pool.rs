//! String pool management for MSI databases
//!
//! The string pool assigns sequential IDs to strings and handles UTF-8 encoding.
//! MSI requires strings to be sorted by their pool ID, not alphabetically.

use crate::error::Result;
use std::collections::HashMap;

/// Maximum length for short strings (stored inline in table rows)
#[allow(dead_code)]
pub const MAX_SHORT_STRING: usize = 255;

/// String pool that assigns IDs to strings
#[derive(Debug)]
pub struct StringPool {
    /// Map from string text to (pool ID, reference count)
    strings: HashMap<String, (u32, u32)>,
    /// Next ID to assign (starts at 1, 0 is reserved for empty/null)
    next_id: u32,
    /// Whether to use long string refs (4 bytes) or short (2 bytes)
    long_string_refs: bool,
}

impl StringPool {
    /// Create a new empty string pool
    pub fn new(long_string_refs: bool) -> Self {
        Self {
            strings: HashMap::new(),
            next_id: 1, // ID 0 is reserved for empty/null strings
            long_string_refs,
        }
    }

    /// Get or assign an ID for a string
    /// Returns the pool ID for the string
    pub fn intern(&mut self, text: &str) -> u32 {
        if text.is_empty() {
            return 0; // Empty strings get ID 0
        }

        if let Some(&(id, count)) = self.strings.get(text) {
            // String already exists, increment refcount
            self.strings.insert(text.to_string(), (id, count + 1));
            return id;
        }

        let id = self.next_id;
        self.next_id += 1;
        self.strings.insert(text.to_string(), (id, 1));
        id
    }

    /// Get the ID for a string without interning it
    pub fn get_id(&self, text: &str) -> Option<u32> {
        if text.is_empty() {
            Some(0)
        } else {
            self.strings.get(text).map(|&(id, _)| id)
        }
    }

    /// Encode a string to UTF-8 bytes
    pub fn encode(text: &str) -> Result<Vec<u8>> {
        Ok(text.as_bytes().to_vec())
    }

    /// Get the number of unique strings in the pool
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Check if the pool is empty
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Whether this pool uses long string refs (4 bytes) or short (2 bytes)
    pub fn long_string_refs(&self) -> bool {
        self.long_string_refs
    }

    /// Get all strings and their IDs for serialization
    pub fn iter(&self) -> impl Iterator<Item = (&str, u32, u32)> {
        self.strings.iter().map(|(s, &(id, refcount))| (s.as_str(), id, refcount))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_empty() {
        let mut pool = StringPool::new(false);
        assert_eq!(pool.intern(""), 0);
        assert_eq!(pool.intern(""), 0);
    }

    #[test]
    fn test_intern_strings() {
        let mut pool = StringPool::new(false);
        let id1 = pool.intern("Hello");
        let id2 = pool.intern("World");
        let id3 = pool.intern("Hello"); // Duplicate

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 1); // Same as first "Hello"
    }

    #[test]
    fn test_encode_utf8() {
        let bytes = StringPool::encode("Hello").unwrap();
        assert_eq!(bytes, vec![72, 101, 108, 108, 111]);
    }
}
