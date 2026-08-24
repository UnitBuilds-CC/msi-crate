/// Test: Is error 2705 specific to InstallExecuteSequence, or does ANY third table trigger it?
use velocity_msi::{MsiBuilder, Column, Value};

fn build_and_test(label: &str, build_fn: impl FnOnce(&mut MsiBuilder)) {
    let mut builder = MsiBuilder::new();
    builder.set_title("Third Table Test");
    builder.set_author("Velocity");
    builder.set_template("Intel", 1033);

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(1024).build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Third Table Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity Corp")],
        vec![Value::from("ProductCode"), Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![Value::from("UpgradeCode"), Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("VelTest:VelTest")],
    ]).unwrap();

    build_fn(&mut builder);

    let fname = format!("third_{}.msi", label.to_lowercase().replace(' ', "_"));
    let msi = builder.build().unwrap();
    std::fs::write(&fname, &msi).unwrap();

    let _ = std::fs::remove_dir_all("C:\\VelTest");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", &fname, "/qn", "/l*v", &format!("third_{}.log", label.to_lowercase().replace(' ', "_"))])
        .output().unwrap();
    let exit = output.status.code().unwrap_or(-1);
    println!("{:40} exit={}", label, exit);

    if exit != 0 {
        let log_name = format!("third_{}.log", label.to_lowercase().replace(' ', "_"));
        if let Ok(log) = std::fs::read_to_string(&log_name) {
            for line in log.lines() {
                if (line.contains("Error ") && !line.contains("Error 0") && !line.contains("2205") && !line.contains("2228"))
                    || line.contains("return value 3")
                {
                    println!("  {}", line.trim());
                }
            }
        }
    }
}

fn main() {
    println!("=== Third table test ===\n");

    // Test 1: No third table (baseline)
    build_and_test("NoThird", |_m| {});

    // Test 2: Add a dummy table (1 column, 1 row)
    build_and_test("DummyTable", |m| {
        m.create_table("DummyTable", vec![
            Column::build("Dummy").string(72).primary_key().build(),
        ]).unwrap();
        m.insert_rows("DummyTable", vec![
            vec![Value::from("TestValue")],
        ]).unwrap();
    });

    // Test 3: Add InstallExecuteSequence (same structure as dummy)
    build_and_test("ExecSeq", |m| {
        m.create_table("InstallExecuteSequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        m.insert_rows("InstallExecuteSequence", vec![
            vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        ]).unwrap();
    });

    // Test 4: Add InstallUISequence (similar to InstallExecuteSequence)
    build_and_test("InstallUISeq", |m| {
        m.create_table("InstallUISequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        m.insert_rows("InstallUISequence", vec![
            vec![Value::from("ExecuteAction"), Value::Null, Value::Int(1000)],
        ]).unwrap();
    });

    // Test 5: Add BOTH InstallExecuteSequence AND InstallUISequence
    build_and_test("BothSeq", |m| {
        m.create_table("InstallExecuteSequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        m.insert_rows("InstallExecuteSequence", vec![
            vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        ]).unwrap();

        m.create_table("InstallUISequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        m.insert_rows("InstallUISequence", vec![
            vec![Value::from("ExecuteAction"), Value::Null, Value::Int(1000)],
        ]).unwrap();
    });

    // Test 6: ExecSeq with ONLY CostInitialize (minimal)
    build_and_test("ExecSeq_Minimal", |m| {
        m.create_table("InstallExecuteSequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        // No rows! Empty table gets filtered out by build()
    });

    // Test 7: ExecSeq with 1 row named "ZZZ" (to test if name matters)
    build_and_test("ExecSeq_ZZZ", |m| {
        m.create_table("InstallExecuteSequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        m.insert_rows("InstallExecuteSequence", vec![
            vec![Value::from("ZZZAction"), Value::Null, Value::Int(9999)],
        ]).unwrap();
    });

    println!("\nDone.");
}
