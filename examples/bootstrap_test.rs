/// Bootstrap test: Create a V3 CFB with minimal system streams,
/// then open with msi crate to add all data and flush.
/// This proves whether the msi crate preserves V3 on flush.
///
/// cargo run --example bootstrap_test -p velocity-msi
use std::io::{Cursor, Write};

fn make_uuid() -> uuid::Uuid {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    uuid::Uuid::from_fields(
        (t & 0xFFFFFFFF) as u32,
        ((t >> 32) & 0xFFFF) as u16,
        (((t >> 48) & 0x0FFF) as u16) | 0x4000,
        &[0x80, 0, 0, 0, 0, 0, 0, 1],
    )
}

/// Encode a stream name using MSI's base-64 Unicode encoding.
fn encode_stream_name(name: &str, is_table: bool) -> String {
    let mut output = String::new();
    if is_table {
        output.push('\u{4840}');
    }
    let mut chars = name.chars().peekable();
    while let Some(ch1) = chars.next() {
        if let Some(value1) = ch_to_b64(ch1) {
            if let Some(&ch2) = chars.peek() {
                if let Some(value2) = ch_to_b64(ch2) {
                    let encoded = 0x3800 + (value2 << 6) + value1;
                    output.push(char::from_u32(encoded).unwrap());
                    chars.next();
                    continue;
                }
            }
            let encoded = 0x4800 + value1;
            output.push(char::from_u32(encoded).unwrap());
        } else {
            output.push(ch1);
        }
    }
    output
}

fn ch_to_b64(ch: char) -> Option<u32> {
    if ch.is_ascii_digit() {
        Some(ch as u32 - '0' as u32)
    } else if ch.is_ascii_uppercase() {
        Some(10 + ch as u32 - 'A' as u32)
    } else if ch.is_ascii_lowercase() {
        Some(36 + ch as u32 - 'a' as u32)
    } else if ch == '.' {
        Some(62)
    } else if ch == '_' {
        Some(63)
    } else {
        None
    }
}

fn main() {
    println!("=== BOOTSTRAP V3 MSI TEST ===\n");

    let out_path = "C:\\temp\\bootstrap_test.msi";
    let log_path = "C:\\temp\\bootstrap_test.log";
    let _ = std::fs::remove_file(out_path);
    let _ = std::fs::remove_file(log_path);

    // Step 1: Create a V3 CFB with minimal system streams
    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut comp =
            cfb::CompoundFile::create_with_version(cfb::Version::V3, cursor)
                .unwrap();

        // Set the MSI CLSID
        let msi_clsid = uuid::Uuid::from_bytes([
            0x00, 0x0C, 0x10, 0x84, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x46,
        ]);
        comp.set_storage_clsid("", msi_clsid).unwrap();

        // Write minimal SummaryInfo (just codepage)
        let summary = velocity_msi::SummaryInfo::new();
        let summary_data = summary.serialize().unwrap();
        let mut s = comp
            .create_stream("\u{0005}SummaryInformation")
            .unwrap();
        s.write_all(&summary_data).unwrap();

        // Write empty _Tables stream (0 bytes = no tables)
        let tables_name = encode_stream_name("_Tables", true);
        let mut s = comp.create_stream(&tables_name).unwrap();
        s.write_all(&[]).unwrap();

        // Write empty _Columns stream (0 bytes = no columns)
        let columns_name = encode_stream_name("_Columns", true);
        let mut s = comp.create_stream(&columns_name).unwrap();
        s.write_all(&[]).unwrap();

        // Write empty _StringPool (just header: codepage 1252)
        let pool_name = encode_stream_name("_StringPool", true);
        let mut s = comp.create_stream(&pool_name).unwrap();
        s.write_all(&1252u32.to_le_bytes()).unwrap();

        // Write empty _StringData (0 bytes)
        let data_name = encode_stream_name("_StringData", true);
        let mut s = comp.create_stream(&data_name).unwrap();
        s.write_all(&[]).unwrap();

        // Write empty _Validation stream (0 bytes)
        let validation_name = encode_stream_name("_Validation", true);
        let mut s = comp.create_stream(&validation_name).unwrap();
        s.write_all(&[]).unwrap();

        comp.flush().unwrap();
    }
    println!(
        "V3 CFB created: {} bytes, version {}",
        buf.len(),
        buf[26]
    );

    // Step 2: Open with msi crate and add all data
    let cursor = Cursor::new(buf);
    let mut pkg = match msi::Package::open(cursor) {
        Ok(p) => {
            println!("msi crate opened V3 CFB OK");
            p
        }
        Err(e) => {
            println!("msi crate open FAILED: {:?}", e);
            return;
        }
    };

    // Set SummaryInfo
    {
        let si = pkg.summary_info_mut();
        si.set_title("Installation Database");
        si.set_subject("Velocity Test Product");
        si.set_author("Velocity Corp");
        si.set_codepage(msi::CodePage::Windows1252);
        si.set_arch("Intel");
        si.set_languages(&[msi::Language::from_code(1033)]);
        si.set_word_count(2);
        si.set_uuid(make_uuid());
        si.set_creating_application("Velocity Installer");
    }
    pkg.set_database_codepage(msi::CodePage::Windows1252);
    println!("SummaryInfo set");

    // Create tables
    pkg.create_table(
        "Property",
        vec![
            msi::Column::build("Property").primary_key().id_string(72),
            msi::Column::build("Value")
                .nullable()
                .localizable()
                .formatted_string(255),
        ],
    )
    .unwrap();

    pkg.create_table(
        "Directory",
        vec![
            msi::Column::build("Directory").primary_key().id_string(72),
            msi::Column::build("Directory_Parent")
                .nullable()
                .string(72),
            msi::Column::build("DefaultDir")
                .nullable()
                .localizable()
                .formatted_string(255),
        ],
    )
    .unwrap();

    pkg.create_table(
        "Component",
        vec![
            msi::Column::build("Component").primary_key().id_string(72),
            msi::Column::build("ComponentId").nullable().string(38),
            msi::Column::build("Directory_").id_string(72),
            msi::Column::build("Attributes").int16(),
            msi::Column::build("Condition").nullable().formatted_string(255),
            msi::Column::build("KeyPath").nullable().id_string(72),
        ],
    )
    .unwrap();

    pkg.create_table(
        "Feature",
        vec![
            msi::Column::build("Feature").primary_key().id_string(38),
            msi::Column::build("Feature_Parent").nullable().id_string(38),
            msi::Column::build("Title")
                .nullable()
                .localizable()
                .formatted_string(64),
            msi::Column::build("Description")
                .nullable()
                .localizable()
                .formatted_string(255),
            msi::Column::build("Display").nullable().int16(),
            msi::Column::build("Level").int16(),
            msi::Column::build("Directory_").nullable().id_string(72),
            msi::Column::build("Attributes").int16(),
        ],
    )
    .unwrap();

    pkg.create_table(
        "FeatureComponents",
        vec![
            msi::Column::build("Feature_").primary_key().id_string(38),
            msi::Column::build("Component_").primary_key().id_string(72),
        ],
    )
    .unwrap();

    pkg.create_table(
        "InstallExecuteSequence",
        vec![
            msi::Column::build("Action").primary_key().id_string(72),
            msi::Column::build("Condition").nullable().formatted_string(255),
            msi::Column::build("Sequence").nullable().int16(),
        ],
    )
    .unwrap();

    pkg.create_table(
        "InstallUISequence",
        vec![
            msi::Column::build("Action").primary_key().id_string(72),
            msi::Column::build("Condition").nullable().formatted_string(255),
            msi::Column::build("Sequence").nullable().int16(),
        ],
    )
    .unwrap();

    pkg.create_table(
        "Media",
        vec![
            msi::Column::build("DiskId").primary_key().int16(),
            msi::Column::build("LastSequence").int16(),
            msi::Column::build("DiskPrompt")
                .nullable()
                .localizable()
                .formatted_string(64),
            msi::Column::build("Cabinet").nullable().string(255),
            msi::Column::build("VolumeLabel")
                .nullable()
                .localizable()
                .id_string(32),
            msi::Column::build("Source")
                .nullable()
                .localizable()
                .id_string(72),
        ],
    )
    .unwrap();

    println!("Tables created");

    // Insert data
    let pc = format!("{{{}}}", make_uuid().hyphenated()).to_uppercase();
    let uc = format!("{{{}}}", make_uuid().hyphenated()).to_uppercase();

    pkg.insert_rows(
        msi::Insert::into("Property")
            .row(vec![
                msi::Value::Str("ProductName".into()),
                msi::Value::Str("Velocity Test".into()),
            ])
            .row(vec![
                msi::Value::Str("ProductVersion".into()),
                msi::Value::Str("1.0.0".into()),
            ])
            .row(vec![
                msi::Value::Str("Manufacturer".into()),
                msi::Value::Str("Velocity Corp".into()),
            ])
            .row(vec![
                msi::Value::Str("ProductCode".into()),
                msi::Value::Str(pc),
            ])
            .row(vec![
                msi::Value::Str("UpgradeCode".into()),
                msi::Value::Str(uc),
            ])
            .row(vec![
                msi::Value::Str("ProductLanguage".into()),
                msi::Value::Str("1033".into()),
            ]),
    )
    .unwrap();

    pkg.insert_rows(
        msi::Insert::into("Directory")
            .row(vec![
                msi::Value::Str("TARGETDIR".into()),
                msi::Value::Null,
                msi::Value::Str("SourceDir".into()),
            ])
            .row(vec![
                msi::Value::Str("ProgramFilesFolder".into()),
                msi::Value::Str("TARGETDIR".into()),
                msi::Value::Str("PFiles".into()),
            ])
            .row(vec![
                msi::Value::Str("INSTALLDIR".into()),
                msi::Value::Str("ProgramFilesFolder".into()),
                msi::Value::Str("VelocityTest".into()),
            ]),
    )
    .unwrap();

    pkg.insert_rows(
        msi::Insert::into("Component").row(vec![
            msi::Value::Str("MainComp".into()),
            msi::Value::Null,
            msi::Value::Str("INSTALLDIR".into()),
            msi::Value::Int(0),
            msi::Value::Null,
            msi::Value::Null,
        ]),
    )
    .unwrap();

    pkg.insert_rows(
        msi::Insert::into("Feature").row(vec![
            msi::Value::Str("MainFeat".into()),
            msi::Value::Null,
            msi::Value::Str("Complete".into()),
            msi::Value::Null,
            msi::Value::Null,
            msi::Value::Int(1),
            msi::Value::Null,
            msi::Value::Int(0),
        ]),
    )
    .unwrap();

    pkg.insert_rows(
        msi::Insert::into("FeatureComponents").row(vec![
            msi::Value::Str("MainFeat".into()),
            msi::Value::Str("MainComp".into()),
        ]),
    )
    .unwrap();

    pkg.insert_rows(
        msi::Insert::into("InstallExecuteSequence")
            .row(vec![
                msi::Value::Str("CostInitialize".into()),
                msi::Value::Null,
                msi::Value::Int(800),
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
                msi::Value::Str("InstallFinalize".into()),
                msi::Value::Null,
                msi::Value::Int(6600),
            ]),
    )
    .unwrap();

    pkg.insert_rows(
        msi::Insert::into("InstallUISequence")
            .row(vec![
                msi::Value::Str("CostInitialize".into()),
                msi::Value::Null,
                msi::Value::Int(800),
            ])
            .row(vec![
                msi::Value::Str("CostFinalize".into()),
                msi::Value::Null,
                msi::Value::Int(1000),
            ])
            .row(vec![
                msi::Value::Str("ExecuteAction".into()),
                msi::Value::Null,
                msi::Value::Int(1300),
            ]),
    )
    .unwrap();

    pkg.insert_rows(
        msi::Insert::into("Media").row(vec![
            msi::Value::Int(1),
            msi::Value::Int(0),
            msi::Value::Null,
            msi::Value::Null,
            msi::Value::Null,
            msi::Value::Null,
        ]),
    )
    .unwrap();

    println!("Data inserted");

    // Flush
    match pkg.flush() {
        Ok(_) => println!("Flush OK"),
        Err(e) => {
            println!("Flush FAILED: {:?}", e);
            return;
        }
    }

    let cursor = pkg.into_inner().unwrap();
    let msi_data = cursor.into_inner();
    std::fs::write(out_path, &msi_data).unwrap();
    println!("Wrote: {} ({} bytes)", out_path, msi_data.len());

    // Verify V3
    println!(
        "Output CFB Version: {} (sector: {})",
        msi_data[26] as u16 + ((msi_data[27] as u16) << 8),
        2u32.pow(
            (msi_data[30] as u16 + ((msi_data[31] as u16) << 8)) as u32
        )
    );

    // Test with msiexec
    println!("\n--- msiexec test ---");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", log_path])
        .output()
        .unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);
    match exit_code {
        0 => println!("SUCCESS!"),
        1603 => println!("1603 (fatal error during install)"),
        1613 => println!("1613 (invalid package)"),
        1619 => println!("1619 (not valid)"),
        1620 => println!("1620 (could not open)"),
        _ => println!("Error {}", exit_code),
    }

    if let Ok(log) = std::fs::read_to_string(log_path) {
        for line in log.lines() {
            if line.contains("Error")
                || line.contains("successful")
                || line.contains("Installation")
                || line.contains("Product:")
                || line.contains("return value 3")
            {
                println!("  {}", line.trim());
            }
        }
    } else {
        println!("(no log)");
    }

    println!("\n=== DONE ===");
}
