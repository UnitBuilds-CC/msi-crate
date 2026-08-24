/// Reference MSI: use the msi crate to create a complete installable MSI
/// with embedded cabinet, then test with msiexec.
/// cargo run --example msi_ref_create_test -p velocity-msi
use std::io::Cursor;

/// Repackage a V4 CFB file as V3 by reading all streams and recreating.
fn repackage_v4_to_v3(v4_data: &[u8]) -> Vec<u8> {
    // Open the V4 file, collect stream names
    let cursor = Cursor::new(v4_data);
    let comp = cfb::CompoundFile::open(cursor).expect("open V4");
    let clsid = *comp.root_entry().clsid();

    let stream_names: Vec<String> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.name().to_string())
        .collect();

    // Read each stream's data
    let mut comp = cfb::CompoundFile::open(Cursor::new(v4_data)).expect("reopen V4");
    let mut streams_with_data: Vec<(String, Vec<u8>)> = Vec::new();
    for name in &stream_names {
        let mut s = comp.open_stream(name.as_str()).expect("open stream");
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut s, &mut data).expect("read stream");
        streams_with_data.push((name.clone(), data));
    }

    // Create a new V3 file
    let mut v3_buf = Vec::new();
    {
        let cursor = Cursor::new(&mut v3_buf);
        let mut v3_comp = cfb::CompoundFile::create_with_version(
            cfb::Version::V3, cursor,
        ).expect("create V3");

        v3_comp.set_storage_clsid("", clsid).expect("set clsid");

        for (name, data) in &streams_with_data {
            let mut s = v3_comp.create_stream(name).expect("create stream");
            std::io::Write::write_all(&mut s, data).expect("write stream");
        }

        v3_comp.flush().expect("flush V3");
    }

    v3_buf
}

fn main() {
    println!("=== MSI CRATE REFERENCE CREATE TEST ===\n");

    // Generate unique codes
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let product_code = format!(
        "{{{:08X}-{:04X}-4{:03X}-{:04X}-{:08X}{:04X}}}",
        (t & 0xFFFFFFFF) as u32,
        ((t >> 32) & 0xFFFF) as u16,
        ((t >> 48) & 0x0FFF) as u16,
        (((t >> 44) & 0x0FFF) as u16) | 0x8000,
        ((t >> 16) & 0xFFFFFFFF) as u32,
        (t & 0xFFFF) as u16,
    );
    let upgrade_code = format!(
        "{{{:08X}-{:04X}-4{:03X}-{:04X}-{:08X}{:04X}}}",
        ((t + 1) & 0xFFFFFFFF) as u32,
        (((t + 1) >> 32) & 0xFFFF) as u16,
        (((t + 1) >> 48) & 0x0FFF) as u16,
        ((((t + 1) >> 44) & 0x0FFF) as u16) | 0x8000,
        (((t + 1) >> 16) & 0xFFFFFFFF) as u32,
        ((t + 1) & 0xFFFF) as u16,
    );

    let cursor = Cursor::new(Vec::new());
    let mut package = msi::Package::create(msi::PackageType::Installer, cursor)
        .expect("create package");

    // Set summary info
    package.summary_info_mut().set_author("Velocity".to_string());
    package.summary_info_mut().set_subject("Test".to_string());
    package.summary_info_mut().set_title("MSI Ref Test".to_string());

    // Set template (platform;language) via separate arch + language
    package.summary_info_mut().set_arch("Intel");
    package.summary_info_mut().set_languages(&[msi::Language::from_code(1033)]);

    // === Property table ===
    package.create_table("Property", vec![
        msi::Column::build("Property").primary_key().id_string(72),
        msi::Column::build("Value").nullable().formatted_string(255),
    ]).expect("create Property");

    package.insert_rows(msi::Insert::into("Property")
        .row(vec![msi::Value::from("ProductName"), msi::Value::from("MSI Ref Test")])
        .row(vec![msi::Value::from("ProductVersion"), msi::Value::from("1.0.0")])
        .row(vec![msi::Value::from("Manufacturer"), msi::Value::from("Velocity")])
        .row(vec![msi::Value::from("ProductCode"), msi::Value::from(product_code.as_str())])
        .row(vec![msi::Value::from("UpgradeCode"), msi::Value::from(upgrade_code.as_str())])
        .row(vec![msi::Value::from("ProductLanguage"), msi::Value::from("1033")])
    ).expect("insert Property");

    // === Directory table ===
    package.create_table("Directory", vec![
        msi::Column::build("Directory").primary_key().id_string(72),
        msi::Column::build("Directory_Parent").nullable().foreign_key("Directory", 1).id_string(72),
        msi::Column::build("DefaultDir").nullable().formatted_string(255),
    ]).expect("create Directory");

    package.insert_rows(msi::Insert::into("Directory")
        .row(vec![msi::Value::from("TARGETDIR"), msi::Value::Null, msi::Value::from("SourceDir")])
        .row(vec![msi::Value::from("ProgramFilesFolder"), msi::Value::from("TARGETDIR"), msi::Value::from("PFiles")])
        .row(vec![msi::Value::from("INSTALLDIR"), msi::Value::from("ProgramFilesFolder"), msi::Value::from("VelRefTest")])
    ).expect("insert Directory");

    // === Component table ===
    package.create_table("Component", vec![
        msi::Column::build("Component").primary_key().id_string(72),
        msi::Column::build("ComponentId").nullable().id_string(38),
        msi::Column::build("Directory_").foreign_key("Directory", 1).id_string(72),
        msi::Column::build("Attributes").int16(),
        msi::Column::build("Condition").nullable().formatted_string(255),
        msi::Column::build("KeyPath").nullable().id_string(72),
    ]).expect("create Component");

    package.insert_rows(msi::Insert::into("Component")
        .row(vec![
            msi::Value::from("MainComp"),
            msi::Value::Null,
            msi::Value::from("INSTALLDIR"),
            msi::Value::Int(0),
            msi::Value::Null,
            msi::Value::from("MainFile"),
        ])
    ).expect("insert Component");

    // === Feature table ===
    package.create_table("Feature", vec![
        msi::Column::build("Feature").primary_key().id_string(38),
        msi::Column::build("Feature_Parent").nullable().foreign_key("Feature", 1).id_string(38),
        msi::Column::build("Title").nullable().formatted_string(64),
        msi::Column::build("Description").nullable().formatted_string(255),
        msi::Column::build("Display").nullable().int16(),
        msi::Column::build("Level").int16(),
        msi::Column::build("Directory_").nullable().foreign_key("Directory", 1).id_string(72),
        msi::Column::build("Attributes").int16(),
    ]).expect("create Feature");

    package.insert_rows(msi::Insert::into("Feature")
        .row(vec![
            msi::Value::from("MainFeat"),
            msi::Value::Null,
            msi::Value::from("Complete"),
            msi::Value::Null,
            msi::Value::Null,
            msi::Value::Int(1),
            msi::Value::Null,
            msi::Value::Int(0),
        ])
    ).expect("insert Feature");

    // === FeatureComponents table ===
    package.create_table("FeatureComponents", vec![
        msi::Column::build("Feature_").primary_key().foreign_key("Feature", 1).id_string(38),
        msi::Column::build("Component_").primary_key().foreign_key("Component", 1).id_string(72),
    ]).expect("create FeatureComponents");

    package.insert_rows(msi::Insert::into("FeatureComponents")
        .row(vec![msi::Value::from("MainFeat"), msi::Value::from("MainComp")])
    ).expect("insert FeatureComponents");

    // === File table ===
    package.create_table("File", vec![
        msi::Column::build("File_").primary_key().id_string(72),
        msi::Column::build("Component_").foreign_key("Component", 1).id_string(72),
        msi::Column::build("FileName").formatted_string(255),
        msi::Column::build("FileSize").nullable().int32(),
        msi::Column::build("Sequence").int16(),
    ]).expect("create File");

    let file_content = b"Hello from MSI reference test!";
    package.insert_rows(msi::Insert::into("File")
        .row(vec![
            msi::Value::from("MainFile"),
            msi::Value::from("MainComp"),
            msi::Value::from("testfile.txt"),
            msi::Value::Int(file_content.len() as i32),
            msi::Value::Int(1),
        ])
    ).expect("insert File");

    // === Media table ===
    package.create_table("Media", vec![
        msi::Column::build("DiskId").primary_key().int16(),
        msi::Column::build("LastSequence").int16(),
        msi::Column::build("DiskPrompt").nullable().formatted_string(64),
        msi::Column::build("Cabinet").nullable().category(msi::Category::Cabinet).string(255),
        msi::Column::build("VolumeLabel").nullable().formatted_string(32),
        msi::Column::build("Source").nullable().formatted_string(72),
    ]).expect("create Media");

    package.insert_rows(msi::Insert::into("Media")
        .row(vec![
            msi::Value::Int(1),
            msi::Value::Int(1),
            msi::Value::Null,
            msi::Value::from("#velcab.cab"),
            msi::Value::Null,
            msi::Value::Null,
        ])
    ).expect("insert Media");

    // === InstallExecuteSequence table ===
    package.create_table("InstallExecuteSequence", vec![
        msi::Column::build("Action").primary_key().id_string(72),
        msi::Column::build("Condition").nullable().formatted_string(255),
        msi::Column::build("Sequence").nullable().int16(),
    ]).expect("create InstallExecuteSequence");

    package.insert_rows(msi::Insert::into("InstallExecuteSequence")
        .row(vec![msi::Value::from("CostInitialize"), msi::Value::Null, msi::Value::Int(800)])
        .row(vec![msi::Value::from("FileCost"), msi::Value::Null, msi::Value::Int(900)])
        .row(vec![msi::Value::from("CostFinalize"), msi::Value::Null, msi::Value::Int(1000)])
        .row(vec![msi::Value::from("InstallValidate"), msi::Value::Null, msi::Value::Int(1400)])
        .row(vec![msi::Value::from("InstallInitialize"), msi::Value::Null, msi::Value::Int(1500)])
        .row(vec![msi::Value::from("InstallFinalize"), msi::Value::Null, msi::Value::Int(6600)])
    ).expect("insert InstallExecuteSequence");

    // === InstallUISequence table ===
    package.create_table("InstallUISequence", vec![
        msi::Column::build("Action").primary_key().id_string(72),
        msi::Column::build("Condition").nullable().formatted_string(255),
        msi::Column::build("Sequence").nullable().int16(),
    ]).expect("create InstallUISequence");

    package.insert_rows(msi::Insert::into("InstallUISequence")
        .row(vec![msi::Value::from("CostInitialize"), msi::Value::Null, msi::Value::Int(800)])
        .row(vec![msi::Value::from("CostFinalize"), msi::Value::Null, msi::Value::Int(1000)])
        .row(vec![msi::Value::from("ExecuteAction"), msi::Value::Null, msi::Value::Int(1300)])
    ).expect("insert InstallUISequence");

    // === Create cabinet and embed as stream ===
    let cab_files = vec![
        velocity_msi::CabinetFile {
            name: "testfile.txt".to_string(),
            data: file_content.to_vec(),
        },
    ];
    let cab_data = velocity_msi::build_cabinet(&cab_files);
    println!("Cabinet data: {} bytes", cab_data.len());

    // Write cabinet stream
    {
        let mut writer = package.write_stream("#velcab.cab").expect("write_stream");
        std::io::Write::write_all(&mut writer, &cab_data).expect("write cab data");
    }
    println!("Cabinet stream written");

    // Flush and get data (msi crate produces V4, need to convert to V3)
    package.flush().expect("flush");
    let cursor = package.into_inner().expect("into_inner");
    let v4_data = cursor.into_inner();
    println!("msi crate V4 MSI size: {} bytes", v4_data.len());

    // Repackage from V4 to V3 (msiexec requires V3)
    let msi_data = repackage_v4_to_v3(&v4_data);
    println!("Repackaged V3 MSI size: {} bytes", msi_data.len());

    // Write to file
    let _ = std::fs::create_dir_all("C:\\temp");
    let path = "C:\\temp\\msi_ref_create.msi";
    let log_path = "C:\\temp\\msi_ref_create.log";
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(log_path);
    std::fs::write(path, &msi_data).expect("write msi");
    println!("Written to {}", path);

    // List streams
    println!("\nStreams in MSI:");
    let cursor2 = Cursor::new(&msi_data);
    let comp = cfb::CompoundFile::open(cursor2).expect("open cfb");
    let entries: Vec<(String, u64)> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| (e.name().to_string(), e.len()))
        .collect();
    for (name, size) in &entries {
        let name_bytes: Vec<u8> = name.encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect();
        let hex: String = name_bytes.iter()
            .take(16)
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");
        println!("  '{}' ({} bytes) [{}]", name, size, hex);
    }

    // Test with msiexec
    println!("\nTesting with msiexec...");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", path, "/qn", "/l*v", log_path])
        .output()
        .expect("msiexec");
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);

    if exit_code == 0 {
        println!("SUCCESS! Reference MSI installed!");
        // Check installed files
        let install_dir = "C:\\Program Files (x86)\\VelRefTest";
        if std::path::Path::new(install_dir).exists() {
            println!("Install dir exists: {}", install_dir);
            if let Ok(entries) = std::fs::read_dir(install_dir) {
                for entry in entries.flatten() {
                    println!("  INSTALLED: {} ({} bytes)",
                        entry.file_name().to_string_lossy(),
                        entry.metadata().map(|m| m.len()).unwrap_or(0));
                }
            }
        } else {
            println!("WARNING: Install dir NOT found!");
        }

        // Uninstall
        println!("\nUninstalling...");
        let uninst = std::process::Command::new("msiexec")
            .args(&["/x", &product_code, "/qn"])
            .output()
            .expect("uninstall");
        println!("Uninstall exit code: {}", uninst.status.code().unwrap_or(-1));
    } else {
        println!("FAILED! Exit code: {}", exit_code);
        // Check log for errors
        if let Ok(log) = std::fs::read_to_string(log_path) {
            println!("\nKey log entries:");
            for line in log.lines() {
                if line.contains("error") || line.contains("Error") ||
                   line.contains("2725") || line.contains("1603") ||
                   line.contains("return value 3") {
                    println!("  {}", line);
                }
            }
        } else {
            println!("No log file found at {}", log_path);
        }
    }
}
