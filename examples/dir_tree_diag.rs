/// Test: Isolate why CostInitialize can't link the Directory tree (error 2705).
/// Tests various Directory table configurations with InstallExecuteSequence.
use velocity_msi::{MsiBuilder, Column, Value};

fn build_msi(label: &str, dir_rows: Vec<Vec<Value>>, seq_rows: Vec<Vec<Value>>) -> i32 {
    let mut builder = MsiBuilder::new();
    builder.set_title("Dir Tree Test");
    builder.set_author("Velocity");
    builder.set_template("Intel", 1033);

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(1024).build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Dir Tree Test")],
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
    builder.insert_rows("Directory", dir_rows).unwrap();

    if !seq_rows.is_empty() {
        builder.create_table("InstallExecuteSequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        builder.insert_rows("InstallExecuteSequence", seq_rows).unwrap();
    }

    let fname = format!("dir_{}.msi", label.to_lowercase());
    let msi = builder.build().unwrap();
    std::fs::write(&fname, &msi).unwrap();

    let _ = std::fs::remove_dir_all("C:\\VelTest");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", &fname, "/qn", "/l*v", &format!("dir_{}.log", label.to_lowercase())])
        .output().unwrap();
    let exit = output.status.code().unwrap_or(-1);
    println!("{:40} exit={}", label, exit);
    if exit != 0 {
        let log_name = format!("dir_{}.log", label.to_lowercase());
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
    exit
}

fn main() {
    println!("=== Directory tree diagnostic ===\n");

    let seq = vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
    ];

    // Test 1: Baseline - no sequence table (proven to work)
    build_msi("Baseline", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("VelTest")],
    ], vec![]);

    // Test 2: DefaultDir with pipe format "SHORT|Long"
    build_msi("PipeFmt", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("VELTEST|VelTest")],
    ], seq.clone());

    // Test 3: DefaultDir with simple name (no pipe, no colon)
    build_msi("SimpleName", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("VelTest")],
    ], seq.clone());

    // Test 4: Only TARGETDIR (no child directories)
    build_msi("RootOnly", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
    ], seq.clone());

    // Test 5: TARGETDIR with different DefaultDir format
    build_msi("TargetDot", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from(".")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("VelTest")],
    ], seq.clone());

    // Test 6: Three-level tree (TARGETDIR -> ProgramFilesFolder -> INSTALLDIR)
    build_msi("ThreeLevel", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("ProgramFilesFolder"), Value::from("TARGETDIR"), Value::from("PFiles")],
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFilesFolder"), Value::from("VelTest")],
    ], seq.clone());

    // Test 7: Same as Test 3 but with CostInitialize only (no CostFinalize)
    build_msi("CostInitOnly", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("VelTest")],
    ], vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
    ]);

    // Test 8: Same as Test 3 but with CostFinalize only (no CostInitialize)
    build_msi("CostFinOnly", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("VelTest")],
    ], vec![
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
    ]);

    // Test 9: Directory_Parent column NOT nullable (wrong schema)
    // Actually let's test with the correct nullable on Directory_Parent
    // but with a different DefaultDir for TARGETDIR
    build_msi("TargetSrcNew", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceNewDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("VelTest")],
    ], seq.clone());

    println!("\nDone.");
}
