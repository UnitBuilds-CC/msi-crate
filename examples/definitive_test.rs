/// Definitive test: Build a complete MSI with all required install tables
/// and test with msiexec. Goal: error 0 (success).
///
/// cargo run --example definitive_test -p velocity-msi
use velocity_msi::{Column, MsiBuilder, Value};

fn make_uuid() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let a = (t & 0xFFFFFFFF) as u32;
    let b = ((t >> 32) & 0xFFFF) as u16;
    let c = (((t >> 48) & 0x0FFF) as u16) | 0x4000;
    let d = ((t >> 64) as u16 & 0x3FFF) | 0x8000;
    let e = (t >> 80) as u64;
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:04X}-{:012X}}}",
        a, b, c, d, e & 0xFFFFFFFFFFFF
    )
}

fn main() {
    println!("=== DEFINITIVE MSI TEST ===\n");

    let out_path = "C:\\temp\\definitive_test.msi";
    let log_path = "C:\\temp\\definitive_test.log";
    let _ = std::fs::remove_file(out_path);
    let _ = std::fs::remove_file(log_path);

    let product_code = make_uuid();
    let upgrade_code = make_uuid();

    let mut builder = MsiBuilder::new();
    builder.set_title("Installation Database");
    builder.set_author("Velocity Corp");
    builder.set_subject("Velocity Test Product");
    builder.set_template("Intel", 1033);

    // === Property table ===
    builder
        .create_table(
            "Property",
            vec![
                Column::build("Property")
                    .string(72)
                    .primary_key()
                    .build(),
                Column::build("Value")
                    .nullable()
                    .localizable()
                    .string(255)
                    .build(),
            ],
        )
        .unwrap();

    builder
        .insert_rows(
            "Property",
            vec![
                vec![Value::from("ProductName"), Value::from("Velocity Test")],
                vec![Value::from("ProductVersion"), Value::from("1.0.0")],
                vec![Value::from("Manufacturer"), Value::from("Velocity Corp")],
                vec![Value::from("ProductCode"), Value::from(product_code.as_str())],
                vec![Value::from("UpgradeCode"), Value::from(upgrade_code.as_str())],
                vec![Value::from("ProductLanguage"), Value::from("1033")],
            ],
        )
        .unwrap();

    // === Directory table ===
    builder
        .create_table(
            "Directory",
            vec![
                Column::build("Directory")
                    .string(72)
                    .primary_key()
                    .build(),
                Column::build("Directory_Parent")
                    .nullable()
                    .string(72)
                    .build(),
                Column::build("DefaultDir")
                    .nullable()
                    .localizable()
                    .string(255)
                    .build(),
            ],
        )
        .unwrap();

    builder
        .insert_rows(
            "Directory",
            vec![
                vec![
                    Value::from("TARGETDIR"),
                    Value::Null,
                    Value::from("SourceDir"),
                ],
                vec![
                    Value::from("ProgramFilesFolder"),
                    Value::from("TARGETDIR"),
                    Value::from("PFiles"),
                ],
                vec![
                    Value::from("INSTALLDIR"),
                    Value::from("ProgramFilesFolder"),
                    Value::from("VelocityTest"),
                ],
            ],
        )
        .unwrap();

    // === Component table ===
    builder
        .create_table(
            "Component",
            vec![
                Column::build("Component")
                    .string(72)
                    .primary_key()
                    .build(),
                Column::build("ComponentId")
                    .nullable()
                    .string(38)
                    .build(),
                Column::build("Directory_").string(72).build(),
                Column::build("Attributes").int16().build(),
                Column::build("Condition")
                    .nullable()
                    .string(255)
                    .build(),
                Column::build("KeyPath")
                    .nullable()
                    .string(72)
                    .build(),
            ],
        )
        .unwrap();

    builder
        .insert_rows(
            "Component",
            vec![vec![
                Value::from("MainComp"),
                Value::Null,
                Value::from("INSTALLDIR"),
                Value::from(0i32),
                Value::Null,
                Value::Null,
            ]],
        )
        .unwrap();

    // === Feature table ===
    builder
        .create_table(
            "Feature",
            vec![
                Column::build("Feature")
                    .string(38)
                    .primary_key()
                    .build(),
                Column::build("Feature_Parent")
                    .nullable()
                    .string(38)
                    .build(),
                Column::build("Title")
                    .nullable()
                    .localizable()
                    .string(64)
                    .build(),
                Column::build("Description")
                    .nullable()
                    .localizable()
                    .string(255)
                    .build(),
                Column::build("Display")
                    .nullable()
                    .int16()
                    .build(),
                Column::build("Level").int16().build(),
                Column::build("Directory_")
                    .nullable()
                    .string(72)
                    .build(),
                Column::build("Attributes").int16().build(),
            ],
        )
        .unwrap();

    builder
        .insert_rows(
            "Feature",
            vec![vec![
                Value::from("MainFeat"),
                Value::Null,
                Value::from("Complete"),
                Value::Null,
                Value::Null,
                Value::from(1i32),
                Value::Null,
                Value::from(0i32),
            ]],
        )
        .unwrap();

    // === FeatureComponents table ===
    builder
        .create_table(
            "FeatureComponents",
            vec![
                Column::build("Feature_")
                    .string(38)
                    .primary_key()
                    .build(),
                Column::build("Component_")
                    .string(72)
                    .primary_key()
                    .build(),
            ],
        )
        .unwrap();

    builder
        .insert_rows(
            "FeatureComponents",
            vec![vec![
                Value::from("MainFeat"),
                Value::from("MainComp"),
            ]],
        )
        .unwrap();

    // === InstallExecuteSequence table ===
    builder
        .create_table(
            "InstallExecuteSequence",
            vec![
                Column::build("Action")
                    .string(72)
                    .primary_key()
                    .build(),
                Column::build("Condition")
                    .nullable()
                    .string(255)
                    .build(),
                Column::build("Sequence")
                    .nullable()
                    .int16()
                    .build(),
            ],
        )
        .unwrap();

    builder
        .insert_rows(
            "InstallExecuteSequence",
            vec![
                vec![
                    Value::from("CostInitialize"),
                    Value::Null,
                    Value::from(800i32),
                ],
                vec![
                    Value::from("CostFinalize"),
                    Value::Null,
                    Value::from(1000i32),
                ],
                vec![
                    Value::from("InstallValidate"),
                    Value::Null,
                    Value::from(1400i32),
                ],
                vec![
                    Value::from("InstallInitialize"),
                    Value::Null,
                    Value::from(1500i32),
                ],
                vec![
                    Value::from("InstallFinalize"),
                    Value::Null,
                    Value::from(6600i32),
                ],
            ],
        )
        .unwrap();

    // === InstallUISequence table ===
    builder
        .create_table(
            "InstallUISequence",
            vec![
                Column::build("Action")
                    .string(72)
                    .primary_key()
                    .build(),
                Column::build("Condition")
                    .nullable()
                    .string(255)
                    .build(),
                Column::build("Sequence")
                    .nullable()
                    .int16()
                    .build(),
            ],
        )
        .unwrap();

    builder
        .insert_rows(
            "InstallUISequence",
            vec![
                vec![
                    Value::from("CostInitialize"),
                    Value::Null,
                    Value::from(800i32),
                ],
                vec![
                    Value::from("CostFinalize"),
                    Value::Null,
                    Value::from(1000i32),
                ],
                vec![
                    Value::from("ExecuteAction"),
                    Value::Null,
                    Value::from(1300i32),
                ],
            ],
        )
        .unwrap();

    // === Media table ===
    builder
        .create_table(
            "Media",
            vec![
                Column::build("DiskId").int16().primary_key().build(),
                Column::build("LastSequence").int16().build(),
                Column::build("DiskPrompt")
                    .nullable()
                    .localizable()
                    .string(64)
                    .build(),
                Column::build("Cabinet")
                    .nullable()
                    .string(255)
                    .build(),
                Column::build("VolumeLabel")
                    .nullable()
                    .localizable()
                    .string(32)
                    .build(),
                Column::build("Source")
                    .nullable()
                    .localizable()
                    .string(72)
                    .build(),
            ],
        )
        .unwrap();

    builder
        .insert_rows(
            "Media",
            vec![vec![
                Value::from(1i32),
                Value::from(0i32),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
            ]],
        )
        .unwrap();

    // Build the MSI
    let msi_data = builder.build().unwrap();
    std::fs::write(out_path, &msi_data).unwrap();
    println!("Wrote: {} ({} bytes)", out_path, msi_data.len());

    // Verify CFB version
    let ver = msi_data[26] as u16 + ((msi_data[27] as u16) << 8);
    let sector_pow = msi_data[30] as u16 + ((msi_data[31] as u16) << 8);
    println!(
        "CFB Version: {} (sector size: {})",
        ver,
        2u32.pow(sector_pow as u32)
    );

    // List streams
    println!("\n--- Streams ---");
    let mut comp = cfb::CompoundFile::open(std::io::Cursor::new(&msi_data)).unwrap();
    let stream_names: Vec<String> = comp
        .walk()
        .filter(|e| e.is_stream())
        .map(|e| e.name().to_string())
        .collect();
    for name in &stream_names {
        let stream = comp.open_stream(name).unwrap();
        println!("  {} ({} bytes)", name, stream.len());
    }

    // Test with msiexec
    println!("\n--- msiexec test ---");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", log_path])
        .output()
        .unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);
    match exit_code {
        0 => println!("SUCCESS! MSI installed!"),
        1603 => println!("1603 (fatal error during install)"),
        1613 => println!("1613 (invalid package - CFB version issue)"),
        1619 => println!("1619 (not valid package)"),
        1620 => println!("1620 (could not open - data issue)"),
        _ => println!("Error {}", exit_code),
    }

    // Read log for details
    if let Ok(log) = std::fs::read_to_string(log_path) {
        println!("\n--- Log highlights ---");
        for line in log.lines() {
            if line.contains("Error")
                || line.contains("successful")
                || line.contains("Installation")
                || line.contains("Product:")
                || line.contains("return value 3")
                || line.contains("2219")
                || line.contains("2203")
            {
                println!("  {}", line.trim());
            }
        }
    } else {
        println!("(no log file found)");
    }

    println!("\n=== DONE ===");
}
