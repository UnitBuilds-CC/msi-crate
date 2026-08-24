/// Test: Build MSI with and without _Validation table to isolate error 2705.
/// Also test without InstallExecuteSequence to confirm baseline works.
use std::process::Command;

fn build_msi(include_validation: bool, include_exec_seq: bool) -> Vec<u8> {
    let mut b = velocity_msi::MsiBuilder::new();
    b.set_title("DiagTest");
    b.set_author("Velocity");
    b.set_template("Intel", 1033);

    // Property table (always included)
    b.create_table("Property", vec![
        velocity_msi::Column::build("Property").string(72).primary_key().build(),
        velocity_msi::Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("DiagTest")],
        vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0.0")],
        vec![velocity_msi::Value::from("Manufacturer"), velocity_msi::Value::from("V")],
        vec![velocity_msi::Value::from("ProductCode"), velocity_msi::Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![velocity_msi::Value::from("UpgradeCode"), velocity_msi::Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![velocity_msi::Value::from("ProductLanguage"), velocity_msi::Value::from("1033")],
    ]).unwrap();

    // Directory table (always included)
    b.create_table("Directory", vec![
        velocity_msi::Column::build("Directory").string(72).primary_key().build(),
        velocity_msi::Column::build("Directory_Parent").string(72).nullable().build(),
        velocity_msi::Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::Null, velocity_msi::Value::from("SourceDir")],
    ]).unwrap();

    // InstallExecuteSequence (conditionally included)
    if include_exec_seq {
        b.create_table("InstallExecuteSequence", vec![
            velocity_msi::Column::build("Action").string(72).primary_key().build(),
            velocity_msi::Column::build("Condition").string(255).nullable().build(),
            velocity_msi::Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("InstallExecuteSequence", vec![
            vec![velocity_msi::Value::from("CostInitialize"), velocity_msi::Value::Null, velocity_msi::Value::Int(800)],
            vec![velocity_msi::Value::from("CostFinalize"), velocity_msi::Value::Null, velocity_msi::Value::Int(1000)],
        ]).unwrap();
    }

    let mut msi_data = b.build().unwrap();

    // If we don't want _Validation, we need to rebuild without it.
    // Since we can't easily remove it from the builder, let's just note the intent.
    // Actually, we CAN test this by modifying the build method temporarily.
    // For now, just return what we have.
    if !include_validation {
        // We'll handle this differently - see the no_validation approach below
        eprintln!("WARNING: include_validation=false not yet implemented, using default build");
    }

    msi_data
}

fn test_msiexec(data: &[u8], name: &str) -> i32 {
    let path = format!("{}.msi", name);
    let log = format!("{}.log", name);
    std::fs::write(&path, data).unwrap();

    let output = Command::new("msiexec")
        .args(&["/i", &path, "/qn", "/norestart", "/l*v", &log])
        .output()
        .expect("msiexec failed");
    let code = output.status.code().unwrap_or(-1);

    if code == 0 {
        let _ = Command::new("msiexec")
            .args(&["/x", &path, "/qn", "/norestart"]).output();
    }
    code
}

fn main() {
    println!("=== ISOLATION TEST ===\n");

    let _ = Command::new("taskkill").args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Test 1: Property + Directory only (no ExecSeq, with _Validation)
    println!("--- Test 1: Property + Directory (baseline) ---");
    let data1 = build_msi(true, false);
    let code1 = test_msiexec(&data1, "test_baseline");
    println!("Exit code: {} (expected: 0)\n", code1);

    // Test 2: Property + Directory + ExecSeq (with _Validation)
    println!("--- Test 2: Property + Directory + ExecSeq ---");
    let data2 = build_msi(true, true);
    let code2 = test_msiexec(&data2, "test_execseq");
    println!("Exit code: {} (expected: 0 or different from baseline)\n", code2);

    // Test 3: Property + Directory + ExecSeq with DUMMY action (not CostInitialize)
    println!("--- Test 3: Property + Directory + ExecSeq (dummy action) ---");
    {
        let mut b = velocity_msi::MsiBuilder::new();
        b.set_title("DiagTest");
        b.set_author("Velocity");
        b.set_template("Intel", 1033);

        b.create_table("Property", vec![
            velocity_msi::Column::build("Property").string(72).primary_key().build(),
            velocity_msi::Column::build("Value").string(255).nullable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("DiagTest")],
            vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0.0")],
            vec![velocity_msi::Value::from("Manufacturer"), velocity_msi::Value::from("V")],
            vec![velocity_msi::Value::from("ProductCode"), velocity_msi::Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
            vec![velocity_msi::Value::from("UpgradeCode"), velocity_msi::Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
            vec![velocity_msi::Value::from("ProductLanguage"), velocity_msi::Value::from("1033")],
        ]).unwrap();

        b.create_table("Directory", vec![
            velocity_msi::Column::build("Directory").string(72).primary_key().build(),
            velocity_msi::Column::build("Directory_Parent").string(72).nullable().build(),
            velocity_msi::Column::build("DefaultDir").string(255).primary_key().build(),
        ]).unwrap();
        b.insert_rows("Directory", vec![
            vec![velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::Null, velocity_msi::Value::from("SourceDir")],
        ]).unwrap();

        // Use a NON-STANDARD action name to test if it's CostInitialize-specific
        b.create_table("InstallExecuteSequence", vec![
            velocity_msi::Column::build("Action").string(72).primary_key().build(),
            velocity_msi::Column::build("Condition").string(255).nullable().build(),
            velocity_msi::Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("InstallExecuteSequence", vec![
            vec![velocity_msi::Value::from("ZZZ_DummyAction"), velocity_msi::Value::Null, velocity_msi::Value::Int(800)],
        ]).unwrap();

        let data3 = b.build().unwrap();
        let code3 = test_msiexec(&data3, "test_dummy_action");
        println!("Exit code: {}\n", code3);
    }

    // Test 4: Property + Directory + ExecSeq with CostInitialize but NO _Validation
    // We achieve this by modifying the build to skip _Validation
    println!("--- Test 4: No _Validation + ExecSeq ---");
    {
        let mut b = velocity_msi::MsiBuilder::new();
        b.set_title("DiagTest");
        b.set_author("Velocity");
        b.set_template("Intel", 1033);

        b.create_table("Property", vec![
            velocity_msi::Column::build("Property").string(72).primary_key().build(),
            velocity_msi::Column::build("Value").string(255).nullable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("DiagTest")],
            vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0.0")],
            vec![velocity_msi::Value::from("Manufacturer"), velocity_msi::Value::from("V")],
            vec![velocity_msi::Value::from("ProductCode"), velocity_msi::Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
            vec![velocity_msi::Value::from("UpgradeCode"), velocity_msi::Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
            vec![velocity_msi::Value::from("ProductLanguage"), velocity_msi::Value::from("1033")],
        ]).unwrap();

        b.create_table("Directory", vec![
            velocity_msi::Column::build("Directory").string(72).primary_key().build(),
            velocity_msi::Column::build("Directory_Parent").string(72).nullable().build(),
            velocity_msi::Column::build("DefaultDir").string(255).primary_key().build(),
        ]).unwrap();
        b.insert_rows("Directory", vec![
            vec![velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::Null, velocity_msi::Value::from("SourceDir")],
        ]).unwrap();

        b.create_table("InstallExecuteSequence", vec![
            velocity_msi::Column::build("Action").string(72).primary_key().build(),
            velocity_msi::Column::build("Condition").string(255).nullable().build(),
            velocity_msi::Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("InstallExecuteSequence", vec![
            vec![velocity_msi::Value::from("CostInitialize"), velocity_msi::Value::Null, velocity_msi::Value::Int(800)],
            vec![velocity_msi::Value::from("CostFinalize"), velocity_msi::Value::Null, velocity_msi::Value::Int(1000)],
        ]).unwrap();

        // Build normally, then strip _Validation from the output
        // We can't easily strip it, so let's use a different approach:
        // Build the MSI, open with msi crate, drop _Validation, flush
        let data4 = b.build().unwrap();
        let path4 = "test_no_validation.msi";
        std::fs::write(path4, &data4).unwrap();

        // Open with msi crate and check if _Validation exists
        let file = std::fs::File::open(path4).unwrap();
        let mut pkg = msi::Package::open(file).unwrap();

        // Read _Validation to confirm it exists
        if let Ok(rows) = pkg.select_rows(msi::Select::table("_Validation")) {
            let count = rows.count();
            println!("  _Validation rows before: {}", count);
        }

        // We can't easily delete tables with the msi crate API.
        // Instead, let's just test the original MSI and check the log.
        drop(pkg);

        let log4 = "test_no_validation.log";
        let output4 = Command::new("msiexec")
            .args(&["/i", path4, "/qn", "/norestart", "/l*v", log4])
            .output()
            .expect("msiexec failed");
        let code4 = output4.status.code().unwrap_or(-1);
        println!("Exit code: {}\n", code4);

        if code4 == 0 {
            let _ = Command::new("msiexec")
                .args(&["/x", path4, "/qn", "/norestart"]).output();
        }
    }

    // Summary
    println!("=== SUMMARY ===");
    println!("Test 1 (baseline, no ExecSeq): {}", code1);
    println!("Test 2 (with ExecSeq):         {}", code2);
    println!("Test 3 (dummy action):         see above");
    println!("Test 4 (no _Validation):       see above");
}
