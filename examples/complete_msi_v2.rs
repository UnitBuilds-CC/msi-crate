/// Create complete MSI from scratch using msi crate with ALL required tables
/// cargo run --example complete_msi_v2 -p velocity-msi
use std::io::Cursor;

fn make_uuid() -> uuid::Uuid {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let a = (t & 0xFFFFFFFF) as u32;
    let b = ((t >> 32) & 0xFFFF) as u16;
    let c = (((t >> 48) & 0x0FFF) as u16) | 0x4000;
    uuid::Uuid::from_fields(a, b, c, &[0x80, 0, 0, 0, 0, 0, 0, 1])
}

fn main() {
    println!("=== COMPLETE MSI V2 TEST ===\n");

    let out_path = "C:\\temp\\complete_v2.msi";
    let _ = std::fs::remove_file(out_path);
    let _ = std::fs::remove_file("C:\\temp\\complete_v2.log");

    let cursor = Cursor::new(Vec::new());
    let mut pkg = match msi::Package::create(msi::PackageType::Installer, cursor) {
        Ok(p) => p,
        Err(e) => { println!("Create failed: {:?}", e); return; }
    };
    println!("Package created");

    // Set SummaryInfo
    {
        let si = pkg.summary_info_mut();
        si.set_title("Velocity Test Installation");
        si.set_subject("Velocity Test");
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

    // Create ALL required tables
    // Property
    pkg.create_table("Property", vec![
        msi::Column::build("Property").primary_key().string(72),
        msi::Column::build("Value").nullable().localizable().string(255),
    ]).unwrap();

    // Directory
    pkg.create_table("Directory", vec![
        msi::Column::build("Directory").primary_key().string(72),
        msi::Column::build("Directory_Parent").nullable().string(72),
        msi::Column::build("DefaultDir").nullable().localizable().string(255),
    ]).unwrap();

    // Component
    pkg.create_table("Component", vec![
        msi::Column::build("Component").primary_key().string(72),
        msi::Column::build("ComponentId").nullable().string(38),
        msi::Column::build("Directory_").string(72),
        msi::Column::build("Attributes").int16(),
        msi::Column::build("Condition").nullable().string(255),
        msi::Column::build("KeyPath").nullable().string(72),
    ]).unwrap();

    // Feature
    pkg.create_table("Feature", vec![
        msi::Column::build("Feature").primary_key().string(38),
        msi::Column::build("Feature_Parent").nullable().string(38),
        msi::Column::build("Title").nullable().localizable().string(64),
        msi::Column::build("Description").nullable().localizable().string(255),
        msi::Column::build("Display").nullable().int16(),
        msi::Column::build("Level").int16(),
        msi::Column::build("Directory_").nullable().string(72),
        msi::Column::build("Attributes").int16(),
    ]).unwrap();

    // FeatureComponents
    pkg.create_table("FeatureComponents", vec![
        msi::Column::build("Feature_").primary_key().string(38),
        msi::Column::build("Component_").primary_key().string(72),
    ]).unwrap();

    // InstallExecuteSequence
    pkg.create_table("InstallExecuteSequence", vec![
        msi::Column::build("Action").primary_key().string(72),
        msi::Column::build("Condition").nullable().string(255),
        msi::Column::build("Sequence").nullable().int16(),
    ]).unwrap();

    // InstallUISequence
    pkg.create_table("InstallUISequence", vec![
        msi::Column::build("Action").primary_key().string(72),
        msi::Column::build("Condition").nullable().string(255),
        msi::Column::build("Sequence").nullable().int16(),
    ]).unwrap();

    // Media
    pkg.create_table("Media", vec![
        msi::Column::build("DiskId").primary_key().int16(),
        msi::Column::build("LastSequence").int32(),
        msi::Column::build("Cabinet").nullable().string(255),
        msi::Column::build("VolumeLabel").nullable().string(32),
        msi::Column::build("Source").nullable().string(72),
    ]).unwrap();

    println!("All tables created");

    // Insert data
    let pc = format!("{{{}}}", make_uuid().hyphenated()).to_uppercase();
    let uc = format!("{{{}}}", make_uuid().hyphenated()).to_uppercase();

    pkg.insert_rows(msi::Insert::into("Property")
        .row(vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Velocity Test".into())])
        .row(vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())])
        .row(vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Velocity Corp".into())])
        .row(vec![msi::Value::Str("ProductCode".into()), msi::Value::Str(pc)])
        .row(vec![msi::Value::Str("UpgradeCode".into()), msi::Value::Str(uc)])
        .row(vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())])
    ).unwrap();

    pkg.insert_rows(msi::Insert::into("Directory")
        .row(vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str("SourceDir".into())])
        .row(vec![msi::Value::Str("ProgramFilesFolder".into()), msi::Value::Str("TARGETDIR".into()), msi::Value::Str("PFiles".into())])
        .row(vec![msi::Value::Str("INSTALLDIR".into()), msi::Value::Str("ProgramFilesFolder".into()), msi::Value::Str("VelocityTest".into())])
    ).unwrap();

    pkg.insert_rows(msi::Insert::into("Component")
        .row(vec![msi::Value::Str("MainComp".into()), msi::Value::Null, msi::Value::Str("INSTALLDIR".into()), msi::Value::Int(0), msi::Value::Null, msi::Value::Null])
    ).unwrap();

    pkg.insert_rows(msi::Insert::into("Feature")
        .row(vec![msi::Value::Str("MainFeat".into()), msi::Value::Null, msi::Value::Str("Complete".into()), msi::Value::Null, msi::Value::Null, msi::Value::Int(1), msi::Value::Null, msi::Value::Int(0)])
    ).unwrap();

    pkg.insert_rows(msi::Insert::into("FeatureComponents")
        .row(vec![msi::Value::Str("MainFeat".into()), msi::Value::Str("MainComp".into())])
    ).unwrap();

    pkg.insert_rows(msi::Insert::into("InstallExecuteSequence")
        .row(vec![msi::Value::Str("CostInitialize".into()), msi::Value::Null, msi::Value::Int(800)])
        .row(vec![msi::Value::Str("CostFinalize".into()), msi::Value::Null, msi::Value::Int(1000)])
        .row(vec![msi::Value::Str("InstallValidate".into()), msi::Value::Null, msi::Value::Int(1400)])
        .row(vec![msi::Value::Str("InstallInitialize".into()), msi::Value::Null, msi::Value::Int(1500)])
        .row(vec![msi::Value::Str("InstallFiles".into()), msi::Value::Null, msi::Value::Int(4000)])
        .row(vec![msi::Value::Str("InstallFinalize".into()), msi::Value::Null, msi::Value::Int(6600)])
    ).unwrap();

    pkg.insert_rows(msi::Insert::into("InstallUISequence")
        .row(vec![msi::Value::Str("CostInitialize".into()), msi::Value::Null, msi::Value::Int(800)])
        .row(vec![msi::Value::Str("CostFinalize".into()), msi::Value::Null, msi::Value::Int(1000)])
        .row(vec![msi::Value::Str("ExecuteAction".into()), msi::Value::Null, msi::Value::Int(1300)])
    ).unwrap();

    println!("All data inserted");

    // Flush
    match pkg.flush() {
        Ok(_) => println!("Flush OK"),
        Err(e) => { println!("Flush failed: {:?}", e); return; }
    }

    let cursor = pkg.into_inner().unwrap();
    let msi_data = cursor.into_inner();
    std::fs::write(out_path, &msi_data).unwrap();
    println!("Wrote: {} ({} bytes)", out_path, msi_data.len());

    // Test with msiexec
    println!("\n--- msiexec test ---");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", "C:\\temp\\complete_v2.log"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);
    match exit_code {
        0 => println!("SUCCESS!"),
        1603 => println!("1603 (accepted but install error)"),
        1613 => println!("1613 (invalid package)"),
        1619 => println!("1619 (not valid)"),
        1620 => println!("1620 (could not open)"),
        _ => println!("Error {}", exit_code),
    }

    if let Ok(log) = std::fs::read_to_string("C:\\temp\\complete_v2.log") {
        for line in log.lines() {
            if line.contains("Error") || line.contains("2219") || line.contains("2203") ||
               line.contains("Product:") || line.contains("successful") || line.contains("Installation") {
                println!("  {}", line.trim());
            }
        }
    }

    println!("\n=== DONE ===");
}
