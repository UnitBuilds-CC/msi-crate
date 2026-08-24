/// Create MSI using msi crate API but with V3 CFB container
/// Strategy: Create V3 CFB -> write empty SummaryInfo -> open with msi crate -> add data -> flush
/// cargo run --example v3_msi_build -p velocity-msi
use std::io::{Cursor, Write};

fn make_uuid() -> uuid::Uuid {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let a = (t & 0xFFFFFFFF) as u32;
    let b = ((t >> 32) & 0xFFFF) as u16;
    let c = (((t >> 48) & 0x0FFF) as u16) | 0x4000;
    uuid::Uuid::from_fields(a, b, c, &[0x80, 0, 0, 0, 0, 0, 0, 1])
}

fn main() {
    println!("=== V3 MSI BUILD TEST ===\n");

    let out_path = "C:\\temp\\v3_msi.msi";
    let _ = std::fs::remove_file(out_path);
    let _ = std::fs::remove_file("C:\\temp\\v3_msi.log");

    // Step 1: Create a V3 CFB with minimal SummaryInfo stream
    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut comp = cfb::CompoundFile::create_with_version(cfb::Version::V3, cursor).unwrap();

        // Set MSI CLSID
        let msi_clsid = uuid::Uuid::from_bytes([
            0x00, 0x0C, 0x10, 0x84, 0x00, 0x00, 0x00, 0x00,
            0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
        ]);
        comp.set_storage_clsid("", msi_clsid).unwrap();

        // Write a minimal SummaryInfo stream
        // We need at least a valid property set with codepage
        let summary = velocity_msi::SummaryInfo::new();
        let summary_data = summary.serialize().unwrap();
        let mut s = comp.create_stream("\u{0005}SummaryInformation").unwrap();
        s.write_all(&summary_data).unwrap();

        comp.flush().unwrap();
    }
    println!("V3 CFB created: {} bytes", buf.len());

    // Verify V3
    println!("CFB Version: {} (sector: {})", 
        buf[26] as u16 + (buf[27] as u16) * 256,
        2u32.pow((buf[30] as u16 + (buf[31] as u16) * 256) as u32));

    // Step 2: Open with msi crate and add all data
    let cursor = Cursor::new(buf);
    let mut pkg = match msi::Package::open(cursor) {
        Ok(p) => { println!("msi crate opened V3 CFB OK"); p }
        Err(e) => { println!("msi crate open failed: {:?}", e); return; }
    };

    // Set SummaryInfo properly
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

    // Create tables
    pkg.create_table("Property", vec![
        msi::Column::build("Property").primary_key().string(72),
        msi::Column::build("Value").nullable().localizable().string(255),
    ]).unwrap();
    pkg.create_table("Directory", vec![
        msi::Column::build("Directory").primary_key().string(72),
        msi::Column::build("Directory_Parent").nullable().string(72),
        msi::Column::build("DefaultDir").nullable().localizable().string(255),
    ]).unwrap();
    pkg.create_table("Component", vec![
        msi::Column::build("Component").primary_key().string(72),
        msi::Column::build("ComponentId").nullable().string(38),
        msi::Column::build("Directory_").string(72),
        msi::Column::build("Attributes").int16(),
        msi::Column::build("Condition").nullable().string(255),
        msi::Column::build("KeyPath").nullable().string(72),
    ]).unwrap();
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
    pkg.create_table("FeatureComponents", vec![
        msi::Column::build("Feature_").primary_key().string(38),
        msi::Column::build("Component_").primary_key().string(72),
    ]).unwrap();
    pkg.create_table("InstallExecuteSequence", vec![
        msi::Column::build("Action").primary_key().string(72),
        msi::Column::build("Condition").nullable().string(255),
        msi::Column::build("Sequence").nullable().int16(),
    ]).unwrap();
    pkg.create_table("InstallUISequence", vec![
        msi::Column::build("Action").primary_key().string(72),
        msi::Column::build("Condition").nullable().string(255),
        msi::Column::build("Sequence").nullable().int16(),
    ]).unwrap();
    println!("Tables created");

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

    println!("Data inserted");

    // Flush
    match pkg.flush() {
        Ok(_) => println!("Flush OK"),
        Err(e) => { println!("Flush failed: {:?}", e); return; }
    }

    let cursor = pkg.into_inner().unwrap();
    let msi_data = cursor.into_inner();
    std::fs::write(out_path, &msi_data).unwrap();
    println!("Wrote: {} ({} bytes)", out_path, msi_data.len());

    // Verify V3
    println!("Output CFB Version: {} (sector: {})", 
        msi_data[26] as u16 + (msi_data[27] as u16) * 256,
        2u32.pow((msi_data[30] as u16 + (msi_data[31] as u16) * 256) as u32));

    // Test with msiexec
    println!("\n--- msiexec test ---");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", "C:\\temp\\v3_msi.log"])
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

    if let Ok(log) = std::fs::read_to_string("C:\\temp\\v3_msi.log") {
        for line in log.lines() {
            if line.contains("Error") || line.contains("2219") || line.contains("2203") ||
               line.contains("Product:") || line.contains("successful") || line.contains("Installation") {
                println!("  {}", line.trim());
            }
        }
    }

    println!("\n=== DONE ===");
}
