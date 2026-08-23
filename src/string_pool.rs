//! String pool management for MSI databases
//!
//! The string pool assigns sequential IDs to strings and handles UTF-8 encoding.
//! MSI requires strings to be sorted by their pool ID, not alphabetically.

use crate::error::Result;
use std::collections::BTreeMap;

/// Maximum length for short strings (stored inline in table rows)
#[allow(dead_code)]
pub const MAX_SHORT_STRING: usize = 255;

/// String pool that assigns IDs to strings
#[derive(Debug)]
pub struct StringPool {
    /// Map from string text to (pool ID, reference count)
    /// Uses BTreeMap so strings are assigned IDs in alphabetical order,
    /// matching the msi crate reference implementation.
    strings: BTreeMap<String, (u32, u32)>,
    /// Next ID to assign (starts at 1, 0 is reserved for empty/null)
    next_id: u32,
    /// Whether to use long string refs (4 bytes) or short (2 bytes)
    long_string_refs: bool,
}

impl StringPool {
    /// Create a new empty string pool
    pub fn new(long_string_refs: bool) -> Self {
        Self {
            strings: BTreeMap::new(),
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

    /// Encode a string to Windows-1252 bytes.
    /// ASCII characters (0x00-0x7F) map directly. For characters in the
    /// 0x80-0xFF range, we use the Windows-1252 code page mapping.
    /// Characters outside the BMP or not in Windows-1252 are replaced with '?'.
    pub fn encode_win1252(text: &str) -> Vec<u8> {
        let mut result = Vec::with_capacity(text.len());
        for ch in text.chars() {
            let cp = ch as u32;
            if cp <= 0x7F {
                result.push(cp as u8);
            } else if cp <= 0xFF {
                // Latin-1 supplement maps directly to Windows-1252 for most chars
                // except for a few positions (0x80-0x9F) that have special mappings
                result.push(cp as u8);
            } else {
                // Common Windows-1252 extensions for Unicode code points
                let byte = match cp {
                    0x20AC => 0x80, // €
                    0x201A => 0x82, // ‚
                    0x0192 => 0x83, // ƒ
                    0x201E => 0x84, // „
                    0x2026 => 0x85, // …
                    0x2020 => 0x86, // †
                    0x2021 => 0x87, // ‡
                    0x02C6 => 0x88, // ˆ
                    0x2030 => 0x89, // ‰
                    0x0160 => 0x8A, // Š
                    0x2039 => 0x8B, // ‹
                    0x0152 => 0x8C, // Œ
                    0x017D => 0x8E, // Ž
                    0x2018 => 0x91, // '
                    0x2019 => 0x92, // '
                    0x201C => 0x93, // "
                    0x201D => 0x94, // "
                    0x2022 => 0x95, // •
                    0x2013 => 0x96, // –
                    0x2014 => 0x97, // —
                    0x02DC => 0x98, // ˜
                    0x2122 => 0x99, // ™
                    0x0161 => 0x9A, // š
                    0x203A => 0x9B, // ›
                    0x0153 => 0x9C, // œ
                    0x017E => 0x9E, // ž
                    0x0178 => 0x9F, // Ÿ
                    _ => b'?',       // Unknown char → '?'
                };
                result.push(byte);
            }
        }
        result
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

    /// Reassign all string pool IDs in alphabetical order.
    /// Must be called after all strings have been interned but before
    /// any table serialization. This ensures IDs match alphabetical order,
    /// which is what the msi crate reference implementation does (BTreeMap).
    pub fn reindex(&mut self) {
        // BTreeMap already iterates in alphabetical order.
        // Collect (key, refcount) pairs, then reassign IDs sequentially.
        let sorted: Vec<(String, u32)> = self.strings.iter()
            .map(|(k, &(_, rc))| (k.clone(), rc))
            .collect();
        self.strings.clear();
        self.next_id = 1;
        for (key, rc) in sorted {
            let id = self.next_id;
            self.next_id += 1;
            self.strings.insert(key, (id, rc));
        }
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
