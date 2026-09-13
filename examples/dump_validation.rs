/// Diagnostic: decode stream names, string pool, and _Validation table from an MSI.
use std::io::{Cursor, Read};

fn to_b64(ch: char) -> Option<u32> {
    if ch.is_ascii_digit() { Some(ch as u32 - '0' as u32) }
    else if ch.is_ascii_uppercase() { Some(10 + ch as u32 - 'A' as u32) }
    else if ch.is_ascii_lowercase() { Some(36 + ch as u32 - 'a' as u32) }
    else if ch == '.' { Some(62) }
    else if ch == '_' { Some(63) }
    else { None }
}

fn from_b64(val: u8) -> Option<char> {
    match val {
        0..=9 => Some((b'0' + val) as char),
        10..=35 => Some((b'A' + val - 10) as char),
        36..=61 => Some((b'a' + val - 36) as char),
        62 => Some('.'),
        63 => Some('_'),
        _ => None,
    }
}

fn decode_stream_name(encoded: &str) -> Option<(bool, String)> {
    let chars: Vec<char> = encoded.chars().collect();
    if chars.is_empty() { return None; }

    let mut idx = 0;
    let mut has_prefix = false;
    if chars[0] == '\u{4840}' {
        has_prefix = true;
        idx = 1;
    }
    if !has_prefix { return None; }

    let mut decoded = String::new();
    while idx < chars.len() {
        let c = chars[idx] as u32;
        if c >= 0x3800 && c < 0x4800 {
            let val = (c - 0x3800) as u16; // MUST be u16, not u8!
            let v1 = (val & 0x3F) as u8;
            let v2 = (val >> 6) as u8;
            decoded.push(from_b64(v1)?);
            decoded.push(from_b64(v2)?);
            idx += 1;
        } else if c >= 0x4800 && c < 0x4840 {
            decoded.push(from_b64((c - 0x4800) as u8)?);
            idx += 1;
        } else {
            decoded.push(chars[idx]);
            idx += 1;
        }
    }
    Some((has_prefix, decoded))
}

fn main() {
    let data = std::fs::read("velocity_install_test.msi").unwrap();
    let cursor = Cursor::new(&data);
    let mut cfb = cfb::CompoundFile::open(cursor).unwrap();

    // Collect stream names first (to avoid borrow issues)
    let stream_entries: Vec<(String, bool)> = cfb.walk()
        .filter(|e| e.is_stream())
        .map(|e| (e.name().to_string(), e.is_stream()))
        .collect();

    // Decode stream names and read data
    let mut streams: Vec<(String, String, Vec<u8>)> = Vec::new(); // (raw_name, decoded_name, data)
    for (raw_name, _) in &stream_entries {
        let mut stream_data = Vec::new();
        let mut stream = cfb.open_stream(raw_name).unwrap();
        stream.read_to_end(&mut stream_data).unwrap();

        if raw_name.starts_with('\u{0005}') {
            println!("  \\x05{} ({} bytes)", &raw_name[1..], stream_data.len());
            continue;
        }

        if let Some((is_table, decoded)) = decode_stream_name(raw_name) {
            let marker = if is_table { "[T]" } else { "[ ]" };
            println!("  {} {} ({} bytes)", marker, decoded, stream_data.len());
            streams.push((raw_name.clone(), decoded, stream_data));
        } else {
            println!("  [?] {:?} ({} bytes)", raw_name, stream_data.len());
        }
    }

    // Find and decode string pool
    let mut string_pool: Vec<String> = Vec::new();
    if let Some((_, _, pool_data)) = find_stream(&streams, "_StringPool") {
        if let Some((_, _, data_data)) = find_stream(&streams, "_StringData") {
            string_pool = decode_string_pool(pool_data, data_data);
            println!("\nString pool: {} entries", string_pool.len());
            for (i, s) in string_pool.iter().enumerate() {
                if i < 80 || s.contains("Directory") || s.contains("Component") || s.contains("Feature") {
                    println!("  [{}] = {:?}", i, s);
                }
            }
        }
    }

    // Decode _Columns to get schema for each table
    if let Some((_, _, columns_data)) = find_stream(&streams, "_Columns") {
        println!("\n=== _Columns ({} bytes) ===", columns_data.len());
        // _Columns schema: Table(string ref), Number(int16), Name(string ref), Type(int16)
        // Column-major: all Table values, all Number values, all Name values, all Type values
        // We need the string pool to decode table/name refs
        // Each column: Table=2 bytes, Number=2 bytes, Name=2 bytes, Type=2 bytes
        // But it's column-major, so we need to know the row count first
        // Total bytes = 4 columns * num_entries * 2 bytes each
        // So num_entries = data.len() / 8
        let num_entries = columns_data.len() / 8;
        println!("  Entries: {}", num_entries);

        // Read column-major: col0=all Tables, col1=all Numbers, col2=all Names, col3=all Types
        let col_size = num_entries * 2;
        for i in 0..num_entries {
            let table_id = u16::from_le_bytes([columns_data[i*2], columns_data[i*2+1]]) as usize;
            let number = {
                let off = col_size + i * 2;
                let raw = u16::from_le_bytes([columns_data[off], columns_data[off+1]]);
                (raw as i16 ^ -0x8000i16) as i32
            };
            let name_id = {
                let off = col_size * 2 + i * 2;
                u16::from_le_bytes([columns_data[off], columns_data[off+1]]) as usize
            };
            let type_bits = {
                let off = col_size * 3 + i * 2;
                let raw = u16::from_le_bytes([columns_data[off], columns_data[off+1]]);
                (raw as i16 ^ -0x8000i16) as u16
            };

            let table_name = if table_id < string_pool.len() { &string_pool[table_id] } else { "?" };
            let col_name = if name_id < string_pool.len() { &string_pool[name_id] } else { "?" };

            // Only show Directory-related columns
            if table_name.contains("Directory") || table_name == "_Validation" {
                println!("  {}[{}] = {} (type=0x{:04X})", table_name, number, col_name, type_bits);
            }
        }
    }

    // Decode _Tables system table
    if let Some((_, _, tables_data)) = find_stream(&streams, "_Tables") {
        println!("\n=== _Tables ({} bytes) ===", tables_data.len());
        // _Tables has 1 column: Name (string ref, 2 bytes)
        // Column-major: just one column
        let num_rows = tables_data.len() / 2;
        println!("  Tables listed: {}", num_rows);
        for i in 0..num_rows {
            let name_id = u16::from_le_bytes([tables_data[i*2], tables_data[i*2+1]]) as usize;
            let name = if name_id < string_pool.len() { &string_pool[name_id] } else { "?" };
            println!("  {}", name);
        }
    }

    // Decode Directory table
    if let Some((_, _, dir_data)) = find_stream(&streams, "Directory") {
        println!("\n=== Directory ({} bytes) ===", dir_data.len());
        println!("  Hex: {:02X?}", dir_data);
        // Directory schema: Directory(string), Directory_Parent(string nullable), DefaultDir(string)
        // All string refs are 2 bytes (short refs)
        // Column-major: 3 columns × num_rows × 2 bytes
        let num_rows = dir_data.len() / 6; // 3 cols × 2 bytes = 6 bytes per row
        println!("  Rows: {}", num_rows);
        let col_size = num_rows * 2;
        for i in 0..num_rows {
            let dir_id = u16::from_le_bytes([dir_data[i*2], dir_data[i*2+1]]) as usize;
            let parent_id = {
                let off = col_size + i * 2;
                u16::from_le_bytes([dir_data[off], dir_data[off+1]]) as usize
            };
            let default_id = {
                let off = col_size * 2 + i * 2;
                u16::from_le_bytes([dir_data[off], dir_data[off+1]]) as usize
            };
            println!("  Row {}: dir_id={} parent_id={} default_id={}", i, dir_id, parent_id, default_id);
            let dir_name = if dir_id < string_pool.len() { &string_pool[dir_id] } else { "?" };
            let parent_name = if parent_id == 0 { "NULL".to_string() }
                             else if parent_id < string_pool.len() { string_pool[parent_id].clone() }
                             else { "?".to_string() };
            let default_name = if default_id < string_pool.len() { &string_pool[default_id] } else { "?" };
            println!("    -> dir={:?} parent={:?} default={:?}", dir_name, parent_name, default_name);
        }
    }

    // Decode _Validation table
    if let Some((_, _, val_data)) = find_stream(&streams, "_Validation") {
        println!("\n=== _Validation ({} bytes) ===", val_data.len());

        // Column offsets for 51 rows:
        // col0 (Table):       0..102     (51 × 2)
        // col1 (Column):      102..204   (51 × 2)
        // col2 (Nullable):    204..306   (51 × 2)
        // col3 (MinValue):    306..510   (51 × 4)
        // col4 (MaxValue):    510..714   (51 × 4)
        // col5 (KeyTable):    714..816   (51 × 2)
        // col6 (KeyColumn):   816..918   (51 × 2)
        // col7 (Category):    918..1020  (51 × 2)
        // col8 (Set):         1020..1122 (51 × 2)
        // col9 (Description): 1122..1224 (51 × 2)
        let num_rows = val_data.len() / 24;
        let col5_off = num_rows * (2+2+2+4+4); // offset to KeyTable column
        let col6_off = col5_off + num_rows * 2; // offset to KeyColumn column

        println!("  KeyTable column (offset {}):", col5_off);
        for i in 0..num_rows.min(15) {
            let off = col5_off + i * 2;
            let id = u16::from_le_bytes([val_data[off], val_data[off+1]]) as usize;
            let name = if id == 0 { "NULL".to_string() }
                      else if id < string_pool.len() { format!("{:?}", string_pool[id]) }
                      else { format!("ID_{}", id) };
            println!("    Row {}: {}", i, name);
        }

        println!("  KeyColumn column (offset {}):", col6_off);
        for i in 0..num_rows.min(15) {
            let off = col6_off + i * 2;
            let raw = u16::from_le_bytes([val_data[off], val_data[off+1]]);
            let val = if raw == 0 { "NULL".to_string() }
                     else { format!("{}", (raw as i16 ^ -0x8000i16) as i32) };
            println!("    Row {}: {}", i, val);
        }

        // _Validation has 10 columns, all 2 bytes except MinValue/MaxValue (4 bytes each)
        // Column widths: Table(2) + Column(2) + Nullable(2) + MinValue(4) + MaxValue(4) +
        //                KeyTable(2) + KeyColumn(2) + Category(2) + Set(2) + Description(2) = 24
        // Column-major: each column's values are contiguous
        // Column offsets within the data:
        //   col0 (Table):       0..num_rows*2
        //   col1 (Column):      num_rows*2..num_rows*4
        //   col2 (Nullable):    num_rows*4..num_rows*6
        //   col3 (MinValue):    num_rows*6..num_rows*10  (4 bytes each!)
        //   col4 (MaxValue):    num_rows*10..num_rows*14 (4 bytes each!)
        //   col5 (KeyTable):    num_rows*14..num_rows*16
        //   col6 (KeyColumn):   num_rows*16..num_rows*18
        //   col7 (Category):    num_rows*18..num_rows*20
        //   col8 (Set):         num_rows*20..num_rows*22
        //   col9 (Description): num_rows*22..num_rows*24

        // Total size = num_rows * (2+2+2+4+4+2+2+2+2+2) = num_rows * 24
        let row_size = 24;
        let num_rows = val_data.len() / row_size;
        println!("  Rows: {} (expected: {})", num_rows, val_data.len() / row_size);

        let col_offset = |col_bytes: usize, col_idx: usize| -> usize {
            // Calculate the byte offset for a given column
            // Column widths in order: 2,2,2,4,4,2,2,2,2,2
            let widths = [2,2,2,4,4,2,2,2,2,2];
            let mut offset = 0;
            for i in 0..col_idx {
                offset += widths[i] * num_rows;
            }
            offset
        };

        for i in 0..num_rows {
            let table_id = {
                let off = col_offset(2, 0) + i * 2;
                u16::from_le_bytes([val_data[off], val_data[off+1]]) as usize
            };
            let column_id = {
                let off = col_offset(2, 1) + i * 2;
                u16::from_le_bytes([val_data[off], val_data[off+1]]) as usize
            };
            let nullable_id = {
                let off = col_offset(2, 2) + i * 2;
                u16::from_le_bytes([val_data[off], val_data[off+1]]) as usize
            };
            let min_value = {
                let off = col_offset(4, 3) + i * 4;
                i32::from_le_bytes([val_data[off], val_data[off+1], val_data[off+2], val_data[off+3]])
            };
            let max_value = {
                let off = col_offset(4, 4) + i * 4;
                i32::from_le_bytes([val_data[off], val_data[off+1], val_data[off+2], val_data[off+3]])
            };
            let key_table_id = {
                let off = col_offset(2, 5) + i * 2;
                u16::from_le_bytes([val_data[off], val_data[off+1]]) as usize
            };
            let key_column_raw = {
                let off = col_offset(2, 6) + i * 2;
                u16::from_le_bytes([val_data[off], val_data[off+1]])
            };
            let key_column = (key_column_raw as i16 ^ -0x8000i16) as i32;
            let category_id = {
                let off = col_offset(2, 7) + i * 2;
                u16::from_le_bytes([val_data[off], val_data[off+1]]) as usize
            };
            let set_id = {
                let off = col_offset(2, 8) + i * 2;
                u16::from_le_bytes([val_data[off], val_data[off+1]]) as usize
            };
            let desc_id = {
                let off = col_offset(2, 9) + i * 2;
                u16::from_le_bytes([val_data[off], val_data[off+1]]) as usize
            };

            let lookup = |id: usize| -> String {
                if id == 0 { "NULL".to_string() }
                else if id < string_pool.len() { format!("{:?}", string_pool[id]) }
                else { format!("ID_{}", id) }
            };

            let table_name = lookup(table_id);
            let min_str = if min_value == 0 { "NULL".to_string() } else {
                let decoded = min_value ^ -0x80000000i32;
                format!("{}", decoded)
            };
            let max_str = if max_value == 0 { "NULL".to_string() } else {
                let decoded = max_value ^ -0x80000000i32;
                format!("{}", decoded)
            };
            let kt_str = lookup(key_table_id);
            let kc_str = if key_column_raw == 0 { "NULL".to_string() } else { format!("{}", key_column) };
            let cat_str = lookup(category_id);
            let set_str = lookup(set_id);
            let desc_str = lookup(desc_id);

            println!("  Row {}: table={} col={} nullable={} min={} max={} key_table={} key_col={} cat={} set={} desc={}",
                i, table_name, lookup(column_id), lookup(nullable_id),
                min_str, max_str, kt_str, kc_str, cat_str, set_str, desc_str);
        }
    }
}

fn find_stream<'a>(streams: &'a [(String, String, Vec<u8>)], name: &str) -> Option<&'a (String, String, Vec<u8>)> {
    streams.iter().find(|(_, decoded, _)| decoded == name)
}

fn decode_string_pool(pool_data: &[u8], data_data: &[u8]) -> Vec<String> {
    if pool_data.len() < 4 { return vec![]; }

    let header = u32::from_le_bytes([pool_data[0], pool_data[1], pool_data[2], pool_data[3]]);
    let codepage = header & 0xFFFF;
    let _long_refs = (header >> 31) != 0;
    println!("String pool codepage: {}", codepage);

    let mut strings = vec![String::new()]; // ID 0 = empty
    let mut pos = 4;
    let mut data_pos = 0;

    while pos + 4 <= pool_data.len() {
        let len = u16::from_le_bytes([pool_data[pos], pool_data[pos+1]]) as usize;
        let _refcount = u16::from_le_bytes([pool_data[pos+2], pool_data[pos+3]]);
        pos += 4;

        if data_pos + len <= data_data.len() {
            let bytes = &data_data[data_pos..data_pos + len];
            let s = String::from_utf8_lossy(bytes).to_string();
            strings.push(s);
            data_pos += len;
        } else {
            strings.push(String::new());
        }
    }

    strings
}
