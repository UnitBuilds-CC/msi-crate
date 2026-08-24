/// Test: msi crate V4 → cfb V3 conversion
/// Does the cfb crate's V3 conversion preserve MSI validity?
/// cargo run --example v3_conversion_test -p velocity-msi
use std::io::{Cursor, Read, Write};

fn main() {
    println!("=== V3 CONVERSION TEST ===\n");

    // Step 1: Create MSI with msi crate (V4)
    let cursor = Cursor::new(Vec::new());
    let mut pkg = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
    {
        let si = pkg.summary_info_mut();
        si.set_title("Test Product");
        si.set_author("Test Corp");
        si.set_codepage(msi::CodePage::Windows1252);
        si.set_arch("Intel");
        si.set_languages(&[msi::Language::from_code(1033)]);
        si.set_word_count(2);
        si.set_uuid(uuid::Uuid::nil());
        si.set_creating_application("Test");
    }
    pkg.set_database_codepage(msi::CodePage::Windows1252);

    // Create Property table
    pkg.create_table("Property", vec![
        msi::Column::build("Property").primary_key().id_string(72),
        msi::Column::build("Value").nullable().localizable().formatted_string(255),
    ]).unwrap();
    pkg.insert_rows(msi::Insert::into("Property")
        .row(vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Test".into())])
        .row(vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0".into())])
        .row(vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Test".into())])
        .row(vec![msi::Value::Str("ProductCode".into()), msi::Value::Str("{12345678-1234-4234-8234-123456789012}".into())])
        .row(vec![msi::Value::Str("UpgradeCode".into()), msi::Value::Str("{22345678-1234-4234-8234-123456789012}".into())])
        .row(vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())])
    ).unwrap();

    // Create Directory table
    pkg.create_table("Directory", vec![
        msi::Column::build("Directory").primary_key().id_string(72),
        msi::Column::build("Directory_Parent").nullable().string(72),
        msi::Column::build("DefaultDir").nullable().localizable().formatted_string(255),
    ]).unwrap();
    pkg.insert_rows(msi::Insert::into("Directory")
        .row(vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str("SourceDir".into())])
        .row(vec![msi::Value::Str("ProgramFilesFolder".into()), msi::Value::Str("TARGETDIR".into()), msi::Value::Str("PFiles".into())])
        .row(vec![msi::Value::Str("INSTALLDIR".into()), msi::Value::Str("ProgramFilesFolder".into()), msi::Value::Str("Test".into())])
    ).unwrap();

    // Create Component table
    pkg.create_table("Component", vec![
        msi::Column::build("Component").primary_key().id_string(72),
        msi::Column::build("ComponentId").nullable().string(38),
        msi::Column::build("Directory_").id_string(72),
        msi::Column::build("Attributes").int16(),
        msi::Column::build("Condition").nullable().formatted_string(255),
        msi::Column::build("KeyPath").nullable().id_string(72),
    ]).unwrap();
    pkg.insert_rows(msi::Insert::into("Component")
        .row(vec![msi::Value::Str("C1".into()), msi::Value::Null, msi::Value::Str("INSTALLDIR".into()), msi::Value::Int(0), msi::Value::Null, msi::Value::Null])
    ).unwrap();

    // Create Feature table
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
    pkg.insert_rows(msi::Insert::into("Feature")
        .row(vec![msi::Value::Str("F1".into()), msi::Value::Null, msi::Value::Null, msi::Value::Null, msi::Value::Null, msi::Value::Int(1), msi::Value::Null, msi::Value::Int(0)])
    ).unwrap();

    // Create FeatureComponents table
    pkg.create_table("FeatureComponents", vec![
        msi::Column::build("Feature_").primary_key().id_string(38),
        msi::Column::build("Component_").primary_key().id_string(72),
    ]).unwrap();
    pkg.insert_rows(msi::Insert::into("FeatureComponents")
        .row(vec![msi::Value::Str("F1".into()), msi::Value::Str("C1".into())])
    ).unwrap();

    // Create Media table
    pkg.create_table("Media", vec![
        msi::Column::build("DiskId").primary_key().int16(),
        msi::Column::build("LastSequence").int16(),
        msi::Column::build("DiskPrompt").nullable().localizable().formatted_string(64),
        msi::Column::build("Cabinet").nullable().string(255),
        msi::Column::build("VolumeLabel").nullable().localizable().id_string(32),
        msi::Column::build("Source").nullable().localizable().id_string(72),
    ]).unwrap();
    pkg.insert_rows(msi::Insert::into("Media")
        .row(vec![msi::Value::Int(1), msi::Value::Int(0), msi::Value::Null, msi::Value::Null, msi::Value::Null, msi::Value::Null])
    ).unwrap();

    pkg.flush().unwrap();
    let v4_data = pkg.into_inner().unwrap().into_inner();
    println!("V4 MSI: {} bytes (V{})", v4_data.len(), v4_data[26]);

    // Test V4 directly
    let v4_path = "C:\\temp\\msi_crate_v4.msi";
    std::fs::write(v4_path, &v4_data).unwrap();
    println!("\n--- V4 msiexec test ---");
    test_msiexec(v4_path);

    // Step 2: Convert V4 to V3 using cfb crate
    let v3_path = "C:\\temp\\msi_crate_v3.msi";
    {
        let mut src = cfb::CompoundFile::open(Cursor::new(&v4_data)).unwrap();

        // Read all streams
        let stream_entries: Vec<(String, Vec<u8>)> = {
            let names: Vec<String> = src.walk()
                .filter(|e| e.is_stream())
                .map(|e| e.name().to_string())
                .collect();
            names.into_iter().map(|name| {
                let mut stream = src.open_stream(&name).unwrap();
                let mut data = Vec::new();
                stream.read_to_end(&mut data).unwrap();
                (name, data)
            }).collect()
        };

        let root_clsid = {
            let re = src.root_entry();
            *re.clsid()
        };

        println!("\nV4 streams ({}):", stream_entries.len());
        for (name, data) in &stream_entries {
            let hex: String = name.encode_utf16()
                .map(|c| format!("{:04X}", c))
                .collect::<Vec<_>>().join(" ");
            println!("  [{}] {} bytes", hex, data.len());
        }
        println!("V4 root CLSID: {}", root_clsid);

        // Create V3
        let mut v3_buf = Vec::new();
        {
            let cursor = Cursor::new(&mut v3_buf);
            let mut v3 = cfb::CompoundFile::create_with_version(
                cfb::Version::V3, cursor,
            ).unwrap();
            v3.set_storage_clsid("", root_clsid).unwrap();
            for (name, data) in &stream_entries {
                let mut s = v3.create_stream(name).unwrap();
                s.write_all(data).unwrap();
            }
            v3.flush().unwrap();
        }
        std::fs::write(v3_path, &v3_buf).unwrap();
        println!("\nV3 MSI: {} bytes (V{})", v3_buf.len(), v3_buf[26]);

        // Verify V3 streams
        let mut v3_comp = cfb::CompoundFile::open(Cursor::new(&v3_buf)).unwrap();
        let v3_names: Vec<String> = v3_comp.walk()
            .filter(|e| e.is_stream())
            .map(|e| e.name().to_string())
            .collect();
        println!("V3 streams ({}):", v3_names.len());
        for name in &v3_names {
            let mut s = v3_comp.open_stream(name).unwrap();
            let mut data = Vec::new();
            s.read_to_end(&mut data).unwrap();
            let hex: String = name.encode_utf16()
                .map(|c| format!("{:04X}", c))
                .collect::<Vec<_>>().join(" ");
            println!("  [{}] {} bytes", hex, data.len());
        }
        let v3_root = v3_comp.root_entry();
        println!("V3 root CLSID: {}", v3_root.clsid());
    }

    // Test V3
    println!("\n--- V3 msiexec test ---");
    test_msiexec(v3_path);

    println!("\n=== DONE ===");
}

fn test_msiexec(path: &str) {
    let output = std::process::Command::new("msiexec")
        .args(&["/i", path, "/qn"])
        .output().unwrap();
    let ec = output.status.code().unwrap_or(-1);
    let desc = match ec {
        0 => "SUCCESS",
        1613 => "invalid package",
        1619 => "not valid",
        1620 => "could not open",
        _ => "other",
    };
    println!("  msiexec: {} ({})", ec, desc);
}
