/// Diagnostic: build MSI, repackage with cfb crate, test both with msiexec.
/// Definitively isolates OLE writer bugs from data bugs.
use std::io::{Cursor, Read, Write};
use std::path::Path;

fn main() {
    let output_dir = Path::new(r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output");
    std::fs::create_dir_all(output_dir).unwrap();

    let custom_path = output_dir.join("diag_custom.msi");
    let cfb_path = output_dir.join("diag_cfb.msi");

    // Create test files
    let test_dir = output_dir.join("diag_files");
    std::fs::create_dir_all(&test_dir).unwrap();
    std::fs::write(test_dir.join("hello.txt"), "Hello from velocity-msi!").unwrap();
    std::fs::write(test_dir.join("data.txt"), "Test data for diagnostic").unwrap();

    // Build MSI
    let mut builder = velocity_msi::MsiBuilder::new();
    builder.set_title("Diagnostic Test");
    builder.set_author("Velocity");
    builder.set_subject("CFB repack diagnostic");
    builder.set_template("Intel", 1033);
    builder.set_include_validation(false);

    let product_code = "{F1234567-1234-1234-1234-123456789ABC}";
    let upgrade_code = "{F2234567-1234-1234-1234-123456789ABC}";

    create_tables(&mut builder);
    populate_properties(&mut builder, product_code, upgrade_code);
    populate_directories(&mut builder);

    let files = vec![
        (test_dir.join("hello.txt"), "hello.txt"),
        (test_dir.join("data.txt"), "data.txt"),
    ];
    populate_components(&mut builder, &files);
    populate_features(&mut builder, files.len());
    populate_sequences(&mut builder);
    build_cabinet(&mut builder, &files);

    let msi_data = builder.build().unwrap();
    std::fs::write(&custom_path, &msi_data).unwrap();
    eprintln!("Custom OLE: {} bytes", msi_data.len());

    // Repackage with cfb crate
    repackage_cfb(&msi_data, &cfb_path);
    let cfb_data = std::fs::read(&cfb_path).unwrap();
    eprintln!("CFB repack: {} bytes", cfb_data.len());

    // Verify stream data matches
    verify_streams(&msi_data, &cfb_data);

    eprintln!("\n=== Test commands ===");
    eprintln!("# Custom OLE:");
    eprintln!("Start-Process msiexec -ArgumentList '/i','{}','/qn','/l*vx','{}' -Wait",
        custom_path.display(), output_dir.join("diag_custom.log").display());
    eprintln!("# CFB repack:");
    eprintln!("Start-Process msiexec -ArgumentList '/i','{}','/qn','/l*vx','{}' -Wait",
        cfb_path.display(), output_dir.join("diag_cfb.log").display());
    eprintln!("# Uninstall:");
    eprintln!("Start-Process msiexec -ArgumentList '/x','{}','/qn' -Wait", product_code);
}

fn repackage_cfb(msi_data: &[u8], output: &Path) {
    // Open our MSI with cfb crate
    let cursor = Cursor::new(msi_data);
    let mut src = cfb::CompoundFile::open(cursor).expect("cfb should read our MSI");

    // Collect all stream paths first (to avoid borrow issues)
    let mut stream_paths: Vec<String> = Vec::new();
    for entry in src.walk() {
        if entry.is_stream() {
            stream_paths.push(entry.path().to_string_lossy().to_string());
        }
    }

    // Now read each stream's data
    let mut streams: Vec<(String, Vec<u8>)> = Vec::new();
    for path in &stream_paths {
        let mut stream = src.open_stream(path).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        streams.push((path.clone(), data));
    }
    drop(src);

    eprintln!("Streams found: {}", streams.len());
    for (path, data) in &streams {
        eprintln!("  {} ({} bytes)", path, data.len());
    }

    // Create new compound file with cfb - use V3 (512-byte sectors) for MSI compatibility
    let cursor = Cursor::new(Vec::new());
    let mut dst = cfb::CompoundFile::create_with_version(cfb::Version::V3, cursor).unwrap();
    for (path, data) in &streams {
        let mut s = dst.create_stream(path).unwrap();
        s.write_all(data).unwrap();
    }

    // Write to bytes
    let mut cfb_data = dst.into_inner().into_inner();

    // Patch MSI CLSID on root entry: {000C1084-0000-0000-C000-000000000046}
    let clsid: [u8; 16] = [
        0x84, 0x10, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00,
        0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
    ];
    // Find root entry: first dir sector, first entry, offset 80
    let sector_size = 1usize << u16::from_le_bytes([cfb_data[30], cfb_data[31]]);
    let first_dir = u32::from_le_bytes([cfb_data[48], cfb_data[49], cfb_data[50], cfb_data[51]]) as usize;
    let root_off = 512 + first_dir * sector_size;
    if root_off + 96 <= cfb_data.len() {
        cfb_data[root_off + 80..root_off + 96].copy_from_slice(&clsid);
    }

    std::fs::write(output, &cfb_data).unwrap();
}

fn verify_streams(custom: &[u8], cfb_data: &[u8]) {
    // Open both with cfb and compare stream data
    let src_custom = cfb::CompoundFile::open(Cursor::new(custom)).expect("open custom");
    let src_cfb = cfb::CompoundFile::open(Cursor::new(cfb_data)).expect("open cfb");

    eprintln!("\n=== Stream verification ===");
    let mut all_match = true;
    for entry in src_custom.walk() {
        if !entry.is_stream() { continue; }
        let path = entry.path().to_string_lossy().to_string();
        let custom_size = entry.len();

        // Try to find same stream in cfb
        match src_cfb.entry(&path) {
            Ok(cfb_entry) => {
                let cfb_size = cfb_entry.len();
                if custom_size != cfb_size {
                    eprintln!("  SIZE MISMATCH: {} (custom={} cfb={})", path, custom_size, cfb_size);
                    all_match = false;
                } else {
                    // Compare data
                    let mut s1 = cfb::CompoundFile::open(Cursor::new(custom)).unwrap();
                    let mut s2 = cfb::CompoundFile::open(Cursor::new(cfb_data)).unwrap();
                    let mut d1 = Vec::new();
                    let mut d2 = Vec::new();
                    s1.open_stream(&path).unwrap().read_to_end(&mut d1).unwrap();
                    s2.open_stream(&path).unwrap().read_to_end(&mut d2).unwrap();
                    if d1 == d2 {
                        eprintln!("  OK: {} ({} bytes)", path, custom_size);
                    } else {
                        eprintln!("  DATA MISMATCH: {} ({} bytes)", path, custom_size);
                        all_match = false;
                    }
                }
            }
            Err(_) => {
                eprintln!("  MISSING in CFB: {}", path);
                all_match = false;
            }
        }
    }

    if all_match {
        eprintln!("\nAll streams match! OLE writer is correct.");
    } else {
        eprintln!("\nStream differences found! OLE writer has bugs.");
    }
}

// ── MSI table creation and population ──────────────────────────────

fn create_tables(b: &mut velocity_msi::MsiBuilder) {
    use velocity_msi::Column;
    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).nullable().build(),
    ]).unwrap();
    b.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(38).nullable().build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
    b.create_table("File", vec![
        Column::build("File").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).build(),
        Column::build("FileSize").int32().build(),
        Column::build("Version").string(72).nullable().build(),
        Column::build("Language").string(20).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
        Column::build("Sequence").int32().build(),
    ]).unwrap();
    b.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int32().build(),
        Column::build("DiskPrompt").string(255).nullable().build(),
        Column::build("VolumeLabel").string(32).nullable().build(),
        Column::build("Cabinet").string(255).nullable().build(),
        Column::build("Source").string(72).nullable().build(),
    ]).unwrap();
    b.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").string(38).nullable().build(),
        Column::build("Title").string(64).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().build(),
    ]).unwrap();
    b.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    b.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.create_table("InstallUISequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
}

fn populate_properties(b: &mut velocity_msi::MsiBuilder, pc: &str, uc: &str) {
    use velocity_msi::Value;
    for (n, v) in &[
        ("ProductName", "Diagnostic Test"),
        ("ProductVersion", "1.0.0"),
        ("Manufacturer", "Velocity"),
        ("ProductCode", pc),
        ("UpgradeCode", uc),
        ("ProductLanguage", "1033"),
    ] {
        b.insert_rows("Property", vec![vec![Value::from(*n), Value::from(*v)]]).unwrap();
    }
}

fn populate_directories(b: &mut velocity_msi::MsiBuilder) {
    use velocity_msi::Value;
    b.insert_rows("Directory", vec![vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")]]).unwrap();
    b.insert_rows("Directory", vec![vec![Value::from("LocalAppDataFolder"), Value::from("TARGETDIR"), Value::from("LocalAppData")]]).unwrap();
    b.insert_rows("Directory", vec![vec![Value::from("INSTALLDIR"), Value::from("LocalAppDataFolder"), Value::from("DiagTest:DiagTest")]]).unwrap();
}

fn populate_components(b: &mut velocity_msi::MsiBuilder, files: &[(std::path::PathBuf, &str)]) {
    use velocity_msi::Value;
    for (i, (path, name)) in files.iter().enumerate() {
        let c = format!("comp_{}", i);
        let f = format!("file_{}", i);
        let sz = std::fs::metadata(path).map(|m| m.len() as i32).unwrap_or(0);
        b.insert_rows("Component", vec![vec![
            Value::from(c.as_str()), Value::Null, Value::from("INSTALLDIR"),
            Value::Int(0), Value::Null, Value::from(f.as_str()),
        ]]).unwrap();
        b.insert_rows("File", vec![vec![
            Value::from(f.as_str()), Value::from(c.as_str()), Value::from(*name),
            Value::Int(sz), Value::Null, Value::Null, Value::Int(0), Value::Int((i+1) as i32),
        ]]).unwrap();
    }
    b.insert_rows("Media", vec![vec![
        Value::Int(1), Value::Int(files.len() as i32), Value::Null, Value::Null,
        Value::from("#Velocity.cab"), Value::Null,
    ]]).unwrap();
}

fn populate_features(b: &mut velocity_msi::MsiBuilder, count: usize) {
    use velocity_msi::Value;
    b.insert_rows("Feature", vec![vec![
        Value::from("Complete"), Value::Null, Value::from("Complete"), Value::from("All"),
        Value::Int(1), Value::Int(1), Value::from("INSTALLDIR"), Value::Int(0),
    ]]).unwrap();
    for i in 0..count {
        b.insert_rows("FeatureComponents", vec![vec![
            Value::from("Complete"), Value::from(format!("comp_{}", i).as_str()),
        ]]).unwrap();
    }
}

fn populate_sequences(b: &mut velocity_msi::MsiBuilder) {
    use velocity_msi::Value;
    for (a, c, s) in &[
        ("LaunchConditions", Some("NOT Installed") as Option<&str>, 100i32),
        ("CostInitialize", None as Option<&str>, 800),
        ("FileCost", None as Option<&str>, 900),
        ("CostFinalize", None as Option<&str>, 1000),
        ("InstallValidate", None as Option<&str>, 1400),
        ("InstallInitialize", None as Option<&str>, 1500),
        ("ProcessComponents", None as Option<&str>, 1600),
        ("InstallFiles", None as Option<&str>, 4000),
        ("RegisterProduct", None as Option<&str>, 6100),
        ("PublishFeatures", Some("NOT Installed") as Option<&str>, 6300),
        ("PublishProduct", Some("NOT Installed") as Option<&str>, 6400),
        ("InstallFinalize", None as Option<&str>, 6600),
    ] {
        let cv = match c { Some(s) => Value::from(*s), None => Value::Null };
        b.insert_rows("InstallExecuteSequence", vec![vec![Value::from(*a), cv, Value::Int(*s)]]).unwrap();
    }
    for (a, c, s) in &[
        ("LaunchConditions", None as Option<&str>, 100i32),
        ("CostInitialize", None as Option<&str>, 800),
        ("CostFinalize", None as Option<&str>, 1000),
        ("ExecuteAction", None as Option<&str>, 1300),
    ] {
        let cv = match c { Some(s) => Value::from(*s), None => Value::Null };
        b.insert_rows("InstallUISequence", vec![vec![Value::from(*a), cv, Value::Int(*s)]]).unwrap();
    }
}

fn build_cabinet(b: &mut velocity_msi::MsiBuilder, files: &[(std::path::PathBuf, &str)]) {
    let ids: Vec<String> = (0..files.len()).map(|i| format!("file_{}", i)).collect();
    let mut cab = Cursor::new(Vec::new());
    {
        let mut builder = cab::CabinetBuilder::new();
        let folder = builder.add_folder(cab::CompressionType::MsZip);
        for id in &ids { folder.add_file(id); }
        let mut w = builder.build(&mut cab).unwrap();
        for (p, _) in files {
            let mut fw = w.next_file().unwrap().unwrap();
            let mut r = std::fs::File::open(p).unwrap();
            std::io::copy(&mut r, &mut fw).unwrap();
        }
        w.finish().unwrap();
    }
    b.add_stream("Velocity.cab".to_string(), cab.into_inner());
}
