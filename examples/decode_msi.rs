use cfb::CompoundFile;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

fn main() {
    let file = File::open("target/test_velocity.msi").unwrap();
    let mut comp = CompoundFile::open(file).unwrap();
    
    let entries: Vec<(PathBuf, bool)> = comp.walk()
        .map(|e| (e.path().to_path_buf(), e.is_stream()))
        .collect();
    
    for (p, is_stream) in &entries {
        if !is_stream { continue; }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let mut stream = comp.open_stream(p).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        
        let cps: Vec<String> = name.chars().map(|c| format!("U+{:04X}", c as u32)).collect();
        let name_str = cps.join(" ");
        
        // Identify streams by their codepoints
        if name_str.contains("U+3F3F U+4577 U+446C U+3E6A") {
            println!("=== _StringPool ({} bytes) ===", data.len());
            decode_string_pool(&data);
        } else if name_str.contains("U+3F3F U+4577 U+446C U+3B6A") {
            println!("=== _StringData ({} bytes) ===", data.len());
            decode_string_data(&data);
        } else if name_str.contains("U+4840 U+3F7F") {
            println!("=== _Tables ({} bytes) ===", data.len());
            decode_tables(&data);
        } else if name_str.contains("U+4840 U+3B3F") {
            println!("=== _Columns ({} bytes) ===", data.len());
            decode_columns(&data);
        } else if name_str.contains("U+4840 U+3FFF") {
            println!("=== _Validation ({} bytes) ===", data.len());
            decode_validation(&data);
        } else if name.contains("Summary") {
            println!("=== SummaryInformation ({} bytes) ===", data.len());
            // Just show first 64 bytes
            for (i, chunk) in data.chunks(16).enumerate().take(4) {
                print!("  {:04x}: ", i * 16);
                for b in chunk { print!("{:02x} ", b); }
                println!();
            }
        } else {
            println!("=== Property ({}) ({} bytes) ===", name_str, data.len());
            // Show raw hex
            for (i, chunk) in data.chunks(16).enumerate().take(4) {
                print!("  {:04x}: ", i * 16);
                for b in chunk { print!("{:02x} ", b); }
                println!();
            }
        }
    }
}

fn decode_string_pool(data: &[u8]) {
    if data.len() < 4 { println!("  Too short!"); return; }
    let codepage = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let long_refs = (codepage & 0x80000000) != 0;
    let actual_codepage = codepage & 0x7FFFFFFF;
    println!("  Codepage: {} (long_refs={})", actual_codepage, long_refs);
    
    let mut offset = 4;
    let mut string_id = 1u32;
    let mut total_data_len = 0u32;
    while offset + 4 <= data.len() {
        let len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        let refcount = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);
        println!("  String {}: len={} refcount={}", string_id, len, refcount);
        total_data_len += len as u32;
        offset += 4;
        string_id += 1;
    }
    println!("  Total strings: {}, total data bytes needed: {}", string_id - 1, total_data_len);
}

fn decode_string_data(data: &[u8]) {
    println!("  Raw bytes ({}):", data.len());
    for (i, chunk) in data.chunks(32).enumerate() {
        print!("  {:04x}: ", i * 32);
        for b in chunk { print!("{:02x} ", b); }
        // Also show as ASCII where printable
        print!(" |");
        for b in chunk {
            if *b >= 0x20 && *b < 0x7f { print!("{}", *b as char); }
            else { print!("."); }
        }
        println!("|");
    }
}

fn decode_tables(data: &[u8]) {
    // _Tables has 1 column: Name (string ref, 2 bytes)
    println!("  Raw hex:");
    for (i, chunk) in data.chunks(16).enumerate() {
        print!("  {:04x}: ", i * 16);
        for b in chunk { print!("{:02x} ", b); }
        println!();
    }
    // Each row is 2 bytes (string ref)
    let num_rows = data.len() / 2;
    println!("  Rows: {}", num_rows);
    for i in 0..num_rows {
        let sref = u16::from_le_bytes([data[i * 2], data[i * 2 + 1]]);
        println!("    Row {}: string_ref={}", i, sref);
    }
}

fn decode_columns(data: &[u8]) {
    // _Columns has 4 columns: Table(str,2), Number(i16,2), Name(str,2), Type(i32,4)
    // Column-major: all Table values, then all Number values, then all Name values, then all Type values
    let num_cols = 4;
    let col_widths = [2, 2, 2, 4]; // Table(str), Number(i16), Name(str), Type(i32)
    let total_row_width: usize = col_widths.iter().sum();
    
    if data.len() % total_row_width != 0 {
        println!("  WARNING: data size {} not multiple of row width {}", data.len(), total_row_width);
    }
    let num_rows = data.len() / total_row_width;
    println!("  Rows: {}, total bytes: {}", num_rows, data.len());
    
    // Column-major: column 0 data, then column 1 data, etc.
    let mut offset = 0;
    let col_names = ["Table", "Number", "Name", "Type"];
    for (col_idx, (name, &width)) in col_names.iter().zip(col_widths.iter()).enumerate() {
        println!("  Column '{}' ({} bytes per value):", name, width);
        for row in 0..num_rows {
            let start = offset + row * width;
            if width == 2 {
                let val = u16::from_le_bytes([data[start], data[start + 1]]);
                println!("    Row {}: {}", row, val);
            } else if width == 4 {
                let val = i32::from_le_bytes([data[start], data[start + 1], data[start + 2], data[start + 3]]);
                // Decode bitfield
                let size = val & 0xFF;
                let nullable = (val >> 8) & 1;
                let pk = (val >> 9) & 1;
                let type_code = (val >> 10) & 0x3F;
                println!("    Row {}: 0x{:08X} (size={}, nullable={}, pk={}, type={})", row, val as u32, size, nullable, pk, type_code);
            }
        }
        offset += num_rows * width;
    }
}

fn decode_validation(data: &[u8]) {
    // _Validation has 10 columns
    let col_widths = [2, 2, 2, 4, 4, 2, 2, 2, 2, 2]; // Table, Column, Nullable(str), Min(i32), Max(i32), KeyTable(str), KeyCol(i16), Category(str), Set(str), Desc(str)
    let total_row_width: usize = col_widths.iter().sum();
    let num_rows = if total_row_width > 0 { data.len() / total_row_width } else { 0 };
    println!("  Rows: {}, row_width: {}, total: {}", num_rows, total_row_width, data.len());
    
    let mut offset = 0;
    let col_names = ["Table", "Column", "Nullable", "MinValue", "MaxValue", "KeyTable", "KeyColumn", "Category", "Set", "Description"];
    for (col_idx, (name, &width)) in col_names.iter().zip(col_widths.iter()).enumerate() {
        println!("  Column '{}' ({} bytes):", name, width);
        for row in 0..num_rows {
            let start = offset + row * width;
            if width == 2 {
                let val = u16::from_le_bytes([data[start], data[start + 1]]);
                println!("    Row {}: {}", row, val);
            } else {
                let val = i32::from_le_bytes([data[start], data[start + 1], data[start + 2], data[start + 3]]);
                println!("    Row {}: {}", row, val);
            }
        }
        offset += num_rows * width;
    }
}
