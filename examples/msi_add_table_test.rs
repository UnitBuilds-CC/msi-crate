/// Key test: Take working MSI (Property+Directory = exit 0),
/// add InstallExecuteSequence via msi crate, test if it works.
/// This isolates whether the bug is in velocity-msi or the msi crate's ExecSeq handling.
use velocity_msi::{MsiBuilder, Column, Value};
use std::process::Command;

fn main() {
    // Step 1: Build working MSI (Property + Directory)
    let mut b = MsiBuilder::new();
    b.set_title("MSI Crate Add Test");
    b.set_author("V");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("MsiAddTest")],
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
    std::fs::write("msi_add_test.msi", &msi_data).unwrap();

    // Test baseline (should be exit 0)
    let code0 = test_msi("msi_add_test.msi", "msi_add_base");
    println!("Baseline (Property+Directory): exit {}", code0);

    // Step 2: Open with msi crate, add InstallExecuteSequence, flush
    {
        let file = std::fs::OpenOptions::new()
            .read(true).write(true)
            .open("msi_add_test.msi").unwrap();
        let mut pkg = msi::Package::open(file).unwrap();

        // Add InstallExecuteSequence using msi crate's API
        pkg.create_table("InstallExecuteSequence", vec![
            msi::Column::build("Action").primary_key().string(72),
            msi::Column::build("Condition").nullable().string(255),
            msi::Column::build("Sequence").nullable().int16(),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("InstallExecuteSequence")
            .row(vec![msi::Value::Str("CostInitialize".into()), msi::Value::Null, msi::Value::Int(800)])
            .row(vec![msi::Value::Str("CostFinalize".into()), msi::Value::Null, msi::Value::Int(1000)])
        ).unwrap();
        println!("Added InstallExecuteSequence via msi crate.");

        pkg.flush().unwrap();
        println!("Flushed.");
    }

    // Step 3: Test modified MSI
    let code1 = test_msi("msi_add_test.msi", "msi_add_exec");
    println!("After adding ExecSeq (msi crate): exit {}", code1);

    // Step 4: Also try the reverse - build with ExecSeq, add Directory via msi crate
    println!("\n--- Reverse test: build with ExecSeq, add Directory ---");
    let mut b2 = MsiBuilder::new();
    b2.set_title("Msi Add Dir Test");
    b2.set_author("V");
    b2.set_template("Intel", 1033);

    b2.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b2.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("MsiAddDirTest")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("V")],
        // Different GUID to avoid conflicts
        vec![Value::from("ProductCode"), Value::from("{BBBDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![Value::from("UpgradeCode"), Value::from("{CCCDD5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    b2.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b2.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
    ]).unwrap();

    let msi_data2 = b2.build().unwrap();
    std::fs::write("msi_add_dir_test.msi", &msi_data2).unwrap();

    // Test baseline (Property + ExecSeq, should be exit 0)
    let code2 = test_msi("msi_add_dir_test.msi", "msi_add_dir_base");
    println!("Baseline (Property+ExecSeq): exit {}", code2);

    // Open with msi crate, add Directory
    {
        let file = std::fs::OpenOptions::new()
            .read(true).write(true)
            .open("msi_add_dir_test.msi").unwrap();
        let mut pkg = msi::Package::open(file).unwrap();

        pkg.create_table("Directory", vec![
            msi::Column::build("Directory").primary_key().string(72),
            msi::Column::build("Directory_Parent").nullable().string(72),
            msi::Column::build("DefaultDir").primary_key().string(255),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Directory")
            .row(vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str("SourceDir".into())])
        ).unwrap();
        println!("Added Directory via msi crate.");

        pkg.flush().unwrap();
        println!("Flushed.");
    }

    let code3 = test_msi("msi_add_dir_test.msi", "msi_add_dir_mod");
    println!("After adding Directory (msi crate): exit {}", code3);

    println!("\n=== RESULTS ===");
    println!("Property+Dir:                  exit {} {}", code0, if code0 == 0 { "✓" } else { "✗" });
    println!("+ ExecSeq (msi crate):         exit {} {}", code1, if code1 == 0 { "✓" } else { "✗" });
    println!("Property+ExecSeq:              exit {} {}", code2, if code2 == 0 { "✓" } else { "✗" });
    println!("+ Directory (msi crate):       exit {} {}", code3, if code3 == 0 { "✓" } else { "✗" });

    if code0 == 0 && code1 != 0 {
        println!("\n>>> msi crate CANNOT add ExecSeq to working MSI.");
        println!(">>> This means the issue is in how _Tables/_Columns/string pool");
        println!(">>> are rebuilt when a new table is added.");
    }
    if code2 == 0 && code3 != 0 {
        println!("\n>>> msi crate CANNOT add Directory to working ExecSeq MSI.");
        println!(">>> Same issue - rebuilding metadata when adding tables is broken.");
    }
    if code1 == 0 || code3 == 0 {
        println!("\n>>> msi crate CAN combine the tables successfully!");
        println!(">>> The bug is in velocity-msi's serialization of the combined tables.");
    }
}

fn test_msi(path: &str, log_name: &str) -> i32 {
    let log = format!("{}.log", log_name);
    let _ = std::fs::remove_file(&log);
    let output = Command::new("msiexec")
        .args(&["/i", path, "/qn", "/norestart", "/lv", &log])
        .output()
        .expect("msiexec failed");
    let code = output.status.code().unwrap_or(-1);

    // Print key errors from log
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
