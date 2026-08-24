/// Test: open our velocity-msi output with the msi crate, add InstallExecuteSequence,
/// and test with msiexec. This isolates whether the bug is in our table serialization
/// or in the msi crate's write path.
///
/// Test 1: Open + flush only (no changes) → should still work
/// Test 2: Open + add InstallExecuteSequence → does it work?
/// Test 3: If test 2 works, binary-compare the modified MSI with our full MSI
use std::fs;
use std::process::Command;
use velocity_msi::{Column, MsiBuilder, Value};

fn build_base_msi() -> Vec<u8> {
    let mut builder = MsiBuilder::new();
    builder.set_title("Open Modify Test");
    builder.set_author("Velocity");
    builder.set_template("Intel", 1033);

    builder
        .create_table(
            "Property",
            vec![
                Column::build("Property").string(72).primary_key().build(),
                Column::build("Value").string(1024).build(),
            ],
        )
        .unwrap();
    builder
        .insert_rows(
            "Property",
            vec![
                vec![Value::from("ProductName"), Value::from("Open Modify Test")],
                vec![Value::from("ProductVersion"), Value::from("1.0.0")],
                vec![Value::from("Manufacturer"), Value::from("Velocity Corp")],
                vec![
                    Value::from("ProductCode"),
                    Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}"),
                ],
                vec![
                    Value::from("UpgradeCode"),
                    Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}"),
                ],
                vec![Value::from("ProductLanguage"), Value::from("1033")],
            ],
        )
        .unwrap();

    builder
        .create_table(
            "Directory",
            vec![
                Column::build("Directory").string(72).primary_key().build(),
                Column::build("Directory_Parent")
                    .string(72)
                    .nullable()
                    .build(),
                Column::build("DefaultDir").string(255).primary_key().build(),
            ],
        )
        .unwrap();
    builder
        .insert_rows(
            "Directory",
            vec![
                vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
                vec![
                    Value::from("INSTALLDIR"),
                    Value::from("TARGETDIR"),
                    Value::from("VelTest:VelTest"),
                ],
            ],
        )
        .unwrap();

    builder.build().unwrap()
}

fn test_msi(path: &str, label: &str) -> i32 {
    let _ = fs::remove_dir_all("C:\\VelTest");
    let log = format!("{}.log", label);
    let output = Command::new("msiexec")
        .args(&["/i", path, "/qn", "/l*v", &log])
        .output()
        .unwrap();
    let exit = output.status.code().unwrap_or(-1);
    println!("  {} → exit={}", label, exit);
    if exit != 0 {
        // Print relevant error lines from log
        if let Ok(log_content) = fs::read_to_string(&log) {
            for line in log_content.lines() {
                if (line.contains("Error ") && !line.contains("Error 0")
                    && !line.contains("2205")
                    && !line.contains("2228"))
                    || line.contains("return value 3")
                {
                    println!("    {}", line.trim());
                }
            }
        }
    }
    exit
}

fn main() {
    println!("=== MSI Open/Modify Test ===\n");

    // Build our base MSI (Property + Directory, known to work)
    let msi_data = build_base_msi();
    let base_path = "open_base.msi";
    fs::write(base_path, &msi_data).unwrap();
    println!("Base MSI: {} ({} bytes)", base_path, msi_data.len());

    // Test 0: Base MSI should work
    println!("\n--- Test 0: Base MSI (velocity-msi) ---");
    let exit0 = test_msi(base_path, "open_base");
    assert_eq!(exit0, 0, "Base MSI should work!");

    // Test 1: Open with msi crate, flush without changes
    println!("\n--- Test 1: msi crate open + flush (no changes) ---");
    {
        let flush_path = "open_flush.msi";
        fs::copy(base_path, flush_path).unwrap();

        let mut pkg = msi::open_rw(flush_path).expect("open_rw failed");
        pkg.flush().expect("flush failed");
        drop(pkg);

        let exit1 = test_msi(flush_path, "open_flush");
        if exit1 != 0 {
            println!("  *** msi crate flush BROKE the MSI! ***");
            println!("  This means the msi crate's write path has issues too.");
        } else {
            println!("  msi crate flush preserves validity.");
        }
    }

    // Test 2: Open with msi crate, add InstallExecuteSequence, flush
    println!("\n--- Test 2: msi crate open + add InstallExecuteSequence ---");
    {
        let seq_path = "open_seq.msi";
        fs::copy(base_path, seq_path).unwrap();

        let mut pkg = msi::open_rw(seq_path).expect("open_rw failed");

        // Create InstallExecuteSequence table (using msi crate's Column type)
        let columns = vec![
            msi::Column::build("Action").primary_key().string(72),
            msi::Column::build("Condition").nullable().string(255),
            msi::Column::build("Sequence").nullable().int16(),
        ];
        pkg.create_table("InstallExecuteSequence", columns)
            .expect("create_table failed");

        // Insert standard sequence actions
        pkg.insert_rows(
            msi::Insert::into("InstallExecuteSequence")
                .row(vec![
                    msi::Value::Str("CostInitialize".into()),
                    msi::Value::Null,
                    msi::Value::Int(800),
                ])
                .row(vec![
                    msi::Value::Str("FileCost".into()),
                    msi::Value::Null,
                    msi::Value::Int(900),
                ])
                .row(vec![
                    msi::Value::Str("CostFinalize".into()),
                    msi::Value::Null,
                    msi::Value::Int(1000),
                ])
                .row(vec![
                    msi::Value::Str("InstallValidate".into()),
                    msi::Value::Null,
                    msi::Value::Int(1400),
                ])
                .row(vec![
                    msi::Value::Str("InstallInitialize".into()),
                    msi::Value::Null,
                    msi::Value::Int(1500),
                ])
                .row(vec![
                    msi::Value::Str("InstallFiles".into()),
                    msi::Value::Null,
                    msi::Value::Int(4000),
                ])
                .row(vec![
                    msi::Value::Str("InstallFinalize".into()),
                    msi::Value::Null,
                    msi::Value::Int(6600),
                ]),
        )
        .expect("insert_rows failed");

        pkg.flush().expect("flush failed");
        drop(pkg);

        let exit2 = test_msi(seq_path, "open_seq");
        if exit2 == 0 {
            println!("  *** SUCCESS! msi crate can add InstallExecuteSequence! ***");
            println!("  This proves our table DATA is the issue, not the OLE layer.");
        } else {
            println!("  msi crate also fails with InstallExecuteSequence.");
            println!("  This might mean the msi crate's write path has issues.");
        }
    }

    // Test 3: Binary compare - if test 2 worked, compare the MSI structures
    println!("\n--- Test 3: Binary comparison ---");
    {
        let base_data = fs::read(base_path).unwrap();
        let seq_path = "open_seq.msi";
        if std::path::Path::new(seq_path).exists() {
            let seq_data = fs::read(seq_path).unwrap();
            println!("  Base MSI: {} bytes", base_data.len());
            println!("  Seq MSI:  {} bytes", seq_data.len());

            // Compare using cfb to list streams
            let base_cursor = std::io::Cursor::new(&base_data);
            let seq_cursor = std::io::Cursor::new(&seq_data);

            if let (Ok(base_cfb), Ok(seq_cfb)) = (
                cfb::CompoundFile::open(base_cursor),
                cfb::CompoundFile::open(seq_cursor),
            ) {
                println!("\n  Base streams:");
                for entry in base_cfb.walk() {
                    if entry.is_stream() {
                        println!(
                            "    {} ({} bytes)",
                            entry.name(),
                            entry.len()
                        );
                    }
                }
                println!("\n  Seq streams:");
                for entry in seq_cfb.walk() {
                    if entry.is_stream() {
                        println!(
                            "    {} ({} bytes)",
                            entry.name(),
                            entry.len()
                        );
                    }
                }
            }
        }
    }

    println!("\nDone.");
}
