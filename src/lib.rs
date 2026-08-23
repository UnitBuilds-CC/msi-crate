//! velocity-msi - MSI package generator
//!
//! Creates Windows Installer (MSI) packages using a custom OLE V3 compound
//! file writer and custom MSI table serialization.  No external dependencies
//! for the OLE layer — the entire compound file is built from scratch.

mod error;
pub mod ole;
mod string_pool;
mod summary;
mod table;
pub mod validate;
pub mod cabinet;

pub use error::{MsiError, Result};
pub use string_pool::StringPool;
pub use summary::SummaryInfo;
pub use table::{Column, ColumnType, Table, Value};
pub use validate::{validate_ole, validate_msi_semantics, MsiSemanticReport, MsiValidationInfo};
pub use cabinet::{CabinetFile, build_cabinet};

use std::collections::BTreeMap;
use std::io::Write;

/// MSI package builder
///
/// Creates Windows Installer (MSI) databases by managing tables, string pools,
/// and summary information, then assembling them into an OLE V3 compound file.
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
    /// Whether to include _Validation table (default: true)
    include_validation: bool,
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
                // Generate a pseudo-UUID for the Revision Number.
                // msiexec REQUIRES this property (PID 9) to open the MSI.
                s.rev_number = Some(Self::generate_uuid(&now));
                // PID 14 (Security) defaults to 405 in SummaryInfo::new()
                // PID 15 (WordCount) defaults to 2
                s
            },
            long_string_refs: false,
            extra_streams: Vec::new(),
            include_validation: true,
        }
    }

    /// Generate a UUID string from a timestamp (pseudo-random, but unique enough).
    fn generate_uuid(now: &chrono::DateTime<chrono::Utc>) -> String {
        let ts = now.timestamp();
        let ns = now.timestamp_subsec_nanos();
        // Mix bits to get reasonable distribution
        let a = (ts as u32).wrapping_mul(0x5bd1e995).wrapping_add(ns);
        let b = (ts as u32).wrapping_mul(0x1b873593) ^ ns;
        let c = (ts as u16).wrapping_mul(0x85eb) ^ (ns >> 16) as u16;
        let d = ((ts >> 16) as u16).wrapping_add(ns as u16);
        format!(
            "{{{:08X}-{:04X}-{:04X}-{:04X}-{:04X}{:08X}}}",
            a, b >> 16, (b & 0xFFFF) | 0x4000, // version 4
            (c & 0x3FFF) | 0x8000, // variant 1
            d, ts as u32
        )
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

    /// Disable _Validation table generation (for testing/debugging).
    pub fn set_include_validation(&mut self, include: bool) {
        self.include_validation = include;
    }

    /// Build the MSI package and return the complete OLE compound file bytes.
    ///
    /// This creates system tables (_Tables, _Columns, _Validation), serializes
    /// all table data, builds the string pool, and assembles everything into
    /// an OLE V3 compound file using our custom OLE writer.
    pub fn build(&mut self) -> Result<Vec<u8>> {
        // Create system tables (interns all system strings)
        let mut all_tables = self.create_system_tables()?;

        // Remove _Validation if disabled
        if !self.include_validation {
            all_tables.remove("_Validation");
        }

        // Add user tables, filtering out any with 0 rows.
        // Empty tables produce 0-byte streams which msiexec rejects.
        for (name, table) in self.tables.iter() {
            if table.row_count() > 0 {
                all_tables.insert(name.clone(), table.clone());
            }
        }

        // Add _Validation entries for user tables (skip empty tables)
        if self.include_validation {
            if let Some(validation) = all_tables.get_mut("_Validation") {
                for (table_name, table) in &self.tables {
                    if table.row_count() == 0 {
                        continue;
                    }
                    for col in &table.columns {
                        self.string_pool.intern(table_name);
                        self.string_pool.intern(&col.name);
                        let nullable = if col.nullable { "Y" } else { "N" };
                        self.string_pool.intern(nullable);
                        let category_val = col.category.as_ref().map(|c| {
                            self.string_pool.intern(c);
                            Value::Str(c.clone())
                        });
                        validation.add_row(vec![
                            Value::Str(table_name.clone()),
                            Value::Str(col.name.clone()),
                            Value::Str(nullable.to_string()),
                            Value::Null, Value::Null, Value::Null, Value::Null,
                            category_val.unwrap_or(Value::Null), Value::Null, Value::Null,
                        ])?;
                    }
                }
            }
        }

        // Populate _Tables and _Columns with entries for ALL tables
        Self::populate_system_table_metadata(&mut all_tables, &mut self.string_pool)?;

        // Reindex the string pool so IDs are assigned in alphabetical order.
        // This must happen after ALL interning but before ANY serialization.
        // The msi crate reference uses BTreeMap which naturally assigns IDs
        // in alphabetical order. We match that behavior here.
        self.string_pool.reindex();

        // Collect all streams
        let mut ole_streams: Vec<ole::OleStream> = Vec::new();

        // Table streams (ALL tables get TABLE_PREFIX per MSI spec)
        for (name, table) in &all_tables {
            let stream_name = encode_stream_name(name, true);
            let data = table.serialize(&self.string_pool)?;
            ole_streams.push(ole::OleStream {
                name: stream_name,
                data,
            });
        }

        // Summary Information stream
        let summary_data = self.summary.serialize()?;
        ole_streams.push(ole::OleStream {
            name: "\u{0005}SummaryInformation".to_string(),
            data: summary_data,
        });

        // String pool streams
        let (pool_name, pool_data, data_name, data_bytes) = self.build_string_pool()?;
        ole_streams.push(ole::OleStream {
            name: pool_name,
            data: pool_data,
        });
        ole_streams.push(ole::OleStream {
            name: data_name,
            data: data_bytes,
        });

        // Extra streams (cabinets, etc.)
        // Cabinet/binary streams are encoded with is_table=false (no TABLE_PREFIX),
        // matching the msi crate's write_stream() behavior.
        for stream in &self.extra_streams {
            ole_streams.push(ole::OleStream {
                name: encode_stream_name(&stream.name, false),
                data: stream.data.clone(),
            });
        }

        // Build the OLE compound file using our custom V3 writer.
        // This is 100% in-house with no external OLE dependencies.
        // The custom writer produces V3 format (512-byte sectors) as required by MSI.
        // The MSI CLSID is set on the root directory entry so msiexec recognizes the package.
        Ok(ole::build_ole_file(&ole_streams))
    }

    /// Create the system tables (_Tables, _Columns, _Validation)
    ///
    /// Per the msi crate reference implementation:
    /// - _Tables and _Columns are created directly (not via create_table),
    ///   so they do NOT get _Validation entries.
    /// - _Validation IS created via create_table, so it gets _Validation entries
    ///   AND is listed in the _Tables table.
    /// - _Columns entries for ALL tables (system + user) are added in build()
    ///   after all tables are known.
    fn create_system_tables(&mut self) -> Result<BTreeMap<String, Table>> {
        let mut system_tables = BTreeMap::new();

        // _Tables - starts empty; table names are added in build()
        let tables_table = Table::new(
            "_Tables",
            vec![Column::build("Name").string(64).primary_key().build()],
            self.long_string_refs,
        );
        system_tables.insert("_Tables".to_string(), tables_table);

        // _Columns - starts empty; column defs are added in build()
        // Per msi crate reference: PK = (Table, Number), Name is NOT PK.
        let columns_table = Table::new(
            "_Columns",
            vec![
                Column::build("Table").string(64).primary_key().build(),
                Column::build("Number").int16().primary_key().build(),
                Column::build("Name").string(64).build(),
                Column::build("Type").int16().build(),
            ],
            self.long_string_refs,
        );
        system_tables.insert("_Columns".to_string(), columns_table);

        // _Validation - validation rules for _Validation and user tables.
        // NOTE: _Tables and _Columns do NOT get _Validation entries
        // (matching the msi crate's behavior where they bypass create_table).
        // Column sizes match the msi crate's make_validation_columns():
        //   Table/Column use id_string(32), not string(64).
        let mut validation_table = Table::new(
            "_Validation",
            vec![
                Column::build("Table").string(32).primary_key().category("Identifier").build(),
                Column::build("Column").string(32).primary_key().category("Identifier").build(),
                Column::build("Nullable").string(4).build(),
                Column::build("MinValue").int32().nullable().build(),
                Column::build("MaxValue").int32().nullable().build(),
                Column::build("KeyTable").string(255).nullable().category("Identifier").build(),
                Column::build("KeyColumn").int16().nullable().build(),
                Column::build("Category").string(32).nullable().build(),
                Column::build("Set").string(255).nullable().category("Text").build(),
                Column::build("Description").string(255).nullable().category("Text").build(),
            ],
            self.long_string_refs,
        );

        // _Validation entries for the _Validation table itself
        // Categories and Set values match the msi crate's make_validation_columns()
        self.string_pool.intern("_Validation");
        self.string_pool.intern("Table");
        self.string_pool.intern("Column");
        self.string_pool.intern("Nullable");
        self.string_pool.intern("MinValue");
        self.string_pool.intern("MaxValue");
        self.string_pool.intern("KeyTable");
        self.string_pool.intern("KeyColumn");
        self.string_pool.intern("Category");
        self.string_pool.intern("Set");
        self.string_pool.intern("Description");
        self.string_pool.intern("N");
        self.string_pool.intern("Y");
        // Category names used in _Validation entries
        self.string_pool.intern("Identifier");
        self.string_pool.intern("Text");
        // Set values for Nullable column
        self.string_pool.intern("Y;N");
        // Full list of valid categories for the Category column's Set
        let all_categories = "Text;UpperCase;LowerCase;Integer;DoubleInteger;TimeDate;Identifier;Property;Filename;WildCardFilename;Path;Paths;AnyPath;DefaultDir;RegPath;Formatted;FormattedSDDLText;Template;Condition;GUID;Version;Language;Binary;CustomSource;Cabinet;Shortcut";
        self.string_pool.intern(all_categories);

        // _Validation.Table: id_string(32) → category=Identifier
        validation_table.add_row(vec![
            Value::Str("_Validation".to_string()),
            Value::Str("Table".to_string()),
            Value::Str("N".to_string()),
            Value::Null, Value::Null, Value::Null, Value::Null,
            Value::Str("Identifier".to_string()), Value::Null, Value::Null,
        ])?;
        // _Validation.Column: id_string(32) → category=Identifier
        validation_table.add_row(vec![
            Value::Str("_Validation".to_string()),
            Value::Str("Column".to_string()),
            Value::Str("N".to_string()),
            Value::Null, Value::Null, Value::Null, Value::Null,
            Value::Str("Identifier".to_string()), Value::Null, Value::Null,
        ])?;
        // _Validation.Nullable: enum_values(["Y","N"]) → Set="Y;N"
        validation_table.add_row(vec![
            Value::Str("_Validation".to_string()),
            Value::Str("Nullable".to_string()),
            Value::Str("N".to_string()),
            Value::Null, Value::Null, Value::Null, Value::Null,
            Value::Null, Value::Str("Y;N".to_string()), Value::Null,
        ])?;
        // _Validation.MinValue: nullable int32
        validation_table.add_row(vec![
            Value::Str("_Validation".to_string()),
            Value::Str("MinValue".to_string()),
            Value::Str("Y".to_string()),
            Value::Null, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null,
        ])?;
        // _Validation.MaxValue: nullable int32
        validation_table.add_row(vec![
            Value::Str("_Validation".to_string()),
            Value::Str("MaxValue".to_string()),
            Value::Str("Y".to_string()),
            Value::Null, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null,
        ])?;
        // _Validation.KeyTable: id_string(255) → category=Identifier
        validation_table.add_row(vec![
            Value::Str("_Validation".to_string()),
            Value::Str("KeyTable".to_string()),
            Value::Str("Y".to_string()),
            Value::Null, Value::Null, Value::Null, Value::Null,
            Value::Str("Identifier".to_string()), Value::Null, Value::Null,
        ])?;
        // _Validation.KeyColumn: nullable int16
        validation_table.add_row(vec![
            Value::Str("_Validation".to_string()),
            Value::Str("KeyColumn".to_string()),
            Value::Str("Y".to_string()),
            Value::Null, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null,
        ])?;
        // _Validation.Category: enum_values(all_categories) → Set=long list
        validation_table.add_row(vec![
            Value::Str("_Validation".to_string()),
            Value::Str("Category".to_string()),
            Value::Str("Y".to_string()),
            Value::Null, Value::Null, Value::Null, Value::Null,
            Value::Null, Value::Str(all_categories.to_string()), Value::Null,
        ])?;
        // _Validation.Set: text_string(255) → category=Text
        validation_table.add_row(vec![
            Value::Str("_Validation".to_string()),
            Value::Str("Set".to_string()),
            Value::Str("Y".to_string()),
            Value::Null, Value::Null, Value::Null, Value::Null,
            Value::Str("Text".to_string()), Value::Null, Value::Null,
        ])?;
        // _Validation.Description: text_string(255) → category=Text
        validation_table.add_row(vec![
            Value::Str("_Validation".to_string()),
            Value::Str("Description".to_string()),
            Value::Str("Y".to_string()),
            Value::Null, Value::Null, Value::Null, Value::Null,
            Value::Str("Text".to_string()), Value::Null, Value::Null,
        ])?;

        system_tables.insert("_Validation".to_string(), validation_table);

        Ok(system_tables)
    }

    /// Populate _Tables and _Columns with entries for user tables + _Validation.
    /// _Tables and _Columns themselves are NOT listed (matching msi crate behavior).
    fn populate_system_table_metadata(
        all_tables: &mut BTreeMap<String, Table>,
        string_pool: &mut StringPool,
    ) -> Result<()> {
        // Collect table names and column definitions for user tables + _Validation only
        // _Tables and _Columns must NOT list themselves (they are metadata-only).
        let table_info: Vec<(String, Vec<Column>)> = all_tables
            .iter()
            .filter(|(name, _)| *name != "_Tables" && *name != "_Columns")
            .map(|(name, table)| (name.clone(), table.columns.clone()))
            .collect();

        // Add ALL table names to _Tables
        if let Some(tables_table) = all_tables.get_mut("_Tables") {
            for (name, _) in &table_info {
                string_pool.intern(name);
                tables_table.add_row(vec![Value::Str(name.clone())])?;
            }
        }

        // Add column definitions for ALL tables to _Columns
        if let Some(columns_table) = all_tables.get_mut("_Columns") {
            for (table_name, columns) in &table_info {
                for (col_num, col) in columns.iter().enumerate() {
                    string_pool.intern(table_name);
                    string_pool.intern(&col.name);
                    columns_table.add_row(vec![
                        Value::Str(table_name.clone()),
                        Value::Int((col_num + 1) as i32),
                        Value::Str(col.name.clone()),
                        Value::Int(col.bitfield()),
                    ])?;
                }
            }
        }

        Ok(())
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
        // After reindex(), BTreeMap iterates in alphabetical order which
        // equals ID order (IDs assigned sequentially in alphabetical order).
        let strings: Vec<_> = self.string_pool.iter().collect();

        // Use Windows-1252 codepage (1252) - standard for MSI packages.
        let mut pool_data = Vec::new();
        // Header: codepage in low 16 bits, long string refs flag at bit 31
        let mut header: u32 = 1252; // Windows-1252 codepage
        if self.string_pool.long_string_refs() {
            header |= 0x80000000;
        }
        pool_data.write_all(&header.to_le_bytes())?;
        for (text, _id, refcount) in &strings {
            let encoded = StringPool::encode_win1252(text);
            // Entry: u16 length + u16 refcount
            // Per msi crate reference: length = encoded byte count (NO null terminator)
            let len = encoded.len() as u16;
            pool_data.write_all(&len.to_le_bytes())?;
            pool_data.write_all(&((*refcount).min(0xFFFF) as u16).to_le_bytes())?;
        }

        // _StringData stream - concatenated encoded strings in ID order.
        // Per msi crate reference: NO null terminators between strings.
        // The length field in _StringPool = exact encoded byte count.
        let mut string_data = Vec::new();
        for (text, _id, _refcount) in &strings {
            string_data.write_all(&StringPool::encode_win1252(text))?;
        }

        // Use the standard stream name encoding (same as msi crate reference)
        let pool_name = encode_stream_name("_StringPool", true);
        let data_name = encode_stream_name("_StringData", true);

        Ok((pool_name, pool_data, data_name, string_data))
    }
}

/// Encode a stream name using MSI's base-64 Unicode encoding.
///
/// `is_table=true`: system metadata tables get TABLE_PREFIX (\u{4840})
/// `is_table=false`: user tables and internal streams (no prefix)
pub fn encode_stream_name(name: &str, is_table: bool) -> String {
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
        let _tables_enc = encode_stream_name("_Tables", true);
        let _cols_enc = encode_stream_name("_Columns", true);
        let _val_enc = encode_stream_name("_Validation", true);
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
