//! Table schema and row serialization for MSI databases

use crate::error::{MsiError, Result};
use crate::string_pool::StringPool;
use std::io::Write;

/// Column data types in MSI tables
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    /// 16-bit signed integer
    Int16,
    /// 32-bit signed integer
    Int32,
    /// String reference (2 or 4 bytes depending on long_string_refs)
    StringRef { max_len: usize },
    /// Binary data (not sorted)
    Binary,
}

impl ColumnType {
    /// Get the base bitfield value for this type (without nullable/PK/valid bits)
    fn base_bitfield(&self) -> i32 {
        match self {
            ColumnType::Int16 => 0x2,
            ColumnType::Int32 => 0x4,
            ColumnType::StringRef { max_len } => 0x800 | (*max_len as i32),
            ColumnType::Binary => 0x900, // COL_STRING_BIT | COL_VALID_BIT
        }
    }

    /// Get the width in bytes for this column type
    pub fn width(&self, long_string_refs: bool) -> usize {
        match self {
            ColumnType::Int16 => 2,
            ColumnType::Int32 => 4,
            ColumnType::StringRef { .. } => {
                if long_string_refs {
                    3 // Long string refs: u16 + u8 (3 bytes)
                } else {
                    2
                }
            }
            ColumnType::Binary => 2, // Binary is stored as short ref
        }
    }

    /// Check if this is a string type
    pub fn is_string(&self) -> bool {
        matches!(self, ColumnType::StringRef { .. })
    }
}

/// Column definition
#[derive(Debug, Clone)]
pub struct Column {
    /// Column name (must be unique within table)
    pub name: String,
    /// Data type
    pub col_type: ColumnType,
    /// Whether the column can contain NULL values
    pub nullable: bool,
    /// Whether this column is part of the primary key
    pub primary_key: bool,
}

impl Column {
    /// Create a new column builder
    pub fn build(name: &str) -> ColumnBuilder {
        ColumnBuilder {
            name: name.to_string(),
            col_type: ColumnType::Int32,
            nullable: false,
            primary_key: false,
        }
    }

    /// Get the bitfield value for this column (used in _Columns table)
    ///
    /// MSI column type bitfield layout (per spec):
    ///   bits 0-7:  field size (max string length, or 2/4 for integers)
    ///   bit 8:     valid bit (always set)
    ///   bit 9:     localizable
    ///   bit 10:    non-binary (set for Int16 and String types)
    ///   bit 11:    string type
    ///   bit 12:    nullable
    ///   bit 13:    primary key
    pub fn bitfield(&self) -> i32 {
        let mut bits = self.col_type.base_bitfield();

        // bit 8: always set (COL_VALID_BIT)
        bits |= 0x100;

        // bit 10: non-binary (set for Int16 and String types)
        match self.col_type {
            ColumnType::Int16 | ColumnType::StringRef { .. } => bits |= 0x400,
            _ => {}
        }

        // bit 12: nullable
        if self.nullable {
            bits |= 0x1000;
        }

        // bit 13: primary key
        if self.primary_key {
            bits |= 0x2000;
        }

        bits
    }
}

/// Builder for creating columns
pub struct ColumnBuilder {
    name: String,
    col_type: ColumnType,
    nullable: bool,
    primary_key: bool,
}

impl ColumnBuilder {
    /// Set as 16-bit integer
    pub fn int16(mut self) -> Self {
        self.col_type = ColumnType::Int16;
        self
    }

    /// Set as 32-bit integer
    pub fn int32(mut self) -> Self {
        self.col_type = ColumnType::Int32;
        self
    }

    /// Set as string with max length
    pub fn string(mut self, max_len: usize) -> Self {
        self.col_type = ColumnType::StringRef { max_len };
        self
    }

    /// Set as binary data
    pub fn binary(mut self) -> Self {
        self.col_type = ColumnType::Binary;
        self
    }

    /// Mark as nullable
    pub fn nullable(mut self) -> Self {
        self.nullable = true;
        self
    }

    /// Mark as primary key
    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self
    }

    /// Build the column
    pub fn build(self) -> Column {
        Column {
            name: self.name,
            col_type: self.col_type,
            nullable: self.nullable,
            primary_key: self.primary_key,
        }
    }
}

/// Value that can be stored in a table cell
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// NULL value (only for nullable columns)
    Null,
    /// Integer value
    Int(i32),
    /// String value
    Str(String),
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Str(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Str(s)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Int(i)
    }
}

/// A table with schema and rows
#[derive(Debug, Clone)]
pub struct Table {
    /// Table name
    pub name: String,
    /// Column definitions
    pub columns: Vec<Column>,
    /// Whether to use long string refs
    long_string_refs: bool,
    /// Rows (each row is a Vec of Values)
    rows: Vec<Vec<Value>>,
}

impl Table {
    /// Create a new table with the given schema
    pub fn new(name: &str, columns: Vec<Column>, long_string_refs: bool) -> Self {
        Self {
            name: name.to_string(),
            columns,
            long_string_refs,
            rows: Vec::new(),
        }
    }

    /// Add a row to the table
    pub fn add_row(&mut self, row: Vec<Value>) -> Result<()> {
        if row.len() != self.columns.len() {
            return Err(MsiError::ColumnCountMismatch {
                expected: self.columns.len(),
                actual: row.len(),
            });
        }
        self.rows.push(row);
        Ok(())
    }

    /// Get the number of rows
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Serialize the table to bytes (row-major order)
    /// Rows are sorted by primary key (string-pool ID for strings)
    pub fn serialize(&self, string_pool: &StringPool) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();

        // Sort rows by primary key
        let mut sorted_rows = self.rows.clone();
        sorted_rows.sort_by(|a, b| {
            for (col_idx, col) in self.columns.iter().enumerate() {
                if !col.primary_key {
                    continue;
                }
                if matches!(col.col_type, ColumnType::Binary) {
                    continue; // Binary columns are not sorted
                }

                let cmp = match (&a[col_idx], &b[col_idx]) {
                    (Value::Null, Value::Null) => std::cmp::Ordering::Equal,
                    (Value::Null, _) => std::cmp::Ordering::Less,
                    (_, Value::Null) => std::cmp::Ordering::Greater,
                    (Value::Int(a), Value::Int(b)) => a.cmp(b),
                    (Value::Str(a), Value::Str(b)) => {
                        // Compare by string-pool ID
                        let id_a = string_pool.get_id(a).unwrap_or(0);
                        let id_b = string_pool.get_id(b).unwrap_or(0);
                        id_a.cmp(&id_b)
                    }
                    _ => std::cmp::Ordering::Equal,
                };

                if cmp != std::cmp::Ordering::Equal {
                    return cmp;
                }
            }
            std::cmp::Ordering::Equal
        });

        // Write in column-major order (MSI spec: all values for column 1, then column 2, etc.)
        for (col_idx, col) in self.columns.iter().enumerate() {
            for row in &sorted_rows {
                self.write_value(&mut buffer, &row[col_idx], &col.col_type, string_pool)?;
            }
        }

        Ok(buffer)
    }

    /// Write a single value to the buffer
    ///
    /// Integer encoding (per MSI spec, matching rust-msi reference):
    ///   - NULL → 0 (raw zero bytes)
    ///   - Int16 → `(value as i16) ^ -0x8000` (XOR with 0x8000)
    ///   - Int32 → `value ^ -0x80000000` (XOR with 0x80000000)
    ///   - String refs → raw u16 (short) or u16+u8 (long, 3 bytes)
    fn write_value<W: Write>(
        &self,
        writer: &mut W,
        value: &Value,
        col_type: &ColumnType,
        string_pool: &StringPool,
    ) -> Result<()> {
        match (value, col_type) {
            (Value::Null, ColumnType::Int16) => {
                writer.write_all(&0u16.to_le_bytes())?;
            }
            (Value::Null, ColumnType::Int32) => {
                writer.write_all(&0u32.to_le_bytes())?;
            }
            (Value::Null, ColumnType::StringRef { .. }) => {
                // NULL string = string pool ID 0
                if self.long_string_refs {
                    writer.write_all(&0u16.to_le_bytes())?;
                    writer.write_all(&[0u8])?;
                } else {
                    writer.write_all(&0u16.to_le_bytes())?;
                }
            }
            (Value::Null, ColumnType::Binary) => {
                writer.write_all(&0u16.to_le_bytes())?;
            }
            (Value::Int(i), ColumnType::Int16) => {
                // XOR encode: flip sign bit so non-zero values are distinguishable from NULL
                let encoded = (*i as i16) ^ -0x8000;
                writer.write_all(&encoded.to_le_bytes())?;
            }
            (Value::Int(i), ColumnType::Int32) => {
                // XOR encode: flip sign bit so non-zero values are distinguishable from NULL
                let encoded = *i ^ -0x80000000i32;
                writer.write_all(&encoded.to_le_bytes())?;
            }
            (Value::Str(s), ColumnType::StringRef { .. }) => {
                let id = string_pool.get_id(s).ok_or_else(|| {
                    MsiError::EncodingError(format!(
                        "String '{}' not found in string pool (was it interned?)",
                        if s.len() > 40 { &s[..40] } else { s }
                    ))
                })?;
                if self.long_string_refs {
                    // Long string refs: 3 bytes (u16 low + u8 high)
                    writer.write_all(&((id & 0xffff) as u16).to_le_bytes())?;
                    writer.write_all(&[((id >> 16) & 0xff) as u8])?;
                } else {
                    writer.write_all(&(id as u16).to_le_bytes())?;
                }
            }
            (Value::Str(s), ColumnType::Binary) => {
                // Binary strings are stored as short refs
                let id = string_pool.get_id(s).ok_or_else(|| {
                    MsiError::EncodingError(format!(
                        "String '{}' not found in string pool (was it interned?)",
                        if s.len() > 40 { &s[..40] } else { s }
                    ))
                })?;
                writer.write_all(&(id as u16).to_le_bytes())?;
            }
            _ => {
                return Err(MsiError::EncodingError(format!(
                    "Type mismatch for value: {:?}",
                    value
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_bitfield_int16() {
        let col = Column::build("Test").int16().build();
        let bits = col.bitfield();
        // Int16: base=0x2, valid=0x100, nonbinary=0x400
        assert_eq!(bits, 0x502);
    }

    #[test]
    fn test_column_bitfield_int32() {
        let col = Column::build("Test").int32().nullable().build();
        let bits = col.bitfield();
        // Int32: base=0x4, valid=0x100, nullable=0x1000
        assert_eq!(bits, 0x1104);
    }

    #[test]
    fn test_column_bitfield_string() {
        let col = Column::build("Test").string(72).primary_key().build();
        let bits = col.bitfield();
        // String(72): base=0x848, valid=0x100, nonbinary=0x400, PK=0x2000
        assert_eq!(bits, 0x2D48);
    }

    #[test]
    fn test_column_bitfield_binary() {
        let col = Column::build("Test").binary().nullable().build();
        let bits = col.bitfield();
        // Binary: base=0x900, valid=0x100 (already in base), nullable=0x1000
        assert_eq!(bits, 0x1900);
    }

    #[test]
    fn test_column_type_width() {
        assert_eq!(ColumnType::Int16.width(false), 2);
        assert_eq!(ColumnType::Int32.width(false), 4);
        assert_eq!(ColumnType::StringRef { max_len: 72 }.width(false), 2);
        assert_eq!(ColumnType::StringRef { max_len: 72 }.width(true), 3); // 3 bytes for long refs
        assert_eq!(ColumnType::Binary.width(false), 2);
    }

    #[test]
    fn test_table_add_row() {
        let mut table = Table::new(
            "Test",
            vec![
                Column::build("Id").int32().primary_key().build(),
                Column::build("Name").string(50).build(),
            ],
            false,
        );

        table.add_row(vec![Value::Int(1), Value::Str("Alice".into())]).unwrap();
        assert_eq!(table.row_count(), 1);
    }

    #[test]
    fn test_table_column_count_mismatch() {
        let mut table = Table::new(
            "Test",
            vec![
                Column::build("Id").int32().build(),
                Column::build("Name").string(50).build(),
            ],
            false,
        );

        let result = table.add_row(vec![Value::Int(1)]); // Missing column
        assert!(result.is_err());
        match result.unwrap_err() {
            MsiError::ColumnCountMismatch { expected, actual } => {
                assert_eq!(expected, 2);
                assert_eq!(actual, 1);
            }
            e => panic!("Expected ColumnCountMismatch, got {:?}", e),
        }
    }

    #[test]
    fn test_serialize_int_columns() {
        let mut table = Table::new(
            "Test",
            vec![
                Column::build("A").int16().build(),
                Column::build("B").int32().build(),
            ],
            false,
        );
        table.add_row(vec![Value::Int(10), Value::Int(1000)]).unwrap();
        table.add_row(vec![Value::Int(20), Value::Int(2000)]).unwrap();

        let pool = StringPool::new(false);
        let data = table.serialize(&pool).unwrap();

        // Column-major: col A (2 rows × 2 bytes) + col B (2 rows × 4 bytes)
        // = 4 + 8 = 12 bytes
        assert_eq!(data.len(), 12);

        // Column A: [10, 20] XOR-encoded as i16 LE
        // 10 ^ 0x8000 = 0x800A → [0x0A, 0x80]
        assert_eq!(data[0], 0x0A);
        assert_eq!(data[1], 0x80);
        // 20 ^ 0x8000 = 0x8014 → [0x14, 0x80]
        assert_eq!(data[2], 0x14);
        assert_eq!(data[3], 0x80);

        // Column B: [1000, 2000] XOR-encoded as i32 LE
        // 1000 ^ 0x80000000 = 0x800003E8 → [0xE8, 0x03, 0x00, 0x80]
        assert_eq!(data[4], 0xE8);
        assert_eq!(data[5], 0x03);
        assert_eq!(data[6], 0x00);
        assert_eq!(data[7], 0x80);
        // 2000 ^ 0x80000000 = 0x800007D0 → [0xD0, 0x07, 0x00, 0x80]
        assert_eq!(data[8], 0xD0);
        assert_eq!(data[9], 0x07);
        assert_eq!(data[10], 0x00);
        assert_eq!(data[11], 0x80);
    }

    #[test]
    fn test_serialize_null_values() {
        let mut table = Table::new(
            "Test",
            vec![
                Column::build("A").int16().nullable().build(),
                Column::build("B").int32().nullable().build(),
            ],
            false,
        );
        table.add_row(vec![Value::Null, Value::Null]).unwrap();

        let pool = StringPool::new(false);
        let data = table.serialize(&pool).unwrap();

        // NULL int16 = 0u16, NULL int32 = 0u32
        assert_eq!(data.len(), 6); // 2 + 4
        assert_eq!(u16::from_le_bytes([data[0], data[1]]), 0);
        assert_eq!(u32::from_le_bytes([data[2], data[3], data[4], data[5]]), 0);
    }

    #[test]
    fn test_serialize_string_columns() {
        let mut table = Table::new(
            "Test",
            vec![
                Column::build("Name").string(72).primary_key().build(),
            ],
            false,
        );

        let mut pool = StringPool::new(false);
        let id_a = pool.intern("Alpha");
        let id_b = pool.intern("Beta");

        table.add_row(vec![Value::Str("Alpha".into())]).unwrap();
        table.add_row(vec![Value::Str("Beta".into())]).unwrap();

        let data = table.serialize(&pool).unwrap();

        // 2 rows × 2 bytes (short string refs) = 4 bytes
        assert_eq!(data.len(), 4);

        // Values should be string pool IDs (sorted by ID)
        let val0 = u16::from_le_bytes([data[0], data[1]]);
        let val1 = u16::from_le_bytes([data[2], data[3]]);
        assert_eq!(val0, id_a as u16);
        assert_eq!(val1, id_b as u16);
    }

    #[test]
    fn test_serialize_long_string_refs() {
        let mut table = Table::new(
            "Test",
            vec![
                Column::build("Name").string(72).build(),
            ],
            true, // long_string_refs = true
        );

        let mut pool = StringPool::new(true);
        pool.intern("Hello");

        table.add_row(vec![Value::Str("Hello".into())]).unwrap();

        let data = table.serialize(&pool).unwrap();
        // Long string refs = 3 bytes per string ref (u16 + u8)
        assert_eq!(data.len(), 3);
    }

    #[test]
    fn test_serialize_uninterned_string_errors() {
        let mut table = Table::new(
            "Test",
            vec![
                Column::build("Name").string(72).build(),
            ],
            false,
        );
        table.add_row(vec![Value::Str("NotInterned".into())]).unwrap();

        let pool = StringPool::new(false); // Empty pool — string not interned
        let result = table.serialize(&pool);
        assert!(result.is_err());
    }

    #[test]
    fn test_row_sorting_by_primary_key() {
        let mut table = Table::new(
            "Test",
            vec![
                Column::build("Id").int32().primary_key().build(),
                Column::build("Val").int32().build(),
            ],
            false,
        );

        // Insert in reverse order
        table.add_row(vec![Value::Int(3), Value::Int(30)]).unwrap();
        table.add_row(vec![Value::Int(1), Value::Int(10)]).unwrap();
        table.add_row(vec![Value::Int(2), Value::Int(20)]).unwrap();

        let pool = StringPool::new(false);
        let data = table.serialize(&pool).unwrap();

        // Column-major: col Id (3 rows × 4 bytes) + col Val (3 rows × 4 bytes)
        // After sorting by Id: [1, 2, 3] and [10, 20, 30]
        // Values are XOR-encoded: decode with (raw ^ -0x80000000)
        let decode_i32 = |offset: usize| -> i32 {
            let raw = i32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
            raw ^ -0x80000000i32
        };

        assert_eq!(decode_i32(0), 1, "Ids should be sorted");
        assert_eq!(decode_i32(4), 2);
        assert_eq!(decode_i32(8), 3);

        assert_eq!(decode_i32(12), 10, "Values should follow sorted order");
        assert_eq!(decode_i32(16), 20);
        assert_eq!(decode_i32(20), 30);
    }
}
