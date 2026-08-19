//! velocity-msi - Clean-room MSI package generator
//!
//! Creates Windows Installer (MSI) packages with a from-scratch OLE V4 writer.
//! No dependency on cfb or rust-msi crates.

mod error;
pub mod ole;
mod string_pool;
mod summary;
mod table;
pub mod validate;

pub use error::{MsiError, Result};
pub use string_pool::StringPool;
pub use summary::SummaryInfo;
pub use table::{Column, ColumnType, Table, Value};
pub use validate::{validate_ole, validate_msi_semantics, MsiSemanticReport, MsiValidationInfo};

use std::collections::BTreeMap;
use std::io::Write;

/// MSI package builder
///
/// Creates Windows Installer (MSI) databases by managing tables, string pools,
/// and summary information, then assembling them into an OLE V4 compound file.
///
/// # Example
/// ```
/// use velocity_msi::{MsiBuilder, Column, Value};
///
/// let mut builder = MsiBuilder::new();
/// builder.set_title("My Product");
/// builder.set_author("My Company");
///
/// builder.create_table("Property", vec![
///     Column::build("Property").string(72).primary_key().build(),
///     Column::build("Value").string(255).nullable().build(),
/// ]).unwrap();
///
/// builder.insert_rows("Property", vec![
///     vec![Value::from("ProductName"), Value::from("My Product")],
/// ]).unwrap();
///
/// let msi_data = builder.build().unwrap();
/// ```
pub struct MsiBuilder {
    string_pool: StringPool,
    tables: BTreeMap<String, Table>,
    summary: SummaryInfo,
    long_string_refs: bool,
    /// Extra OLE streams to embed (e.g., cabinet files)
    extra_streams: Vec<ole::OleStream>,
}

impl MsiBuilder {
    /// Create a new MSI builder with default settings.
    ///
    /// Sets up the string pool, summary information with current timestamps,
    /// and an empty table set.
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        Self {
            string_pool: StringPool::new(false),
            tables: BTreeMap::new(),
            summary: {
                let mut s = SummaryInfo::new();
                s.created = Some(now);
                s.modified = Some(now);
                s.creating_app = Some("Velocity Installer".to_string());
                s
            },
            long_string_refs: false,
            extra_streams: Vec::new(),
        }
    }

    /// Set the product title in SummaryInformation.
    pub fn set_title(&mut self, title: &str) {
        self.summary.title = Some(title.to_string());
    }

    /// Set the product author in SummaryInformation.
    pub fn set_author(&mut self, author: &str) {
        self.summary.author = Some(author.to_string());
    }

    /// Set the product subject in SummaryInformation.
    pub fn set_subject(&mut self, subject: &str) {
        self.summary.subject = Some(subject.to_string());
    }

    /// Set the product comments in SummaryInformation.
    pub fn set_comments(&mut self, comments: &str) {
        self.summary.comments = Some(comments.to_string());
    }

    /// Set the template string (e.g., "x64;1033" for architecture and language).
    pub fn set_template(&mut self, arch: &str, language: u16) {
        self.summary.template = Some(format!("{};{}", arch, language));
    }

    /// Create a new table with the given column schema.
    ///
    /// Returns an error if a table with this name already exists.
    pub fn create_table(&mut self, name: &str, columns: Vec<Column>) -> Result<()> {
        if self.tables.contains_key(name) {
            return Err(MsiError::TableAlreadyExists(name.to_string()));
        }
        let table = Table::new(name, columns, self.long_string_refs);
        self.tables.insert(name.to_string(), table);
        Ok(())
    }

    /// Insert rows into a table. Strings are automatically interned into the string pool.
    ///
    /// Each row must have the same number of values as the table has columns.
    pub fn insert_rows(&mut self, table_name: &str, rows: Vec<Vec<Value>>) -> Result<()> {
        let table = self
            .tables
            .get_mut(table_name)
            .ok_or_else(|| MsiError::TableNotFound(table_name.to_string()))?;

        for row in rows {
            for value in &row {
                if let Value::Str(s) = value {
                    self.string_pool.intern(s);
                }
            }
            table.add_row(row)?;
        }
        Ok(())
    }

    /// Add an extra OLE stream to the MSI (e.g., cabinet files).
    /// The stream name is used as-is (no encoding applied).
    pub fn add_stream(&mut self, name: String, data: Vec<u8>) {
        self.extra_streams.push(ole::OleStream { name, data });
    }

    /// Build the MSI package and return the complete OLE V4 compound file bytes.
    ///
    /// This creates system tables (_Tables, _Columns, _Validation), serializes
    /// all table data, builds the string pool, and assembles everything into
    /// an OLE V4 compound file.
    pub fn build(&mut self) -> Result<Vec<u8>> {
        // Create system tables (interns all system strings)
        let mut all_tables = self.create_system_tables()?;

        // Add user tables
        for (name, table) in self.tables.iter() {
            all_tables.insert(name.clone(), table.clone());
        }

        // Collect all OLE streams
        let mut streams: Vec<ole::OleStream> = Vec::new();

        // Table streams (ALL tables get TABLE_PREFIX per MSI spec)
        for (name, table) in &all_tables {
            let stream_name = encode_stream_name(name, true);
            let data = table.serialize(&self.string_pool)?;
            streams.push(ole::OleStream {
                name: stream_name,
                data,
            });
        }

        // Summary Information stream
        let summary_data = self.summary.serialize()?;
        streams.push(ole::OleStream {
            name: "\u{0005}SummaryInformation".to_string(),
            data: summary_data,
        });

        // String pool streams
        let (pool_name, pool_data, data_name, data_bytes) = self.build_string_pool()?;
        streams.push(ole::OleStream {
            name: pool_name,
            data: pool_data,
        });
        streams.push(ole::OleStream {
            name: data_name,
            data: data_bytes,
        });

        // Extra streams (cabinets, etc.)
        for stream in &self.extra_streams {
            streams.push(stream.clone());
        }

        // Build the OLE V4 compound file
        Ok(ole::build_ole_file(&streams))
    }

    /// Create the system tables (_Tables, _Columns, _Validation)
    fn create_system_tables(&mut self) -> Result<BTreeMap<String, Table>> {
        let mut system_tables = BTreeMap::new();

        // _Tables - list of all user table names
        let mut tables_table = Table::new(
            "_Tables",
            vec![Column::build("Name").string(64).primary_key().build()],
            self.long_string_refs,
        );
        for name in self.tables.keys() {
            self.string_pool.intern(name);
            tables_table.add_row(vec![Value::Str(name.clone())])?;
        }
        system_tables.insert("_Tables".to_string(), tables_table);

        // _Columns - column metadata for all user tables
        let mut columns_table = Table::new(
            "_Columns",
            vec![
                Column::build("Table").string(64).primary_key().build(),
                Column::build("Number").int16().primary_key().build(),
                Column::build("Name").string(64).primary_key().build(),
                Column::build("Type").int16().build(),
            ],
            self.long_string_refs,
        );
        for (table_name, table) in &self.tables {
            for (col_num, col) in table.columns.iter().enumerate() {
                self.string_pool.intern(table_name);
                self.string_pool.intern(&col.name);
                columns_table.add_row(vec![
                    Value::Str(table_name.clone()),
                    Value::Int((col_num + 1) as i32),
                    Value::Str(col.name.clone()),
                    Value::Int(col.bitfield()),
                ])?;
            }
        }
        system_tables.insert("_Columns".to_string(), columns_table);

        // _Validation - validation rules for all user table columns
        let mut validation_table = Table::new(
            "_Validation",
            vec![
                Column::build("Table").string(64).primary_key().build(),
                Column::build("Column").string(64).primary_key().build(),
                Column::build("Nullable").string(4).build(),
                Column::build("MinValue").int32().nullable().build(),
                Column::build("MaxValue").int32().nullable().build(),
                Column::build("KeyTable").string(255).nullable().build(),
                Column::build("KeyColumn").int16().nullable().build(),
                Column::build("Category").string(32).nullable().build(),
                Column::build("Set").string(255).nullable().build(),
                Column::build("Description").string(255).nullable().build(),
            ],
            self.long_string_refs,
        );
        for (table_name, table) in &self.tables {
            for col in &table.columns {
                self.string_pool.intern(table_name);
                self.string_pool.intern(&col.name);
                let nullable = if col.nullable { "Y" } else { "N" };
                self.string_pool.intern(nullable);
                validation_table.add_row(vec![
                    Value::Str(table_name.clone()),
                    Value::Str(col.name.clone()),
                    Value::Str(nullable.to_string()),
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Null,
                ])?;
            }
        }
        system_tables.insert("_Validation".to_string(), validation_table);

        Ok(system_tables)
    }

    /// Build string pool data: returns (pool_name, pool_data, data_name, data_bytes)
    ///
    /// _StringPool format (per MSI spec):
    ///   Header: u32 = codepage (low 16 bits) | long_refs_flag (bit 31)
    ///   Per entry: u16 length + u16 refcount (4 bytes each)
    ///
    /// _StringData format:
    ///   Concatenated encoded string bytes in ID order (1-based, ID 0 = null)
    fn build_string_pool(&self) -> Result<(String, Vec<u8>, String, Vec<u8>)> {
        let mut strings: Vec<_> = self.string_pool.iter().collect();
        strings.sort_by_key(|&(_, id, _)| id);

        let win1252 = encoding_rs::WINDOWS_1252;

        // _StringPool stream
        let mut pool_data = Vec::new();
        // Header: codepage in low 16 bits, long string refs flag at bit 31
        let mut header: u32 = 1252; // Windows-1252 (standard MSI codepage)
        if self.string_pool.long_string_refs() {
            header |= 0x80000000;
        }
        pool_data.write_all(&header.to_le_bytes())?;
        for (text, _id, refcount) in &strings {
            let (encoded, _, _had_errors) = win1252.encode(text);
            // Entry: u16 length + u16 refcount (4 bytes per entry)
            pool_data.write_all(&(encoded.len() as u16).to_le_bytes())?;
            pool_data.write_all(&((*refcount).min(0xFFFF) as u16).to_le_bytes())?;
        }

        // _StringData stream - encoded string bytes in ID order
        let mut string_data = Vec::new();
        for (text, _id, _refcount) in &strings {
            let (encoded, _, _) = win1252.encode(text);
            string_data.write_all(&encoded)?;
        }

        // The Windows Installer uses specific obfuscated names for the string pool streams.
        let pool_name = "\u{4840}\u{3F3F}\u{4577}\u{446C}\u{3E6A}\u{44B2}\u{482F}".to_string();
        let data_name = "\u{4840}\u{3F3F}\u{4577}\u{446C}\u{3B6A}\u{45E4}\u{4824}".to_string();

        Ok((pool_name, pool_data, data_name, string_data))
    }
}

/// Encode a stream name using MSI's base-64 Unicode encoding.
///
/// `is_table=true`: system metadata tables get TABLE_PREFIX (\u{4840})
/// `is_table=false`: user tables and internal streams (no prefix)
pub(crate) fn encode_stream_name(name: &str, is_table: bool) -> String {
    let mut output = String::new();
    if is_table {
        output.push('\u{4840}');
    }
    let mut chars = name.chars().peekable();
    while let Some(ch1) = chars.next() {
        if let Some(value1) = to_b64(ch1) {
            if let Some(&ch2) = chars.peek() {
                if let Some(value2) = to_b64(ch2) {
                    let encoded = 0x3800 + (value2 << 6) + value1;
                    output.push(char::from_u32(encoded).unwrap());
                    chars.next();
                    continue;
                }
            }
            // Last encodable character with no pair — encode singly
            let encoded = 0x4800 + value1;
            output.push(char::from_u32(encoded).unwrap());
        } else {
            output.push(ch1);
        }
    }
    output
}

fn to_b64(ch: char) -> Option<u32> {
    if ch.is_ascii_digit() {
        Some(ch as u32 - '0' as u32)
    } else if ch.is_ascii_uppercase() {
        Some(10 + ch as u32 - 'A' as u32)
    } else if ch.is_ascii_lowercase() {
        Some(36 + ch as u32 - 'a' as u32)
    } else if ch == '.' {
        Some(62)
    } else if ch == '_' {
        Some(63)
    } else {
        None
    }
}

impl Default for MsiBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_simple_msi() {
        let mut builder = MsiBuilder::new();
        builder.set_title("Test MSI");
        builder.set_author("Test Author");

        builder
            .create_table(
                "Property",
                vec![
                    Column::build("Property").string(72).primary_key().build(),
                    Column::build("Value").string(255).nullable().build(),
                ],
            )
            .unwrap();

        builder
            .insert_rows(
                "Property",
                vec![vec![
                    Value::from("ProductName"),
                    Value::from("Test Product"),
                ]],
            )
            .unwrap();

        let msi_data = builder.build().unwrap();
        assert!(!msi_data.is_empty());
    }

    #[test]
    fn test_encode_stream_name() {
        let tables_enc = encode_stream_name("_Tables", true);
        let cols_enc = encode_stream_name("_Columns", true);
        eprintln!("_Tables encoded: {:?} ({} chars)", tables_enc, tables_enc.chars().count());
        eprintln!("_Columns encoded: {:?} ({} chars)", cols_enc, cols_enc.chars().count());
        assert_eq!(
            encode_stream_name("_Columns", true),
            "\u{4840}\u{3b3f}\u{43f2}\u{4438}\u{45b1}"
        );
        assert_eq!(
            encode_stream_name("_Tables", true),
            "\u{4840}\u{3f7f}\u{4164}\u{422f}\u{4836}"
        );
    }

    /// Integration test: generate a full MSI and verify with validate_ole.
    #[test]
    fn test_full_msi_validation() {
        let mut builder = MsiBuilder::new();
        builder.set_title("Integration Test");
        builder.set_author("Velocity");
        builder.set_template("x64", 1033);

        builder
            .create_table(
                "Property",
                vec![
                    Column::build("Property").string(72).primary_key().build(),
                    Column::build("Value").string(255).nullable().build(),
                ],
            )
            .unwrap();

        builder
            .insert_rows(
                "Property",
                vec![
                    vec![Value::from("ProductName"), Value::from("Test Product")],
                    vec![Value::from("ProductVersion"), Value::from("1.0.0")],
                    vec![Value::from("Manufacturer"), Value::from("Test Corp")],
                ],
            )
            .unwrap();

        let msi_data = builder.build().unwrap();

        // Validate the OLE structure
        let info = validate_ole(&msi_data).unwrap();
        assert!(info.valid_ole, "OLE structure should be valid");
        assert!(info.has_summary, "Should have SummaryInformation");
        assert!(info.has_string_pool, "Should have string pool");
        assert!(
            info.stream_names.len() >= 5,
            "Should have at least 5 streams (SummaryInfo, 2 string pool, 2+ tables), got {}",
            info.stream_names.len()
        );
    }
}
