//! OLE Compound File reader for MSI validation.
//!
//! Parses the OLE V4 (or V3) compound file structure to extract stream names
//! and verify structural integrity. This is a minimal read-only parser used
//! solely for validation — it does not reconstruct stream data.

use crate::error::{MsiError, Result};

const OLE_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
const FREE_SECT: u32 = 0xFFFF_FFFF;
const ENDOFCHAIN: u32 = 0xFFFF_FFFE;
const DIR_ENTRY_SIZE: usize = 128;

/// Parsed OLE header with key offsets.
struct OleHeader {
    _major_version: u16,
    _sector_shift: u16,
    sector_size: usize,
    header_size: usize,
    num_dir_sectors: u32,
    num_fat_sectors: u32,
    first_dir_sector: u32,
    first_minifat_sector: u32,
    #[allow(dead_code)]
    num_minifat_sectors: u32,
    mini_stream_cutoff: u32,
    difat_entries: Vec<u32>,
}

/// Result of OLE-level validation of an MSI file.
#[derive(Debug)]
pub struct MsiValidationInfo {
    /// All stream names found in the OLE directory (decoded to UTF-8 where possible).
    pub stream_names: Vec<String>,
    /// Whether the OLE header is structurally valid.
    pub valid_ole: bool,
    /// Whether the \x05SummaryInformation stream is present.
    pub has_summary: bool,
    /// Whether string pool streams are present.
    pub has_string_pool: bool,
    /// Names of table streams found (decoded from MSI base-64 encoding).
    pub table_streams: Vec<String>,
}

/// Validate the OLE structure of an MSI file and extract stream metadata.
pub fn validate_ole(data: &[u8]) -> Result<MsiValidationInfo> {
    // Check minimum size for header
    if data.len() < 512 {
        return Ok(MsiValidationInfo {
            stream_names: Vec::new(),
            valid_ole: false,
            has_summary: false,
            has_string_pool: false,
            table_streams: Vec::new(),
        });
    }

    // Check magic bytes
    if data[0..8] != OLE_MAGIC {
        return Ok(MsiValidationInfo {
            stream_names: Vec::new(),
            valid_ole: false,
            has_summary: false,
            has_string_pool: false,
            table_streams: Vec::new(),
        });
    }

    let header = parse_header(data)?;
    let sector_size = header.sector_size;

    // Read FAT entries
    let fat = read_fat(data, &header, sector_size)?;

    // Read directory entries
    let dir_entries = read_directory(data, &header, &fat, sector_size)?;

    // Extract stream names
    let mut stream_names = Vec::new();
    let mut has_summary = false;
    let mut has_string_pool = false;
    let mut table_streams = Vec::new();

    for entry in &dir_entries {
        let name = &entry.name;
        if name.is_empty() || entry.obj_type == 5 {
            // Skip empty entries and root entry
            if entry.obj_type == 5 {
                continue;
            }
            continue;
        }

        // Check for SummaryInformation (starts with \x05)
        if name.starts_with('\u{0005}') && name.contains("SummaryInformation") {
            has_summary = true;
        }

        // Check for string pool streams (high Unicode codepoints)
        // The _StringPool stream starts with \u{4840}\u{3F3F}...
        if name.starts_with('\u{4840}') {
            has_string_pool = true;
            // Count how many string pool streams (expect 2: pool + data)
        }

        // Check for table streams (start with \u{4840} for system tables,
        // or are encoded user table names)
        // System table streams start with the TABLE_PREFIX \u{4840}
        // but string pool streams also start with that.
        // Table streams that are NOT string pool:
        if name.starts_with('\u{4840}') && !is_string_pool_stream(name) {
            // Try to decode the table name
            if let Some(table_name) = decode_stream_name(name) {
                table_streams.push(table_name);
            }
        }

        stream_names.push(name.clone());
    }

    Ok(MsiValidationInfo {
        stream_names,
        valid_ole: true,
        has_summary,
        has_string_pool,
        table_streams,
    })
}

fn is_string_pool_stream(name: &str) -> bool {
    // _StringPool encoded: \u{4840}\u{3F3F}\u{4577}\u{446C}\u{3E6A}\u{44B2}\u{482F}
    // _StringData encoded: \u{4840}\u{3F3F}\u{4577}\u{446C}\u{3B6A}\u{45E4}\u{4824}
    let pool_prefix = "\u{4840}\u{3F3F}\u{4577}\u{446C}";
    name.starts_with(pool_prefix)
}

fn parse_header(data: &[u8]) -> Result<OleHeader> {
    let major_version = read_u16(data, 26);
    let sector_shift = read_u16(data, 30);
    let sector_size = 1usize << sector_shift;
    // V3: 512-byte header, V4: 4096-byte header
    let header_size = if major_version >= 4 { 4096 } else { 512 };

    let num_dir_sectors = read_u32(data, 40);
    let num_fat_sectors = read_u32(data, 44);
    let first_dir_sector = read_u32(data, 48);
    let first_minifat_sector = read_u32(data, 60);
    let num_minifat_sectors = read_u32(data, 64);
    let mini_stream_cutoff = read_u32(data, 56);

    // Read DIFAT entries from header (offset 76, count depends on version)
    // V3: 4 entries (header=512), V4: 109 entries (header=4096)
    let difat_count = (header_size - 76) / 4;
    let mut difat_entries = Vec::with_capacity(difat_count);
    for i in 0..difat_count {
        let val = read_u32(data, 76 + i * 4);
        if val != FREE_SECT {
            difat_entries.push(val);
        }
    }

    Ok(OleHeader {
        _major_version: major_version,
        _sector_shift: sector_shift,
        sector_size,
        header_size,
        num_dir_sectors,
        num_fat_sectors,
        first_dir_sector,
        first_minifat_sector,
        num_minifat_sectors,
        mini_stream_cutoff,
        difat_entries,
    })
}

fn read_fat(data: &[u8], header: &OleHeader, sector_size: usize) -> Result<Vec<u32>> {
    let entries_per_sector = sector_size / 4;
    let total_entries = header.num_fat_sectors as usize * entries_per_sector;
    let mut fat = vec![FREE_SECT; total_entries];

    for (fat_idx, &fat_sector) in header.difat_entries.iter().enumerate() {
        let sector_start = sector_offset(fat_sector as usize, sector_size, header.header_size);
        if sector_start + sector_size > data.len() {
            return Err(MsiError::EncodingError(format!(
                "FAT sector {} at offset {} exceeds file size {}",
                fat_sector,
                sector_start,
                data.len()
            )));
        }
        let base = fat_idx * entries_per_sector;
        for j in 0..entries_per_sector {
            if base + j < total_entries {
                fat[base + j] = read_u32(data, sector_start + j * 4);
            }
        }
    }

    Ok(fat)
}

struct DirEntry {
    name: String,
    obj_type: u8,
    start_sector: u32,
    stream_size: u64,
}

fn read_directory(
    data: &[u8],
    header: &OleHeader,
    fat: &[u32],
    sector_size: usize,
) -> Result<Vec<DirEntry>> {
    let entries_per_sector = sector_size / DIR_ENTRY_SIZE;
    let mut entries = Vec::new();

    // Follow directory sector chain
    let mut sector = header.first_dir_sector;
    let mut sectors_read = 0u32;
    while sector != ENDOFCHAIN && sector != FREE_SECT {
        if sectors_read >= header.num_dir_sectors {
            break; // Prevent infinite loops
        }
        let base = sector_offset(sector as usize, sector_size, header.header_size);
        if base + sector_size > data.len() {
            break;
        }

        for i in 0..entries_per_sector {
            let off = base + i * DIR_ENTRY_SIZE;
            if off + DIR_ENTRY_SIZE > data.len() {
                break;
            }

            let obj_type = data[off + 66];
            if obj_type == 0 {
                continue; // Empty entry
            }

            // Read name (UTF-16LE, up to 64 bytes = 32 chars)
            let name_len_with_null = read_u16(data, off + 64) as usize;
            // name_len_with_null includes null terminator, in bytes
            let name_bytes = name_len_with_null.saturating_sub(2);
            let name_bytes = name_bytes.min(64); // Max 64 bytes of name data

            let mut name = String::new();
            for j in (0..name_bytes).step_by(2) {
                if off + j + 1 < data.len() {
                    let ch = read_u16(data, off + j);
                    if let Some(c) = char::from_u32(ch as u32) {
                        name.push(c);
                    }
                }
            }

            let start_sector = read_u32(data, off + 116);
            let stream_size = read_u64(data, off + 120);

            entries.push(DirEntry {
                name,
                obj_type,
                start_sector,
                stream_size,
            });
        }

        // Follow chain
        let next = if (sector as usize) < fat.len() {
            fat[sector as usize]
        } else {
            ENDOFCHAIN
        };
        sector = next;
        sectors_read += 1;
    }

    Ok(entries)
}

// ── Stream data reading ─────────────────────────────────────────────────

/// Read data from a regular sector chain.
fn read_sector_chain(
    data: &[u8],
    header: &OleHeader,
    fat: &[u32],
    start_sector: u32,
    size: usize,
) -> Vec<u8> {
    let sector_size = header.sector_size;
    let mut result = Vec::with_capacity(size);
    let mut current = start_sector;
    let mut remaining = size;

    while remaining > 0
        && current != ENDOFCHAIN
        && current != FREE_SECT
        && (current as usize) < fat.len()
    {
        let off = sector_offset(current as usize, sector_size, header.header_size);
        let to_read = remaining.min(sector_size);
        if off + to_read <= data.len() {
            result.extend_from_slice(&data[off..off + to_read]);
        }
        remaining = remaining.saturating_sub(sector_size);
        current = fat[current as usize];
    }

    result.truncate(size);
    result
}

/// Read data from the mini-stream (via MiniFAT chain).
fn read_mini_stream(
    data: &[u8],
    header: &OleHeader,
    _fat: &[u32],
    mini_container: &[u8],
    start_mini_sector: u32,
    size: usize,
) -> Vec<u8> {
    let mini_sector_size = 64; // Always 64 bytes for mini-sectors
    let minifat_base = sector_offset(
        header.first_minifat_sector as usize,
        header.sector_size,
        header.header_size,
    );

    let mut result = Vec::with_capacity(size);
    let mut current = start_mini_sector;
    let mut remaining = size;
    let mut iterations = 0u32;

    while remaining > 0
        && current != ENDOFCHAIN
        && current != FREE_SECT
        && iterations < 100_000
    {
        let off = current as usize * mini_sector_size;
        let to_read = remaining.min(mini_sector_size);
        if off + to_read <= mini_container.len() {
            result.extend_from_slice(&mini_container[off..off + to_read]);
        }
        remaining = remaining.saturating_sub(mini_sector_size);

        // Read MiniFAT entry
        let fat_off = minifat_base + current as usize * 4;
        if fat_off + 4 <= data.len() {
            current = read_u32(data, fat_off);
        } else {
            break;
        }
        iterations += 1;
    }

    result.truncate(size);
    result
}

/// Read a specific stream's data from the OLE file.
fn read_stream_by_entry(
    data: &[u8],
    header: &OleHeader,
    fat: &[u32],
    entry: &DirEntry,
    root_entry: Option<&DirEntry>,
) -> Vec<u8> {
    let size = entry.stream_size as usize;
    if size == 0 {
        return Vec::new();
    }

    if size < header.mini_stream_cutoff as usize {
        // Mini-stream: need root entry's mini-stream container
        if let Some(root) = root_entry {
            let mini_container = read_sector_chain(
                data,
                header,
                fat,
                root.start_sector,
                root.stream_size as usize,
            );
            read_mini_stream(data, header, fat, &mini_container, entry.start_sector, size)
        } else {
            Vec::new()
        }
    } else {
        read_sector_chain(data, header, fat, entry.start_sector, size)
    }
}

// ── String pool parsing ─────────────────────────────────────────────────

/// Parse the MSI string pool streams and return an ID→text mapping.
fn parse_string_pool(
    pool_data: &[u8],
    data_bytes: &[u8],
) -> std::collections::HashMap<u32, String> {
    let mut map = std::collections::HashMap::new();
    if pool_data.len() < 4 {
        return map;
    }

    let _codepage = read_u32(pool_data, 0);
    let mut idx = 4usize;
    let mut data_offset = 0usize;
    let mut id = 1u32; // ID 0 is reserved for empty/null

    while idx + 4 <= pool_data.len() {
        let str_len = read_u16(pool_data, idx) as usize;
        let _refcount = read_u16(pool_data, idx + 2);
        idx += 4;

        if data_offset + str_len <= data_bytes.len() {
            // String data is Windows-1252 encoded
            let raw = &data_bytes[data_offset..data_offset + str_len];
            let text = String::from_utf8_lossy(raw).to_string();
            map.insert(id, text);
            data_offset += str_len;
        }

        id += 1;
    }

    map
}

// ── MSI semantic validation ─────────────────────────────────────────────

/// Semantic validation report for an MSI database.
#[derive(Debug)]
pub struct MsiSemanticReport {
    /// User table names extracted from the _Tables stream.
    pub user_tables: Vec<String>,
    /// Table names that have encoded streams in the OLE file.
    pub table_streams_found: Vec<String>,
    /// Column definitions extracted from the _Columns stream: (table, col_number, col_name, bitfield).
    pub columns: Vec<(String, i16, String, i32)>,
    /// Whether every user table in _Tables has a corresponding stream.
    pub all_tables_have_streams: bool,
    /// Whether the _Columns entries are consistent with the table list.
    pub columns_consistent: bool,
    /// Number of strings in the string pool.
    pub string_pool_size: usize,
}

/// Perform semantic validation of an MSI database.
///
/// This reads the string pool, _Tables, and _Columns streams to verify
/// internal consistency of the MSI database structure.
pub fn validate_msi_semantics(data: &[u8]) -> Result<MsiSemanticReport> {
    if data.len() < 512 || data[0..8] != OLE_MAGIC {
        return Err(MsiError::EncodingError("Not a valid OLE file".into()));
    }

    let header = parse_header(data)?;
    let sector_size = header.sector_size;
    let fat = read_fat(data, &header, sector_size)?;
    let dir_entries = read_directory(data, &header, &fat, sector_size)?;

    // Find root entry (obj_type == 5)
    let root_entry = dir_entries.iter().find(|e| e.obj_type == 5);

    // Build a map of stream name → DirEntry for non-root entries
    let stream_map: std::collections::HashMap<String, &DirEntry> = dir_entries
        .iter()
        .filter(|e| e.obj_type == 2) // Stream objects
        .map(|e| (e.name.clone(), e))
        .collect();

    // Find string pool streams (they have known encoded names)
    let pool_name =
        "\u{4840}\u{3F3F}\u{4577}\u{446C}\u{3E6A}\u{44B2}\u{482F}";
    let data_name =
        "\u{4840}\u{3F3F}\u{4577}\u{446C}\u{3B6A}\u{45E4}\u{4824}";

    let pool_entry = stream_map.get(pool_name);
    let data_entry = stream_map.get(data_name);

    let (pool_entry, data_entry) = match (pool_entry, data_entry) {
        (Some(p), Some(d)) => (p, d),
        _ => {
            return Ok(MsiSemanticReport {
                user_tables: Vec::new(),
                table_streams_found: Vec::new(),
                columns: Vec::new(),
                all_tables_have_streams: false,
                columns_consistent: false,
                string_pool_size: 0,
            });
        }
    };

    // Read string pool data
    let pool_data =
        read_stream_by_entry(data, &header, &fat, pool_entry, root_entry);
    let string_data =
        read_stream_by_entry(data, &header, &fat, data_entry, root_entry);
    let string_map = parse_string_pool(&pool_data, &string_data);
    let string_pool_size = string_map.len();

    // Find _Tables stream (encoded as system table)
    let tables_encoded = crate::encode_stream_name("_Tables", true);
    let columns_encoded = crate::encode_stream_name("_Columns", true);

    // Read _Tables stream
    let mut user_tables = Vec::new();
    if let Some(tables_entry) = stream_map.get(&tables_encoded) {
        let tables_data =
            read_stream_by_entry(data, &header, &fat, tables_entry, root_entry);
        // _Tables has 1 column: Name (string ref, 2 bytes for short refs)
        // Data is column-major: all values for column 1 (N string IDs)
        let mut idx = 0;
        while idx + 2 <= tables_data.len() {
            let string_id = read_u16(&tables_data, idx) as u32;
            idx += 2;
            if let Some(name) = string_map.get(&string_id) {
                user_tables.push(name.clone());
            }
        }
    }

    // Read _Columns stream
    let mut columns: Vec<(String, i16, String, i32)> = Vec::new();
    if let Some(columns_entry) = stream_map.get(&columns_encoded) {
        let columns_data =
            read_stream_by_entry(data, &header, &fat, columns_entry, root_entry);
        // _Columns has 4 columns: Table(string), Number(int16), Name(string), Type(int16)
        // Column-major: [all Table values] [all Number values] [all Name values] [all Type values]
        // We need to figure out the row count first
        if columns_data.len() >= 2 {
            // Calculate row count: total bytes / bytes per row
            // Each row: 2 (Table) + 2 (Number) + 2 (Name) + 2 (Type) = 8 bytes (short refs)
            let bytes_per_row = 2 + 2 + 2 + 2; // short string refs, int16 Type
            let row_count = columns_data.len() / bytes_per_row;

            if row_count > 0 {
                let table_col_size = row_count * 2; // string refs are 2 bytes
                let number_col_size = row_count * 2; // int16
                let name_col_size = row_count * 2; // string refs
                let _type_col_size = row_count * 2; // int16

                let table_off = 0;
                let number_off = table_off + table_col_size;
                let name_off = number_off + number_col_size;
                let type_off = name_off + name_col_size;

                for i in 0..row_count {
                    let table_id = read_u16(
                        &columns_data,
                        table_off + i * 2,
                    ) as u32;
                    let col_num = read_u16(
                        &columns_data,
                        number_off + i * 2,
                    ) as i16;
                    let name_id = read_u16(
                        &columns_data,
                        name_off + i * 2,
                    ) as u32;
                    let bitfield = read_u16(
                        &columns_data,
                        type_off + i * 2,
                    ) as i32;

                    let table_name = string_map
                        .get(&table_id)
                        .cloned()
                        .unwrap_or_default();
                    let col_name = string_map
                        .get(&name_id)
                        .cloned()
                        .unwrap_or_default();

                    columns.push((table_name, col_num, col_name, bitfield));
                }
            }
        }
    }

    // Check that each user table has a corresponding stream.
    // Per MSI spec, ALL table streams (including user tables) use the TABLE_PREFIX.
    let table_streams_found: Vec<String> = user_tables
        .iter()
        .filter(|t| {
            let encoded = crate::encode_stream_name(t, true);
            stream_map.contains_key(&encoded)
        })
        .cloned()
        .collect();

    let all_tables_have_streams = table_streams_found.len() == user_tables.len();

    // Check column consistency: every column should reference a known table
    let columns_consistent = columns.iter().all(|(table, _, _, _)| {
        user_tables.contains(table) || table.starts_with('_')
    });

    Ok(MsiSemanticReport {
        user_tables,
        table_streams_found,
        columns,
        all_tables_have_streams,
        columns_consistent,
        string_pool_size,
    })
}

/// Attempt to decode an MSI encoded stream name back to a table name.
/// Returns None if the name doesn't look like an encoded table stream.
fn decode_stream_name(encoded: &str) -> Option<String> {
    let chars: Vec<char> = encoded.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let mut idx = 0;
    let mut has_prefix = false;

    // Check for TABLE_PREFIX
    if chars[0] == '\u{4840}' {
        has_prefix = true;
        idx = 1;
    }

    if !has_prefix {
        return None;
    }

    // Decode pairs of characters
    let mut decoded = String::new();
    while idx < chars.len() {
        let ch = chars[idx] as u32;
        if (0x4800..=0x483F).contains(&ch) {
            // Single-character encoding: 0x4800 + v (check FIRST, overlaps two-char range)
            let v = ch - 0x4800;
            if let Some(c) = from_b64(v) {
                decoded.push(c);
            }
            idx += 1;
        } else if (0x3800..0x4800).contains(&ch) || (ch > 0x483F && ch <= 0x4BFF) {
            // Two-character encoding: 0x3800 + (v2 << 6) + v1
            // Range: 0x3800-0x4BFF, excluding single-char band 0x4800-0x483F
            let combined = ch - 0x3800;
            let v1 = combined & 0x3F; // first char (lower 6 bits)
            let v2 = (combined >> 6) & 0x3F; // second char (upper 6 bits)
            if let Some(c1) = from_b64(v1) {
                decoded.push(c1);
            }
            if let Some(c2) = from_b64(v2) {
                decoded.push(c2);
            }
            idx += 1; // one encoded char → two decoded chars
        } else {
            // Literal character
            decoded.push(chars[idx]);
            idx += 1;
        }
    }

    if decoded.is_empty() {
        None
    } else {
        Some(decoded)
    }
}

fn from_b64(value: u32) -> Option<char> {
    match value {
        0..=9 => Some(char::from_u32('0' as u32 + value).unwrap()),
        10..=35 => Some(char::from_u32('A' as u32 + value - 10).unwrap()),
        36..=61 => Some(char::from_u32('a' as u32 + value - 36).unwrap()),
        62 => Some('.'),
        63 => Some('_'),
        _ => None,
    }
}

fn sector_offset(sector: usize, sector_size: usize, header_size: usize) -> usize {
    header_size + sector * sector_size
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn read_u64(data: &[u8], offset: usize) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[offset..offset + 8]);
    u64::from_le_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ole;

    #[test]
    fn test_validate_empty_data() {
        let info = validate_ole(&[]).unwrap();
        assert!(!info.valid_ole);
    }

    #[test]
    fn test_validate_bad_magic() {
        let data = vec![0u8; 1024];
        let info = validate_ole(&data).unwrap();
        assert!(!info.valid_ole);
    }

    #[test]
    fn test_validate_generated_msi() {
        // Build a minimal MSI and validate it
        let streams = vec![
            ole::OleStream {
                name: "\u{0005}SummaryInformation".to_string(),
                data: vec![1, 2, 3, 4],
            },
            ole::OleStream {
                name: "TestStream".to_string(),
                data: vec![5, 6, 7, 8],
            },
        ];
        let data = ole::build_ole_file(&streams);
        let info = validate_ole(&data).unwrap();
        assert!(info.valid_ole);
        assert!(info.has_summary);
        assert!(info.stream_names.contains(&"\u{0005}SummaryInformation".to_string()));
    }

    #[test]
    fn test_decode_stream_name_roundtrip() {
        use crate::encode_stream_name;
        let original = "_Columns";
        let encoded = encode_stream_name(original, true);
        let decoded = decode_stream_name(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_decode_stream_name_tables() {
        use crate::encode_stream_name;
        let original = "_Tables";
        let encoded = encode_stream_name(original, true);
        let decoded = decode_stream_name(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_is_string_pool_stream() {
        let pool = "\u{4840}\u{3F3F}\u{4577}\u{446C}\u{3E6A}\u{44B2}\u{482F}";
        assert!(is_string_pool_stream(pool));
        let data = "\u{4840}\u{3F3F}\u{4577}\u{446C}\u{3B6A}\u{45E4}\u{4824}";
        assert!(is_string_pool_stream(data));
    }

    #[test]
    fn test_semantic_validation_full_msi() {
        use crate::{Column, MsiBuilder, Value};

        let mut builder = MsiBuilder::new();
        builder.set_title("Semantic Test");
        builder.set_author("Velocity");

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
                    vec![Value::from("ProductName"), Value::from("Test")],
                    vec![Value::from("ProductVersion"), Value::from("1.0")],
                ],
            )
            .unwrap();

        let msi_data = builder.build().unwrap();
        let report = validate_msi_semantics(&msi_data).unwrap();

        // Should find the Property table
        assert!(
            report.user_tables.contains(&"Property".to_string()),
            "Should find Property table in _Tables stream, found: {:?}",
            report.user_tables
        );

        // String pool should have entries
        assert!(
            report.string_pool_size > 0,
            "String pool should have entries"
        );

        // All tables should have corresponding streams
        assert!(
            report.all_tables_have_streams,
            "All user tables should have encoded streams"
        );

        // Columns should be consistent
        assert!(
            report.columns_consistent,
            "Columns should reference known tables"
        );

        // Should have column entries for Property table
        let prop_cols: Vec<_> = report
            .columns
            .iter()
            .filter(|(t, _, _, _)| t == "Property")
            .collect();
        assert_eq!(
            prop_cols.len(),
            2,
            "Property table should have 2 columns, got {}",
            prop_cols.len()
        );
    }

    #[test]
    fn test_semantic_validation_invalid_data() {
        let result = validate_msi_semantics(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_semantic_validation_multiple_tables() {
        use crate::{Column, MsiBuilder, Value};

        let mut builder = MsiBuilder::new();
        builder.set_title("Multi-Table Test");
        builder.set_author("Velocity");

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
            .create_table(
                "Feature",
                vec![
                    Column::build("Feature").string(38).primary_key().build(),
                    Column::build("Title").string(64).nullable().build(),
                    Column::build("Level").int16().nullable().build(),
                ],
            )
            .unwrap();

        builder
            .insert_rows(
                "Property",
                vec![vec![Value::from("ProductName"), Value::from("Multi")]],
            )
            .unwrap();

        builder
            .insert_rows(
                "Feature",
                vec![vec![
                    Value::from("MainFeature"),
                    Value::from("Complete"),
                    Value::Int(1),
                ]],
            )
            .unwrap();

        let msi_data = builder.build().unwrap();
        let report = validate_msi_semantics(&msi_data).unwrap();

        // Should find both tables
        assert!(report.user_tables.contains(&"Property".to_string()));
        assert!(report.user_tables.contains(&"Feature".to_string()));

        // Check column counts
        let prop_cols: Vec<_> = report
            .columns
            .iter()
            .filter(|(t, _, _, _)| t == "Property")
            .collect();
        assert_eq!(prop_cols.len(), 2, "Property should have 2 columns");

        let feat_cols: Vec<_> = report
            .columns
            .iter()
            .filter(|(t, _, _, _)| t == "Feature")
            .collect();
        assert_eq!(feat_cols.len(), 3, "Feature should have 3 columns");

        // All tables should have streams
        assert!(report.all_tables_have_streams);
        assert!(report.columns_consistent);
    }
}
