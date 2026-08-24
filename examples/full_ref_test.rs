/// Create a complete MSI with msi crate (including ProductCode),
/// convert to V3, and test with msiexec.
/// cargo run --example full_ref_test -p velocity-msi
use std::io::{Cursor, Read, Write};

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

fn main() {
    println!("=== FULL REFERENCE MSI TEST ===\n");

    let product_code = format!("{{{}}}", make_uuid().hyphenated()).to_uppercase();
    let upgrade_code = format!("{{{}}}", make_uuid().hyphenated()).to_uppercase();
    println!("ProductCode: {}", product_code);
    println!("UpgradeCode: {}", upgrade_code);

    // Step 1: Create MSI with msi crate (V4)
    let cursor = Cursor::new(Vec::new());
    let mut pkg = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();

    {
        let si = pkg.summary_info_mut();
        si.set_title("Installation Database");
        si.set_subject("Full Ref Product");
        si.set_author("Test Corp");
        si.set_codepage(msi::CodePage::Windows1252);
        si.set_arch("Intel");
        si.set_languages(&[msi::Language::from_code(1033)]);
        si.set_word_count(2);
        si.set_uuid(make_uuid());
        si.set_creating_application("Test App");
    }
    pkg.set_database_codepage(msi::CodePage::Windows1252);

    // Create ALL tables
    pkg.create_table("Property", vec![
        msi::Column::build("Property").primary_key().id_string(72),
        msi::Column::build("Value").nullable().localizable().formatted_string(255),
    ]).unwrap();
    pkg.create_table("Directory", vec![
        msi::Column::build("Directory").primary_key().id_string(72),
        msi::Column::build("Directory_Parent").nullable().string(72),
        msi::Column::build("DefaultDir").nullable().localizable().formatted_string(255),
    ]).unwrap();
    pkg.create_table("Component", vec![
        msi::Column::build("Component").primary_key().id_string(72),
        msi::Column::build("ComponentId").nullable().string(38),
        msi::Column::build("Directory_").id_string(72),
        msi::Column::build("Attributes").int16(),
        msi::Column::build("Condition").nullable().formatted_string(255),
        msi::Column::build("KeyPath").nullable().id_string(72),
    ]).unwrap();
    pkg.create_table("Feature", vec![
        msi::Column::build("Feature").primary_key().id_string(38),
        msi::Column::build("Feature_Parent").nullable().id_string(38),
        msi::Column::build("Title").nullable().localizable().formatted_string(64),
        msi::Column::build("Description").nullable().localizable().formatted_string(255),
        msi::Column::build("Display").nullable().int16(),
        msi::Column::build("Level").int16(),
        msi::Column::build("Directory_").nullable().id_string(72),
        msi::Column::build("Attributes").int16(),
    ]).unwrap();
    pkg.create_table("FeatureComponents", vec![
        msi::Column::build("Feature_").primary_key().id_string(38),
        msi::Column::build("Component_").primary_key().id_string(72),
    ]).unwrap();
    pkg.create_table("InstallExecuteSequence", vec![
        msi::Column::build("Action").primary_key().id_string(72),
        msi::Column::build("Condition").nullable().formatted_string(255),
        msi::Column::build("Sequence").nullable().int16(),
    ]).unwrap();
    pkg.create_table("InstallUISequence", vec![
        msi::Column::build("Action").primary_key().id_string(72),
        msi::Column::build("Condition").nullable().formatted_string(255),
        msi::Column::build("Sequence").nullable().int16(),
    ]).unwrap();
    pkg.create_table("Media", vec![
        msi::Column::build("DiskId").primary_key().int16(),
        msi::Column::build("LastSequence").int16(),
        msi::Column::build("DiskPrompt").nullable().localizable().formatted_string(64),
        msi::Column::build("Cabinet").nullable().string(255),
        msi::Column::build("VolumeLabel").nullable().localizable().id_string(32),
        msi::Column::build("Source").nullable().localizable().id_string(72),
    ]).unwrap();

    // Insert ALL data INCLUDING ProductCode and UpgradeCode
    pkg.insert_rows(msi::Insert::into("Property")
        .row(vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Full Ref Test".into())])
        .row(vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())])
        .row(vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Test Corp".into())])
        .row(vec![msi::Value::Str("ProductCode".into()), msi::Value::Str(product_code)])
        .row(vec![msi::Value::Str("UpgradeCode".into()), msi::Value::Str(upgrade_code)])
        .row(vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())])
    ).unwrap();

    pkg.insert_rows(msi::Insert::into("Directory")
        .row(vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str("SourceDir".into())])
        .row(vec![msi::Value::Str("ProgramFilesFolder".into()), msi::Value::Str("TARGETDIR".into()), msi::Value::Str("PFiles".into())])
        .row(vec![msi::Value::Str("INSTALLDIR".into()), msi::Value::Str("ProgramFilesFolder".into()), msi::Value::Str("FullRefTest".into())])
    ).unwrap();

    pkg.insert_rows(msi::Insert::into("Component").row(vec![
        msi::Value::Str("MainComp".into()), msi::Value::Null,
        msi::Value::Str("INSTALLDIR".into()), msi::Value::Int(0),
        msi::Value::Null, msi::Value::Null,
    ])).unwrap();

    pkg.insert_rows(msi::Insert::into("Feature").row(vec![
        msi::Value::Str("MainFeat".into()), msi::Value::Null,
        msi::Value::Str("Complete".into()), msi::Value::Null,
        msi::Value::Null, msi::Value::Int(1),
        msi::Value::Null, msi::Value::Int(0),
    ])).unwrap();

    pkg.insert_rows(msi::Insert::into("FeatureComponents").row(vec![
        msi::Value::Str("MainFeat".into()), msi::Value::Str("MainComp".into()),
    ])).unwrap();

    pkg.insert_rows(msi::Insert::into("InstallExecuteSequence")
        .row(vec![msi::Value::Str("CostInitialize".into()), msi::Value::Null, msi::Value::Int(800)])
        .row(vec![msi::Value::Str("CostFinalize".into()), msi::Value::Null, msi::Value::Int(1000)])
        .row(vec![msi::Value::Str("InstallValidate".into()), msi::Value::Null, msi::Value::Int(1400)])
        .row(vec![msi::Value::Str("InstallInitialize".into()), msi::Value::Null, msi::Value::Int(1500)])
        .row(vec![msi::Value::Str("InstallFinalize".into()), msi::Value::Null, msi::Value::Int(6600)])
    ).unwrap();

    pkg.insert_rows(msi::Insert::into("InstallUISequence")
        .row(vec![msi::Value::Str("CostInitialize".into()), msi::Value::Null, msi::Value::Int(800)])
        .row(vec![msi::Value::Str("CostFinalize".into()), msi::Value::Null, msi::Value::Int(1000)])
        .row(vec![msi::Value::Str("ExecuteAction".into()), msi::Value::Null, msi::Value::Int(1300)])
    ).unwrap();

    pkg.insert_rows(msi::Insert::into("Media").row(vec![
        msi::Value::Int(1), msi::Value::Int(0),
        msi::Value::Null, msi::Value::Null, msi::Value::Null, msi::Value::Null,
    ])).unwrap();

    pkg.flush().unwrap();
    let cursor = pkg.into_inner().unwrap();
    let v4_data = cursor.into_inner();
    println!("V4 MSI: {} bytes (V{})", v4_data.len(), v4_data[26]);

    // Step 2: Convert V4 to V3
    let v3_path = "C:\\temp\\full_ref_v3.msi";
    let log_path = "C:\\temp\\full_ref_v3.log";
    let _ = std::fs::remove_file(v3_path);
    let _ = std::fs::remove_file(log_path);

    {
        let mut src = cfb::CompoundFile::open(Cursor::new(&v4_data)).unwrap();
        let stream_entries: Vec<(String, Vec<u8>)> = src.walk()
            .filter(|e| e.is_stream())
            .map(|e| e.name().to_string())
            .collect::<Vec<_>>()
            .into_iter()
            .map(|name| {
                let mut stream = src.open_stream(&name).unwrap();
                let mut data = Vec::new();
                stream.read_to_end(&mut data).unwrap();
                (name, data)
            })
            .collect();

        let root_clsid = {
            let re = src.root_entry();
            *re.clsid()
        };

        let mut v3_buf = Vec::new();
        {
            let cursor = Cursor::new(&mut v3_buf);
            let mut v3 = cfb::CompoundFile::create_with_version(
                cfb::Version::V3, cursor
            ).unwrap();
            v3.set_storage_clsid("", root_clsid).unwrap();
            for (name, data) in &stream_entries {
                let mut s = v3.create_stream(name).unwrap();
                s.write_all(data).unwrap();
            }
            v3.flush().unwrap();
        }
        std::fs::write(v3_path, &v3_buf).unwrap();
        println!("V3 MSI: {} bytes (V{})", v3_buf.len(), v3_buf[26]);
    }

    // Step 3: Test V3 with msiexec
    println!("\n--- msiexec test: V3 with ProductCode ---");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", v3_path, "/qn", "/l*v", log_path])
        .output().unwrap();
    let ec = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", ec);
    match ec {
        0 => println!("SUCCESS! MSI installed!"),
        1603 => println!("1603 (fatal error during install)"),
        1613 => println!("1613 (invalid package)"),
        1619 => println!("1619 (not valid)"),
        1620 => println!("1620 (could not open - data issue)"),
        _ => println!("Error {}", ec),
    }

    // Read log
    if let Ok(log) = std::fs::read_to_string(log_path) {
        println!("\n--- Log highlights ---");
        for line in log.lines() {
            if line.contains("Error") || line.contains("successful")
                || line.contains("Installation") || line.contains("Product:")
                || line.contains("return value 3") || line.contains("2219")
                || line.contains("2203") || line.contains("MSI_")
            {
                println!("  {}", line.trim());
            }
        }
    } else {
        println!("(no log)");
    }

    // Step 4: Also test V4 directly (expect 1613)
    let v4_path = "C:\\temp\\full_ref_v4.msi";
    std::fs::write(v4_path, &v4_data).unwrap();
    println!("\n--- msiexec test: V4 directly ---");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", v4_path, "/qn"])
        .output().unwrap();
    println!("Exit code: {}", output.status.code().unwrap_or(-1));

    println!("\n=== DONE ===");
}
