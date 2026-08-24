/// Dump the string pool from a velocity-msi generated MSI and verify
/// that pool IDs map to the correct strings.
use velocity_msi::{MsiBuilder, Column, Value};

fn main() {
    // Build the failing MSI (Property + Directory + ExecSeq)
    let mut b = MsiBuilder::new();
    b.set_title("Pool Dump");
    b.set_author("V");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("PoolDump")],
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

    let msi_data = b.build().unwrap();
    std::fs::write("pool_dump.msi", &msi_data).unwrap();
    println!("Generated MSI: {} bytes\n", msi_data.len());

    // Parse the _StringPool and _StringData streams from the OLE file
    let (pool_data, string_data) = extract_string_pool(&msi_data);

    // Parse _StringPool
    println!("=== _StringPool ({} bytes) ===", pool_data.len());
    let header = u32::from_le_bytes([pool_data[0], pool_data[1], pool_data[2], pool_data[3]]);
    let codepage = header & 0xFFFF;
    let long_refs = (header >> 31) != 0;
    println!("Header: codepage={}, long_refs={}", codepage, long_refs);

    let entry_size = if long_refs { 6 } else { 4 }; // u16 len + u16 refcount (+ u32 hash for long)
    let num_entries = (pool_data.len() - 4) / entry_size;
    println!("Entries: {} (entry_size={})\n", num_entries, entry_size);

    // Parse each entry and show the string it maps to
    let mut offset = 0usize;
    println!("=== String Pool Mapping ===");
    println!("{:>4}  {:>6}  {:>4}  {}", "ID", "Length", "Refs", "String");
    println!("----  ------  ----  ------");
    let mut strings_found: Vec<(u32, String)> = Vec::new();

    for i in 0..num_entries {
        let entry_off = 4 + i * entry_size;
        let str_len = u16::from_le_bytes([pool_data[entry_off], pool_data[entry_off + 1]]) as usize;
        let ref_count = u16::from_le_bytes([pool_data[entry_off + 2], pool_data[entry_off + 3]]);

        // Extract the string bytes from _StringData
        let str_bytes = &string_data[offset..offset + str_len];
        let string = String::from_utf8_lossy(str_bytes).to_string();
        let id = (i + 1) as u32; // 1-based

        // Highlight key strings
        let marker = match string.as_str() {
            "TARGETDIR" | "SourceDir" | "Directory" | "DefaultDir" | "Directory_Parent" =>
                " <<< KEY",
            "CostInitialize" | "CostFinalize" | "InstallExecuteSequence" | "Action" | "Condition" | "Sequence" =>
                " <<< EXEC",
            _ => "",
        };

        println!("{:>4}  {:>6}  {:>4}  {:?}{}", id, str_len, ref_count, string, marker);
        strings_found.push((id, string));

        offset += str_len;
    }

    println!("\nTotal _StringData consumed: {} of {} bytes", offset, string_data.len());

    // Now verify the Directory table references
    println!("\n=== Directory Table Verification ===");
    let dir_stream = extract_table_stream(&msi_data, "Directory");
    if let Some(dir_data) = dir_stream {
        println!("Directory stream: {} bytes", dir_data.len());
        println!("Hex: {}", dir_data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "));

        // Parse: column-major, 3 columns × 1 row, each value = u16
        // Column 0: Directory (PK string)
        let dir_id = u16::from_le_bytes([dir_data[0], dir_data[1]]);
        // Column 1: Directory_Parent (nullable string)
        let parent_id = u16::from_le_bytes([dir_data[2], dir_data[3]]);
        // Column 2: DefaultDir (PK string)
        let default_id = u16::from_le_bytes([dir_data[4], dir_data[5]]);

        println!("\nDirectory col values:");
        println!("  Directory:      pool ID {} → {:?}", dir_id, lookup_string(&strings_found, dir_id as u32));
        println!("  Directory_Parent: pool ID {} → {:?}", parent_id, lookup_string(&strings_found, parent_id as u32));
        println!("  DefaultDir:     pool ID {} → {:?}", default_id, lookup_string(&strings_found, default_id as u32));

        // Verify expected values
        let dir_str = lookup_string(&strings_found, dir_id as u32);
        let parent_str = if parent_id == 0 { "NULL".to_string() } else { lookup_string(&strings_found, parent_id as u32) };
        let default_str = lookup_string(&strings_found, default_id as u32);

        println!("\nExpected: TARGETDIR / NULL / SourceDir");
        println!("Actual:   {} / {} / {}", dir_str, parent_str, default_str);

        if dir_str == "TARGETDIR" && parent_str == "NULL" && default_str == "SourceDir" {
            println!("✓ Directory data is CORRECT!");
        } else {
            println!("✗ Directory data is WRONG!");
        }
    }
}

fn lookup_string(strings: &[(u32, String)], id: u32) -> String {
    strings.iter()
        .find(|(i, _)| *i == id)
        .map(|(_, s)| s.clone())
        .unwrap_or_else(|| format!("<unknown ID {}>", id))
}

fn extract_string_pool(data: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let pool_name = velocity_msi::encode_stream_name("_StringPool", true);
    let data_name = velocity_msi::encode_stream_name("_StringData", true);
    let pool_utf16: Vec<u16> = pool_name.encode_utf16().collect();
    let data_utf16: Vec<u16> = data_name.encode_utf16().collect();

    let pool = extract_stream(data, &pool_utf16).expect("_StringPool not found");
    let string_data = extract_stream(data, &data_utf16).expect("_StringData not found");
    (pool, string_data)
}

fn extract_table_stream(data: &[u8], table_name: &str) -> Option<Vec<u8>> {
    let enc_name = velocity_msi::encode_stream_name(table_name, true);
    let name_utf16: Vec<u16> = enc_name.encode_utf16().collect();
    extract_stream(data, &name_utf16)
}

fn extract_stream(data: &[u8], target_name: &[u16]) -> Option<Vec<u8>> {
    let sector_size = 512usize;
    let first_dir = u32::from_le_bytes([data[48], data[49], data[50], data[51]]) as usize;
    let first_fat = 0u32;
    let fat_base = 512 + first_fat as usize * sector_size;
    let first_minifat = u32::from_le_bytes([data[60], data[61], data[62], data[63]]) as usize;
    let minifat_base = 512 + first_minifat * sector_size;

    // Read mini stream container
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
            result.extend_from_slice(&data[sector_off..sector_off + sector_size]);
            let fat_off = fat_base + current as usize * 4;
            let next = u32::from_le_bytes([data[fat_off], data[fat_off+1], data[fat_off+2], data[fat_off+3]]);
            if next == 0xFFFFFFFE || next == 0xFFFFFFFF { break; }
            current = next;
        }
        result.truncate(mini_size);
        result
    };

    // Search directory entries
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
            let next = u32::from_le_bytes([data[mf_off], data[mf_off+1], data[mf_off+2], data[mf_off+3]]);
            if next == 0xFFFFFFFE || next == 0xFFFFFFFF { break; }
            current = next;
        }
        result.truncate(size);
        return Some(result);
    }
    None
}
