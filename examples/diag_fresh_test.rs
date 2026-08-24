/// Focused diagnostic: build MSI with Property + Directory + InstallExecuteSequence
/// using velocity-msi, test with msiexec, and capture detailed error info.
///
/// Also creates a SECOND MSI using the msi crate (open + flush) to compare behavior.
use velocity_msi::{MsiBuilder, Column, Value};
use std::process::Command;

fn main() {
    println!("=== velocity-msi diagnostic: Directory + ExecSeq ===\n");

    // Build MSI with velocity-msi
    let mut b = MsiBuilder::new();
    b.set_title("Diag Fresh");
    b.set_author("V");
    b.set_template("Intel", 1033);

    // Property table
    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("DiagFresh")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("V")],
        vec![Value::from("ProductCode"), Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![Value::from("UpgradeCode"), Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    // Directory table (single row - TARGETDIR only)
    b.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
    ]).unwrap();

    // InstallExecuteSequence
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
    std::fs::write("diag_fresh.msi", &msi_data).unwrap();
    println!("Generated MSI: {} bytes", msi_data.len());

    // Dump stream names and sizes from OLE
    dump_ole_streams(&msi_data);

    // Test with msiexec
    let _ = std::fs::remove_file("diag_fresh.log");
    let output = Command::new("msiexec")
        .args(&["/i", "diag_fresh.msi", "/qn", "/norestart", "/lv", "diag_fresh.log"])
        .output()
        .expect("msiexec failed");
    let exit_code = output.status.code().unwrap_or(-1);
    println!("\nmsiexec exit code: {}", exit_code);

    // Read and analyze log
    if let Ok(log) = std::fs::read_to_string("diag_fresh.log") {
        println!("\n=== Key log entries ===");
        for line in log.lines() {
            let lt = line.trim();
            if lt.contains("2705") || lt.contains("1620") || lt.contains("1603")
                || lt.contains("DEBUG: Error") || lt.contains("return value 3")
                || lt.contains("Error 2705") || lt.contains("could not be linked")
                || lt.contains("CostInitialize")
            {
                println!("  {}", lt);
            }
        }

        // Also look for the action that failed
        println!("\n=== Last 30 lines before return value 3 ===");
        let lines: Vec<&str> = log.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.contains("return value 3") {
                let start = if i > 30 { i - 30 } else { 0 };
                for j in start..=i {
                    println!("  {}", lines[j].trim());
                }
                break;
            }
        }
    }

    // Now try with msi crate: open our MSI, drop+recreate Directory table, flush
    println!("\n\n=== msi crate recreate test ===");
    {
        let file = std::fs::OpenOptions::new()
            .read(true).write(true)
            .open("diag_fresh.msi").unwrap();
        let mut pkg = msi::Package::open(file).unwrap();

        // Read current Directory data
        println!("Reading Directory table...");
        let mut rows = pkg.select_rows(msi::Select::table("Directory")).unwrap();
        for row in rows {
            println!("  {:?} {:?} {:?}", &row[0], &row[1], &row[2]);
        }

        // Read _Tables
        println!("\nReading _Tables...");
        let mut rows = pkg.select_rows(msi::Select::table("_Tables")).unwrap();
        for row in rows {
            println!("  {:?}", &row[0]);
        }

        // Drop and recreate Directory
        pkg.drop_table("Directory").unwrap();
        println!("\nDropped Directory table.");

        pkg.create_table("Directory", vec![
            msi::Column::build("Directory").primary_key().string(72),
            msi::Column::build("Directory_Parent").nullable().string(72),
            msi::Column::build("DefaultDir").primary_key().string(255),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Directory")
            .row(vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str("SourceDir".into())])
        ).unwrap();
        println!("Recreated Directory with msi crate API.");

        pkg.flush().unwrap();
        println!("Flushed.");
    }

    // Test modified MSI
    let _ = std::fs::remove_file("diag_fresh_mod.log");
    let output2 = Command::new("msiexec")
        .args(&["/i", "diag_fresh.msi", "/qn", "/norestart", "/lv", "diag_fresh_mod.log"])
        .output()
        .expect("msiexec failed");
    let exit_code2 = output2.status.code().unwrap_or(-1);
    println!("\nModified MSI msiexec exit code: {}", exit_code2);

    if let Ok(log) = std::fs::read_to_string("diag_fresh_mod.log") {
        println!("\n=== Modified MSI key log entries ===");
        for line in log.lines() {
            let lt = line.trim();
            if lt.contains("2705") || lt.contains("1620") || lt.contains("1603")
                || lt.contains("DEBUG: Error") || lt.contains("return value 3")
                || lt.contains("Error 2705") || lt.contains("could not be linked")
                || lt.contains("CostInitialize")
            {
                println!("  {}", lt);
            }
        }
    }

    println!("\n=== RESULTS ===");
    println!("Original:   exit code {}", exit_code);
    println!("Modified:   exit code {}", exit_code2);
    if exit_code != 0 && exit_code2 == 0 {
        println!("\n>>> SERIALIZATION BUG in velocity-msi table data!");
    } else if exit_code != 0 && exit_code2 != 0 {
        println!("\n>>> Both fail. Issue is in OLE structure or system tables.");
    } else if exit_code == 0 {
        println!("\n>>> Original works! Issue may have been fixed.");
    }
}

/// Dump OLE stream names and sizes by parsing the directory entries
fn dump_ole_streams(data: &[u8]) {
    if data.len() < 512 { return; }
    // Read header fields
    let major = u16::from_le_bytes([data[26], data[27]]);
    let sector_shift = u16::from_le_bytes([data[30], data[31]]) as usize;
    let sector_size = 1usize << sector_shift;
    let num_fat = u32::from_le_bytes([data[44], data[45], data[46], data[47]]) as usize;
    let first_dir = u32::from_le_bytes([data[48], data[49], data[50], data[51]]) as usize;
    let mini_cutoff = u32::from_le_bytes([data[56], data[57], data[58], data[59]]);
    let first_minifat = u32::from_le_bytes([data[60], data[61], data[62], data[63]]) as usize;
    let first_mini_sector = {
        // Read FAT to find mini stream start
        let _fat_base = 512; // First FAT sector is always at offset 512 (sector 0)
        // Mini stream sectors come after FAT + Dir + MiniFAT sectors
        // For simplicity, just report what we can from directory entries
        0 // placeholder
    };
    let _ = first_mini_sector;

    println!("\nOLE Header: v{}, sector={}B, mini_cutoff={}B", major, sector_size, mini_cutoff);
    println!("FAT sectors: {}, first_dir: {}, first_minifat: {}", num_fat, first_dir, first_minifat);

    // Read directory entries
    let dir_base = 512 + first_dir * sector_size;
    println!("\nDirectory entries (at offset {}):", dir_base);
    for i in 0..32 {
        let off = dir_base + i * 128;
        if off + 128 > data.len() { break; }
        // Check for empty entry (all zeros in name area)
        let name_len = u16::from_le_bytes([data[off + 64], data[off + 65]]) as usize;
        if name_len == 0 { continue; }
        let name_bytes = name_len.saturating_sub(2).min(64); // cap at 64 bytes (entry name field)
        let mut name_utf16 = Vec::new();
        for j in 0..(name_bytes / 2) {
            if off + j * 2 + 1 >= data.len() { break; }
            name_utf16.push(u16::from_le_bytes([data[off + j * 2], data[off + j * 2 + 1]]));
        }
        let name = String::from_utf16_lossy(&name_utf16);
        let obj_type = data[off + 66];
        let color = data[off + 67];
        let left = i32::from_le_bytes([data[off + 68], data[off + 69], data[off + 70], data[off + 71]]);
        let right = i32::from_le_bytes([data[off + 72], data[off + 73], data[off + 74], data[off + 75]]);
        let child = i32::from_le_bytes([data[off + 76], data[off + 77], data[off + 78], data[off + 79]]);
        let start = u32::from_le_bytes([data[off + 116], data[off + 117], data[off + 118], data[off + 119]]);
        let size = u64::from_le_bytes([
            data[off+120], data[off+121], data[off+122], data[off+123],
            data[off+124], data[off+125], data[off+126], data[off+127],
        ]);
        let type_str = match obj_type { 0 => "empty", 1 => "user", 2 => "stream", 5 => "root", _ => "?" };
        println!("  [{}] name={:?} type={} color={} L={} R={} child={} start={} size={}",
            i, name, type_str, color, left, right, child, start, size);
    }
}
