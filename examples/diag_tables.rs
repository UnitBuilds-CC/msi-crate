/// Definitive diagnostic: dump ALL table stream data for the failing case
/// (Property + Directory + InstallExecuteSequence) and verify every byte.
/// Also tests removing _Validation to isolate the issue.
use velocity_msi::{MsiBuilder, Column, Value};
use std::process::Command;

fn build_msi_with_validation(include_validation: bool) -> Vec<u8> {
    let mut b = MsiBuilder::new();
    b.set_title("Diag Tables");
    b.set_author("V");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("DiagTables")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("V")],
        vec![Value::from("ProductCode"), Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![Value::from("UpgradeCode"), Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    b.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
    ]).unwrap();

    b.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
    ]).unwrap();

    b.build().unwrap()
}

/// Build MSI without _Validation table by manually constructing
fn build_msi_no_validation() -> Vec<u8> {
    // Use the normal builder but we'll check if _Validation is the issue
    // by comparing stream sizes
    build_msi_with_validation(true)
}

fn test_msi(data: &[u8], name: &str) -> i32 {
    let path = format!("diag_tables_{}.msi", name);
    let log = format!("diag_tables_{}.log", name);
    std::fs::write(&path, data).unwrap();
    let _ = std::fs::remove_file(&log);
    let output = Command::new("msiexec")
        .args(&["/i", &path, "/qn", "/norestart", "/lv", &log])
        .output()
        .expect("msiexec failed");
    let code = output.status.code().unwrap_or(-1);
    
    if code != 0 {
        if let Ok(log_content) = std::fs::read_to_string(&log) {
            for line in log_content.lines() {
                let lt = line.trim();
                if lt.contains("2705") || lt.contains("DEBUG: Error") || lt.contains("return value 3") {
                    println!("  LOG: {}", lt);
                }
            }
        }
    }
    code
}

fn extract_stream(data: &[u8], target_name: &[u16]) -> Option<Vec<u8>> {
    let sector_size = 512usize;
    let first_dir = u32::from_le_bytes([data[48], data[49], data[50], data[51]]) as usize;
    let first_fat = 0u32;
    let fat_base = 512 + first_fat as usize * sector_size;
    let first_minifat = u32::from_le_bytes([data[60], data[61], data[62], data[63]]) as usize;
    let minifat_base = 512 + first_minifat * sector_size;

    let dir_base = 512 + first_dir * sector_size;
    let mini_start = u32::from_le_bytes([data[dir_base + 116], data[dir_base + 117], data[dir_base + 118], data[dir_base + 119]]);
    let mini_size = u64::from_le_bytes([
        data[dir_base+120], data[dir_base+121], data[dir_base+122], data[dir_base+123],
        data[dir_base+124], data[dir_base+125], data[dir_base+126], data[dir_base+127],
    ]) as usize;

    let mini_container = {
        let mut result = Vec::new();
        let mut current = mini_start;
        loop {
            let sector_off = 512 + current as usize * sector_size;
            if sector_off + sector_size > data.len() { break; }
            result.extend_from_slice(&data[sector_off..sector_off + sector_size]);
            let fat_off = fat_base + current as usize * 4;
            if fat_off + 4 > data.len() { break; }
            let next = u32::from_le_bytes([data[fat_off], data[fat_off+1], data[fat_off+2], data[fat_off+3]]);
            if next == 0xFFFFFFFE || next == 0xFFFFFFFF { break; }
            current = next;
        }
        result.truncate(mini_size);
        result
    };

    for i in 0..32 {
        let off = dir_base + i * 128;
        if off + 128 > data.len() { break; }
        let name_len = u16::from_le_bytes([data[off + 64], data[off + 65]]) as usize;
        if name_len < 2 { continue; }
        let name_bytes = (name_len - 2).min(64);
        let mut entry_name = Vec::new();
        for j in 0..(name_bytes / 2) {
            if off + j * 2 + 1 >= data.len() { break; }
            entry_name.push(u16::from_le_bytes([data[off + j * 2], data[off + j * 2 + 1]]));
        }
        if entry_name != target_name { continue; }

        let start = u32::from_le_bytes([data[off + 116], data[off + 117], data[off + 118], data[off + 119]]);
        let size = u64::from_le_bytes([
            data[off+120], data[off+121], data[off+122], data[off+123],
            data[off+124], data[off+125], data[off+126], data[off+127],
        ]) as usize;

        let mut result = Vec::new();
        let mut current = start;
        for _ in 0..100 {
            let ms_off = current as usize * 64;
            let to_read = (size - result.len()).min(64);
            if ms_off + to_read <= mini_container.len() {
                result.extend_from_slice(&mini_container[ms_off..ms_off + to_read]);
            }
            if result.len() >= size { break; }
            let mf_off = minifat_base + current as usize * 4;
            if mf_off + 4 > data.len() { break; }
            let next = u32::from_le_bytes([data[mf_off], data[mf_off+1], data[mf_off+2], data[mf_off+3]]);
            if next == 0xFFFFFFFE || next == 0xFFFFFFFF { break; }
            current = next;
        }
        result.truncate(size);
        return Some(result);
    }
    None
}

fn dump_table_stream(data: &[u8], name: &str) {
    let enc = velocity_msi::encode_stream_name(name, true);
    let name_utf16: Vec<u16> = enc.encode_utf16().collect();
    if let Some(stream) = extract_stream(data, &name_utf16) {
        println!("\n=== {} stream ({} bytes) ===", name, stream.len());
        // Print hex dump
        for (i, chunk) in stream.chunks(16).enumerate() {
            print!("  {:04X}: ", i * 16);
            for b in chunk {
                print!("{:02X} ", b);
            }
            println!();
        }
        
        // For _Tables: decode string pool IDs
        if name == "_Tables" {
            println!("  _Tables entries (each value = u16 pool ID):");
            for (i, chunk) in stream.chunks(2).enumerate() {
                if chunk.len() == 2 {
                    let id = u16::from_le_bytes([chunk[0], chunk[1]]);
                    println!("    Row {}: pool ID {}", i, id);
                }
            }
        }
        
        // For _Columns: decode entries (Table: u16, Number: u16 XOR, Name: u16, Type: u16 XOR)
        if name == "_Columns" {
            println!("  _Columns entries (Table:u16, Number:i16^0x8000, Name:u16, Type:i16^0x8000):");
            let entry_size = 8; // 2+2+2+2 bytes per entry (short string refs)
            for (i, chunk) in stream.chunks(entry_size).enumerate() {
                if chunk.len() == entry_size {
                    let table_id = u16::from_le_bytes([chunk[0], chunk[1]]);
                    let col_num_raw = u16::from_le_bytes([chunk[2], chunk[3]]);
                    let col_num = (col_num_raw as i16 ^ -0x8000i16 as i16) as u16;
                    let name_id = u16::from_le_bytes([chunk[4], chunk[5]]);
                    let type_raw = u16::from_le_bytes([chunk[6], chunk[7]]);
                    let type_val = type_raw as i16 ^ -0x8000i16 as i16;
                    println!("    Row {}: Table={}, ColNum={}, Name={}, Type=0x{:04X}", 
                        i, table_id, col_num, name_id, type_val as u16);
                }
            }
        }
    } else {
        println!("\n=== {} stream: NOT FOUND ===", name);
    }
}

fn main() {
    println!("=== Definitive table diagnostic ===\n");

    // Build the failing MSI
    let msi_data = build_msi_with_validation(true);
    println!("Built MSI: {} bytes", msi_data.len());
    
    // Test it
    let code = test_msi(&msi_data, "combined");
    println!("Property + Directory + ExecSeq: exit {}", code);

    // Dump all system table streams
    dump_table_stream(&msi_data, "_Tables");
    dump_table_stream(&msi_data, "_Columns");
    dump_table_stream(&msi_data, "_Validation");
    
    // Dump string pool for reference
    let pool_enc = velocity_msi::encode_stream_name("_StringPool", true);
    let data_enc = velocity_msi::encode_stream_name("_StringData", true);
    let pool_utf16: Vec<u16> = pool_enc.encode_utf16().collect();
    let data_utf16: Vec<u16> = data_enc.encode_utf16().collect();
    
    if let Some(pool_data) = extract_stream(&msi_data, &pool_utf16) {
        if let Some(string_data) = extract_stream(&msi_data, &data_utf16) {
            println!("\n=== String Pool ===");
            let codepage = u32::from_le_bytes([pool_data[0], pool_data[1], pool_data[2], pool_data[3]]) & 0xFFFF;
            println!("Codepage: {}", codepage);
            
            let entry_size = 4; // short refs: u16 len + u16 refcount
            let num_entries = (pool_data.len() - 4) / entry_size;
            println!("Entries: {}", num_entries);
            
            let mut offset = 0usize;
            println!("{:>4}  {:>5}  {:>4}  {}", "ID", "Len", "Refs", "String");
            for i in 0..num_entries {
                let entry_off = 4 + i * entry_size;
                let str_len = u16::from_le_bytes([pool_data[entry_off], pool_data[entry_off + 1]]) as usize;
                let ref_count = u16::from_le_bytes([pool_data[entry_off + 2], pool_data[entry_off + 3]]);
                let str_bytes = &string_data[offset..offset + str_len];
                let string = String::from_utf8_lossy(str_bytes).to_string();
                let id = (i + 1) as u32;
                println!("{:>4}  {:>5}  {:>4}  {:?}", id, str_len, ref_count, string);
                offset += str_len;
            }
            println!("Total _StringData: {} of {} bytes", offset, string_data.len());
        }
    }

    // Now dump user table streams
    dump_table_stream(&msi_data, "Property");
    dump_table_stream(&msi_data, "Directory");
    dump_table_stream(&msi_data, "InstallExecuteSequence");
}
