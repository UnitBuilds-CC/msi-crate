/// Progressive test: add tables one at a time to find exact breaking point.
/// Also binary-compares table stream data between Property-only and Property+Directory MSIs.
use velocity_msi::{MsiBuilder, Column, Value};
use std::process::Command;

fn build_msi(tables: &[&str]) -> Vec<u8> {
    let mut b = MsiBuilder::new();
    b.set_title("Progressive");
    b.set_author("V");
    b.set_template("Intel", 1033);

    // Always include Property
    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("ProgTest")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("V")],
        vec![Value::from("ProductCode"), Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![Value::from("UpgradeCode"), Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    if tables.contains(&"Directory") {
        b.create_table("Directory", vec![
            Column::build("Directory").string(72).primary_key().build(),
            Column::build("Directory_Parent").string(72).nullable().build(),
            Column::build("DefaultDir").string(255).primary_key().build(),
        ]).unwrap();
        b.insert_rows("Directory", vec![
            vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        ]).unwrap();
    }

    if tables.contains(&"InstallExecuteSequence") {
        b.create_table("InstallExecuteSequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("InstallExecuteSequence", vec![
            vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
            vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        ]).unwrap();
    }

    if tables.contains(&"InstallUISequence") {
        b.create_table("InstallUISequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("InstallUISequence", vec![
            vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        ]).unwrap();
    }

    b.build().unwrap()
}

fn test_msi(data: &[u8], name: &str) -> i32 {
    let path = format!("prog_{}.msi", name);
    let log = format!("prog_{}.log", name);
    std::fs::write(&path, data).unwrap();
    let _ = std::fs::remove_file(&log);
    let output = Command::new("msiexec")
        .args(&["/i", &path, "/qn", "/norestart", "/lv", &log])
        .output()
        .expect("msiexec failed");
    output.status.code().unwrap_or(-1)
}

fn find_stream_data(data: &[u8], stream_name_encoded: &[u16]) -> Option<Vec<u8>> {
    // Parse OLE to find a specific stream's data by matching its encoded name
    let sector_size = 512usize;
    let first_dir = u32::from_le_bytes([data[48], data[49], data[50], data[51]]) as usize;
    let mini_start = {
        // Read root entry's starting sector
        let dir_base = 512 + first_dir * sector_size;
        u32::from_le_bytes([data[dir_base + 116], data[dir_base + 117], data[dir_base + 118], data[dir_base + 119]])
    };
    let mini_size = {
        let dir_base = 512 + first_dir * sector_size;
        u64::from_le_bytes([
            data[dir_base+120], data[dir_base+121], data[dir_base+122], data[dir_base+123],
            data[dir_base+124], data[dir_base+125], data[dir_base+126], data[dir_base+127],
        ]) as usize
    };

    // Read mini stream container
    let mini_container = {
        let mut result = Vec::new();
        let first_fat = 0u32; // FAT is always at sector 0
        let fat_base = 512 + first_fat as usize * sector_size;
        let mut current = mini_start;
        loop {
            let off = fat_base + current as usize * 4;
            let sector_off = 512 + current as usize * sector_size;
            result.extend_from_slice(&data[sector_off..sector_off + sector_size]);
            let next = u32::from_le_bytes([data[off], data[off+1], data[off+2], data[off+3]]);
            if next == 0xFFFFFFFE || next == 0xFFFFFFFF { break; }
            current = next;
        }
        result.truncate(mini_size);
        result
    };

    // Read MiniFAT
    let first_minifat = u32::from_le_bytes([data[60], data[61], data[62], data[63]]) as usize;
    let minifat_base = 512 + first_minifat * sector_size;

    // Search directory entries for matching name
    let dir_base = 512 + first_dir * sector_size;
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
        if entry_name != stream_name_encoded { continue; }

        // Found the stream - read its data from mini stream
        let start = u32::from_le_bytes([data[off + 116], data[off + 117], data[off + 118], data[off + 119]]);
        let size = u64::from_le_bytes([
            data[off+120], data[off+121], data[off+122], data[off+123],
            data[off+124], data[off+125], data[off+126], data[off+127],
        ]) as usize;

        // Follow MiniFAT chain
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
        return Some(result);
    }
    None
}

fn main() {
    println!("=== Progressive table test ===\n");

    // Test 1: Property only
    let msi1 = build_msi(&[]);
    let code1 = test_msi(&msi1, "prop");
    println!("Property only: exit {}", code1);

    // Test 2: Property + Directory
    let msi2 = build_msi(&["Directory"]);
    let code2 = test_msi(&msi2, "dir");
    println!("Property + Directory: exit {}", code2);

    // Test 3: Property + ExecSeq
    let msi3 = build_msi(&["InstallExecuteSequence"]);
    let code3 = test_msi(&msi3, "exec");
    println!("Property + ExecSeq: exit {}", code3);

    // Test 4: Property + Directory + ExecSeq
    let msi4 = build_msi(&["Directory", "InstallExecuteSequence"]);
    let code4 = test_msi(&msi4, "both");
    println!("Property + Directory + ExecSeq: exit {}", code4);

    // Now binary-compare the Directory stream between test 2 and test 4
    println!("\n=== Binary comparison of Directory stream ===");
    let dir_enc = velocity_msi::encode_stream_name("Directory", true);
    let dir_name_utf16: Vec<u16> = dir_enc.encode_utf16().collect();

    if let Some(dir_in_2) = find_stream_data(&msi2, &dir_name_utf16) {
        if let Some(dir_in_4) = find_stream_data(&msi4, &dir_name_utf16) {
            println!("Directory in test 2: {} bytes", dir_in_2.len());
            println!("Directory in test 4: {} bytes", dir_in_4.len());
            if dir_in_2 == dir_in_4 {
                println!(">>> Directory streams are IDENTICAL");
            } else {
                println!(">>> Directory streams DIFFER!");
                let min_len = dir_in_2.len().min(dir_in_4.len());
                for i in 0..min_len {
                    if dir_in_2[i] != dir_in_4[i] {
                        println!("  First diff at byte {}: test2=0x{:02X} test4=0x{:02X}", i, dir_in_2[i], dir_in_4[i]);
                        break;
                    }
                }
            }
            println!("  test 2 hex: {}", dir_in_2.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "));
            println!("  test 4 hex: {}", dir_in_4.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "));
        } else {
            println!("Could not find Directory stream in test 4");
        }
    } else {
        println!("Could not find Directory stream in test 2");
    }

    // Also compare _Tables and _Columns streams
    for table_name in &["_Tables", "_Columns", "_Validation"] {
        let enc = velocity_msi::encode_stream_name(table_name, true);
        let name_utf16: Vec<u16> = enc.encode_utf16().collect();
        if let Some(s2) = find_stream_data(&msi2, &name_utf16) {
            if let Some(s4) = find_stream_data(&msi4, &name_utf16) {
                let same = if s2 == s4 { "IDENTICAL" } else { "DIFFER" };
                println!("{} stream: {} bytes vs {} bytes → {}", table_name, s2.len(), s4.len(), same);
                if s2 != s4 {
                    println!("  test 2: {}", s2.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "));
                    println!("  test 4: {}", s4.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "));
                }
            }
        }
    }

    // Compare string pools
    for pool_name in &["_StringPool", "_StringData"] {
        let enc = velocity_msi::encode_stream_name(pool_name, true);
        let name_utf16: Vec<u16> = enc.encode_utf16().collect();
        if let Some(s2) = find_stream_data(&msi2, &name_utf16) {
            if let Some(s4) = find_stream_data(&msi4, &name_utf16) {
                let same = if s2 == s4 { "IDENTICAL" } else { "DIFFER" };
                println!("{} stream: {} bytes vs {} bytes → {}", pool_name, s2.len(), s4.len(), same);
            }
        }
    }

    println!("\n=== Summary ===");
    println!("Property only:         {}", if code1 == 0 { "OK" } else { "FAIL" });
    println!("+ Directory:           {}", if code2 == 0 { "OK" } else { "FAIL" });
    println!("+ ExecSeq:             {}", if code3 == 0 { "OK" } else { "FAIL" });
    println!("+ Dir + ExecSeq:       {}", if code4 == 0 { "OK" } else { "FAIL" });
}
