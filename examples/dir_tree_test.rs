/// Diagnostic: test different Directory table configurations to find what
/// makes CostInitialize happy. Also uses msi crate to read back our MSI.
use velocity_msi::{MsiBuilder, Column, Value};
use std::process::Command;

fn build_msi(dir_rows: Vec<Vec<Value>>) -> Vec<u8> {
    let mut b = MsiBuilder::new();
    b.set_title("DirTest");
    b.set_author("V");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("DirTest")],
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
    b.insert_rows("Directory", dir_rows).unwrap();

    b.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
    ]).unwrap();

    b.build().unwrap()
}

fn test_msi(data: &[u8], name: &str) -> (i32, String) {
    let path = format!("dir_test_{}.msi", name);
    let log = format!("dir_test_{}.log", name);
    std::fs::write(&path, data).unwrap();
    let _ = std::fs::remove_file(&log);
    let output = Command::new("msiexec")
        .args(&["/i", &path, "/qn", "/norestart", "/lv", &log])
        .output()
        .expect("msiexec failed");
    let code = output.status.code().unwrap_or(-1);
    let error = if code != 0 {
        std::fs::read_to_string(&log).ok()
            .and_then(|content| {
                content.lines().find(|l| l.contains("2705") || l.contains("DEBUG: Error"))
                    .map(|l| l.trim().to_string())
            })
            .unwrap_or_default()
    } else { String::new() };
    // Also try reading with msi crate
    (code, error)
}

fn try_read_with_msi_crate(data: &[u8]) {
    // Try to open our MSI with the msi crate and read the Directory table
    let cursor = std::io::Cursor::new(data.to_vec());
    match msi::Package::open(cursor) {
        Ok(mut package) => {
            println!("  msi crate: opened OK");
            match package.select_rows(msi::Select::table("Directory")) {
                Ok(rows) => {
                    let mut count = 0;
                    for row in rows {
                        count += 1;
                        let mut vals = Vec::new();
                        for i in 0..row.len() {
                            let v = &row[i];
                            if let Some(s) = v.as_str() { vals.push(format!("S={}", s)); }
                            else if let Some(i) = v.as_int() { vals.push(format!("I={}", i)); }
                            else { vals.push("Null".to_string()); }
                        }
                        println!("    row: {:?}", vals);
                    }
                    println!("  msi crate: Directory has {} rows", count);
                }
                Err(e) => println!("  msi crate: Directory read error: {:?}", e),
            }
            // Also check _Tables
            match package.select_rows(msi::Select::table("_Tables")) {
                Ok(rows) => {
                    let mut count = 0;
                    for row in rows {
                        count += 1;
                        let name = row[0].as_str().unwrap_or("?");
                        println!("    table: {}", name);
                    }
                    println!("  msi crate: _Tables has {} rows", count);
                }
                Err(e) => println!("  msi crate: _Tables read error: {:?}", e),
            }
        }
        Err(e) => println!("  msi crate: open error: {:?}", e),
    }
}

fn main() {
    println!("=== Testing Directory table configurations ===\n");

    // Test 1: TARGETDIR with NULL parent (current behavior)
    println!("Test 1: TARGETDIR, NULL parent, SourceDir");
    let msi = build_msi(vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
    ]);
    let (code, err) = test_msi(&msi, "t1");
    println!("  exit {} {}", code, err);
    try_read_with_msi_crate(&msi);
    println!();

    // Test 2: TARGETDIR with SELF parent
    println!("Test 2: TARGETDIR, TARGETDIR parent, SourceDir");
    let msi = build_msi(vec![
        vec![Value::from("TARGETDIR"), Value::from("TARGETDIR"), Value::from("SourceDir")],
    ]);
    let (code, err) = test_msi(&msi, "t2");
    println!("  exit {} {}", code, err);
    println!();

    // Test 3: TARGETDIR + subdirectory
    println!("Test 3: TARGETDIR + AppDir subdirectory");
    let msi = build_msi(vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("AppDir"), Value::from("TARGETDIR"), Value::from("MyApp")],
    ]);
    let (code, err) = test_msi(&msi, "t3");
    println!("  exit {} {}", code, err);
    println!();

    // Test 4: TARGETDIR with "." DefaultDir
    println!("Test 4: TARGETDIR, NULL parent, dot (.)");
    let msi = build_msi(vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from(".")],
    ]);
    let (code, err) = test_msi(&msi, "t4");
    println!("  exit {} {}", code, err);
    println!();

    // Test 5: TARGETDIR self-parent + subdirectory
    println!("Test 5: TARGETDIR self-parent + AppDir");
    let msi = build_msi(vec![
        vec![Value::from("TARGETDIR"), Value::from("TARGETDIR"), Value::from("SourceDir")],
        vec![Value::from("AppDir"), Value::from("TARGETDIR"), Value::from("MyApp")],
    ]);
    let (code, err) = test_msi(&msi, "t5");
    println!("  exit {} {}", code, err);
    println!();

    // Test 6: Three-level tree
    println!("Test 6: TARGETDIR + ProgramFilesFolder + AppDir");
    let msi = build_msi(vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("ProgramFilesFolder"), Value::from("TARGETDIR"), Value::from("PFiles")],
        vec![Value::from("AppDir"), Value::from("ProgramFilesFolder"), Value::from("MyApp")],
    ]);
    let (code, err) = test_msi(&msi, "t6");
    println!("  exit {} {}", code, err);
    println!();

    // Test 7: No ExecSeq, just Directory (should still work)
    println!("Test 7: Directory only (no ExecSeq) - baseline");
    {
        let mut b = MsiBuilder::new();
        b.set_title("DirTest");
        b.set_author("V");
        b.set_template("Intel", 1033);
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("DirTest")],
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
        let msi_data = b.build().unwrap();
        std::fs::write("dir_test_t7.msi", &msi_data).unwrap();
        println!("  (no ExecSeq - should open OK)");
        try_read_with_msi_crate(&msi_data);
    }
}
