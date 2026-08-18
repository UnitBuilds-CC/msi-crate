/// Decode and dump ALL streams from any MSI file for comparison.
/// Usage: cargo run -p velocity-msi --example decode_any <path-to-msi>
use cfb::CompoundFile;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = if args.len() > 1 { &args[1] } else { "target/test_velocity.msi" };

    let file = File::open(path).unwrap();
    let mut comp = CompoundFile::open(file).unwrap();

    let entries: Vec<(PathBuf, bool)> = comp
        .walk()
        .map(|e| (e.path().to_path_buf(), e.is_stream()))
        .collect();

    for (p, is_stream) in &entries {
        if !is_stream {
            continue;
        }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let mut stream = comp.open_stream(p).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();

        let cps: Vec<String> = name.chars().map(|c| format!("U+{:04X}", c as u32)).collect();
        let name_str = cps.join(" ");

        println!("=== Stream: '{}' ===", name_str);
        println!("    Codepoints: {}", name_str);
        println!("    Size: {} bytes", data.len());

        // Identify stream type and decode
        if is_string_pool(&name) {
            decode_string_pool(&data);
        } else if is_string_data(&name) {
            decode_string_data(&data);
        } else if is_tables(&name) {
            decode_tables(&data);
        } else if is_columns(&name) {
            decode_columns_generic(&data);
        } else if is_validation(&name) {
            decode_validation_generic(&data);
        } else if name.contains("Summary") {
            decode_summary(&data);
        } else {
            // Unknown stream - show first 64 bytes hex
            dump_hex(&data, 128);
        }
        println!();
    }
}

fn is_string_pool(name: &str) -> bool {
    // _StringPool with TABLE_PREFIX: U+4840 + encoded("_StringPool")
    // Distinguished by U+3E6A at position 4 (encodes "gP" → 'g' + 'P')
    name.contains("U+4840 U+3F3F U+4577 U+446C U+3E6A")
}

fn is_string_data(name: &str) -> bool {
    // _StringData with TABLE_PREFIX: U+4840 + encoded("_StringData")
    // Distinguished by U+3B6A at position 4 (encodes "gD" → 'g' + 'D')
    name.contains("U+4840 U+3F3F U+4577 U+446C U+3B6A")
}

fn is_tables(name: &str) -> bool {
    // _Tables with TABLE_PREFIX: U+4840 U+3F7F ...
    name.contains("U+4840 U+3F7F")
}

fn is_columns(name: &str) -> bool {
    // _Columns with TABLE_PREFIX: U+4840 U+3B3F ...
    name.contains("U+4840 U+3B3F")
}

fn is_validation(name: &str) -> bool {
    // _Validation with TABLE_PREFIX: U+4840 U+3FFF ...
    name.contains("U+4840 U+3FFF")
}

fn dump_hex(data: &[u8], max_bytes: usize) {
    let limit = data.len().min(max_bytes);
    for (i, chunk) in data[..limit].chunks(16).enumerate() {
        print!("    {:04x}: ", i * 16);
        for b in chunk {
            print!("{:02x} ", b);
        }
        print!(" |");
        for b in chunk {
            if *b >= 0x20 && *b < 0x7f {
                print!("{}", *b as char);
            } else {
                print!(".");
            }
        }
        println!("|");
    }
    if data.len() > max_bytes {
        println!("    ... ({} more bytes)", data.len() - max_bytes);
    }
}

fn decode_string_pool(data: &[u8]) {
    if data.len() < 4 {
        println!("    Too short!");
        return;
    }
    let codepage = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let long_refs = (codepage & 0x80000000) != 0;
    let actual_codepage = codepage & 0x7FFFFFFF;
    println!("    Codepage: {} (long_refs={})", actual_codepage, long_refs);

    let mut offset = 4;
    let mut string_id = 1u32;
    while offset + 4 <= data.len() {
        let len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        let refcount = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);
        println!("    String {}: len={} refcount={}", string_id, len, refcount);
        offset += 4;
        string_id += 1;
    }
    println!("    Total strings: {}, header size: {} bytes", string_id - 1, offset - 4);
}

fn decode_string_data(data: &[u8]) {
    println!("    Raw bytes ({}):", data.len());
    // Show as ASCII where printable, with offset markers
    for (i, chunk) in data.chunks(32).enumerate() {
        print!("    {:04x}: ", i * 32);
        for b in chunk {
            print!("{:02x} ", b);
        }
        print!(" |");
        for b in chunk {
            if *b >= 0x20 && *b < 0x7f {
                print!("{}", *b as char);
            } else {
                print!(".");
            }
        }
        println!("|");
    }
}

fn decode_tables(data: &[u8]) {
    // _Tables: 1 column (Name, string ref 2 bytes)
    let num_rows = data.len() / 2;
    println!("    Rows: {}", num_rows);
    for i in 0..num_rows {
        let sref = u16::from_le_bytes([data[i * 2], data[i * 2 + 1]]);
        println!("    Row {}: name_ref={}", i, sref);
    }
}

fn decode_columns_generic(data: &[u8]) {
    // _Columns: 4 columns: Table(str,2), Number(i16,2), Name(str,2), Type(i32,4)
    // Column-major order
    let col_widths: &[usize] = &[2, 2, 2, 4];
    let row_width: usize = col_widths.iter().sum(); // 10
    let num_rows = data.len() / row_width;
    println!("    Rows: {} (row_width={}, data={})", num_rows, row_width, data.len());

    let col_names = ["Table", "Number", "Name", "Type"];
    let mut offset = 0;
    for (col_idx, (name, &width)) in col_names.iter().zip(col_widths.iter()).enumerate() {
        println!("    Column '{}' ({} bytes):", name, width);
        for row in 0..num_rows {
            let start = offset + row * width;
            if start + width > data.len() {
                println!("      Row {}: OUT OF BOUNDS", row);
                continue;
            }
            if width == 2 {
                let val = u16::from_le_bytes([data[start], data[start + 1]]);
                println!("      Row {}: {}", row, val);
            } else {
                let val = u32::from_le_bytes([
                    data[start],
                    data[start + 1],
                    data[start + 2],
                    data[start + 3],
                ]);
                let size = val & 0xFF;
                let nullable = (val >> 8) & 1;
                let pk = (val >> 9) & 1;
                let type_code = (val >> 10) & 0x3F;
                let type_name = match type_code {
                    0 => "string",
                    1 => "int32",
                    2 => "int16",
                    4 => "binary",
                    _ => "unknown",
                };
                println!(
                    "      Row {}: 0x{:08X} (size={}, nullable={}, pk={}, type={}/{})",
                    row, val, size, nullable, pk, type_code, type_name
                );
            }
        }
        offset += num_rows * width;
    }
}

fn decode_validation_generic(data: &[u8]) {
    // _Validation: 10 columns
    let col_widths: &[usize] = &[2, 2, 2, 4, 4, 2, 2, 2, 2, 2];
    let row_width: usize = col_widths.iter().sum(); // 22
    let num_rows = data.len() / row_width;
    println!(
        "    Rows: {} (row_width={}, data={})",
        num_rows, row_width, data.len()
    );

    let col_names = [
        "Table", "Column", "Nullable", "MinValue", "MaxValue", "KeyTable", "KeyColumn",
        "Category", "Set", "Description",
    ];
    let mut offset = 0;
    for (col_idx, (name, &width)) in col_names.iter().zip(col_widths.iter()).enumerate() {
        println!("    Column '{}' ({} bytes):", name, width);
        for row in 0..num_rows {
            let start = offset + row * width;
            if start + width > data.len() {
                println!("      Row {}: OUT OF BOUNDS", row);
                continue;
            }
            if width == 2 {
                let val = u16::from_le_bytes([data[start], data[start + 1]]);
                println!("      Row {}: {}", row, val);
            } else {
                let val = i32::from_le_bytes([
                    data[start],
                    data[start + 1],
                    data[start + 2],
                    data[start + 3],
                ]);
                println!("      Row {}: {}", row, val);
            }
        }
        offset += num_rows * width;
    }
}

fn decode_summary(data: &[u8]) {
    println!("    SummaryInformation ({} bytes):", data.len());
    if data.len() < 48 {
        println!("    Too short for property set header!");
        return;
    }
    // Header
    let bom = u16::from_le_bytes([data[0], data[1]]);
    let ver = u16::from_le_bytes([data[2], data[3]]);
    let os_low = u16::from_le_bytes([data[4], data[5]]);
    let os_high = u16::from_le_bytes([data[6], data[7]]);
    println!("    BOM: 0x{:04X}, Version: {}, OS: {}.{}", bom, ver, os_low, os_high);

    // CLSID (16 bytes at offset 8)
    let clsid = &data[8..24];
    let all_zero = clsid.iter().all(|&b| b == 0);
    println!("    CLSID: {}", if all_zero { "all zeros" } else { "non-zero" });

    // Reserved
    let reserved = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
    println!("    Reserved: {}", reserved);

    // FMTID
    let fmtid = &data[28..44];
    print!("    FMTID: ");
    for b in fmtid {
        print!("{:02x}", b);
    }
    println!();

    // Section offset
    let sect_off = u32::from_le_bytes([data[44], data[45], data[46], data[47]]);
    println!("    Section offset: {}", sect_off);

    if sect_off as usize + 8 > data.len() {
        println!("    Section offset out of bounds!");
        return;
    }

    // Section
    let s = sect_off as usize;
    let sect_size = u32::from_le_bytes([data[s], data[s + 1], data[s + 2], data[s + 3]]);
    let prop_count = u32::from_le_bytes([data[s + 4], data[s + 5], data[s + 6], data[s + 7]]);
    println!("    Section size: {}, Properties: {}", sect_size, prop_count);

    // Index entries
    for i in 0..prop_count as usize {
        let idx_off = s + 8 + i * 8;
        if idx_off + 8 > data.len() {
            break;
        }
        let prop_id = u32::from_le_bytes([
            data[idx_off],
            data[idx_off + 1],
            data[idx_off + 2],
            data[idx_off + 3],
        ]);
        let prop_off = u32::from_le_bytes([
            data[idx_off + 4],
            data[idx_off + 5],
            data[idx_off + 6],
            data[idx_off + 7],
        ]);
        // Read the property type
        let val_off = s + prop_off as usize;
        if val_off + 4 <= data.len() {
            let vtype = u32::from_le_bytes([
                data[val_off],
                data[val_off + 1],
                data[val_off + 2],
                data[val_off + 3],
            ]);
            println!(
                "    Prop {}: id={} offset={} type={}",
                i, prop_id, prop_off, vtype
            );
        } else {
            println!("    Prop {}: id={} offset={} (out of bounds)", i, prop_id, prop_off);
        }
    }
}
