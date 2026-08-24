/// Definitive file installation test: real MSZIP cabinet + File table + all required tables
/// Goal: msiexec exit code 0 AND files written to disk
///
/// cargo run --example real_cabinet_test -p velocity-msi
use velocity_msi::{Column, MsiBuilder, Value};
use std::io::Cursor;

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
    format!("{{{:08X}-{:04X}-{:04X}-{:04X}-{:012X}}}", a, b, c, d, e & 0xFFFFFFFFFFFF)
}

fn build_cabinet(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut cab_data = Cursor::new(Vec::new());
    {
        let mut cab_builder = cab::CabinetBuilder::new();
        let folder = cab_builder.add_folder(cab::CompressionType::MsZip);
        for (name, _) in files {
            folder.add_file(*name);
        }
        let mut cab_writer = cab_builder.build(&mut cab_data).unwrap();
        for (_, data) in files {
            let mut writer = cab_writer.next_file().unwrap().unwrap();
            std::io::copy(&mut &data[..], &mut writer).unwrap();
        }
        cab_writer.finish().unwrap();
    }
    cab_data.into_inner()
}

fn main() {
    println!("=== REAL CABINET FILE INSTALL TEST ===\n");

    let out_path = "C:\\temp\\real_cabinet_test.msi";
    let log_path = "C:\\temp\\real_cabinet_test.log";
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\Users\\visse\\AppData\\Local".to_string());
    let install_dir = format!("{}\\VelTest", local_app_data);
    let _ = std::fs::remove_file(out_path);
    let _ = std::fs::remove_file(log_path);
    let _ = std::fs::remove_dir_all(&install_dir);

    // Create test file content
    let file1_content = b"Hello from velocity-msi! This is test file 1.\r\n";
    let file2_content = b"Second test file - velocity installer works!\r\n";

    // Build a real MSZIP cabinet
    // CFFILE names must match File table primary keys
    let cab_data = build_cabinet(&[
        ("file_0", file1_content),
        ("file_1", file2_content),
    ]);
    println!("Cabinet size: {} bytes", cab_data.len());

    let mut builder = MsiBuilder::new();
    builder.set_title("File Install Test");
    builder.set_author("Velocity");
    builder.set_subject("Test Product");
    builder.set_template("Intel", 1033);

    let pc = make_uuid();
    let uc = make_uuid();
    println!("ProductCode: {}", pc);
    println!("UpgradeCode: {}", uc);

    // === Property table ===
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().localizable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("File Install Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity")],
        vec![Value::from("ProductCode"), Value::from(pc.as_str())],
        vec![Value::from("UpgradeCode"), Value::from(uc.as_str())],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    // === Directory table ===
    // Use LocalAppDataFolder for per-user install (no admin needed)
    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).nullable().localizable().build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("LocalAppDataFolder"), Value::from("TARGETDIR"), Value::from("LocalAppData")],
        vec![Value::from("INSTALLDIR"), Value::from("LocalAppDataFolder"), Value::from("VelTest:VelTest")],
    ]).unwrap();
    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(38).nullable().build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Component", vec![
        vec![Value::from("comp_0"), Value::Null, Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::from("file_0")],
        vec![Value::from("comp_1"), Value::Null, Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::from("file_1")],
    ]).unwrap();

    // === File table (8 columns) ===
    builder.create_table("File", vec![
        Column::build("File").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).build(),
        Column::build("FileSize").int32().build(),
        Column::build("Version").string(72).nullable().build(),
        Column::build("Language").int16().nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
        Column::build("Sequence").int32().build(),
    ]).unwrap();
    builder.insert_rows("File", vec![
        vec![
            Value::from("file_0"),
            Value::from("comp_0"),
            Value::from("test1.txt"),
            Value::Int(file1_content.len() as i32),
            Value::Null, Value::Null, Value::Int(0),
            Value::Int(1),
        ],
        vec![
            Value::from("file_1"),
            Value::from("comp_1"),
            Value::from("test2.txt"),
            Value::Int(file2_content.len() as i32),
            Value::Null, Value::Null, Value::Int(0),
            Value::Int(2),
        ],
    ]).unwrap();

    // === Feature table ===
    builder.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").string(38).nullable().build(),
        Column::build("Title").string(64).nullable().localizable().build(),
        Column::build("Description").string(255).nullable().localizable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().build(),
    ]).unwrap();
    builder.insert_rows("Feature", vec![
        vec![Value::from("Complete"), Value::Null, Value::from("Setup"), Value::Null, Value::Null, Value::Int(1), Value::Null, Value::Int(0)],
    ]).unwrap();

    // === FeatureComponents table ===
    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("FeatureComponents", vec![
        vec![Value::from("Complete"), Value::from("comp_0")],
        vec![Value::from("Complete"), Value::from("comp_1")],
    ]).unwrap();

    // === Media table ===
    builder.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int16().build(),
        Column::build("DiskPrompt").string(64).nullable().localizable().build(),
        Column::build("Cabinet").string(255).nullable().build(),
        Column::build("VolumeLabel").string(32).nullable().localizable().build(),
        Column::build("Source").string(72).nullable().localizable().build(),
    ]).unwrap();
    builder.insert_rows("Media", vec![
        vec![Value::Int(1), Value::Int(2), Value::Null, Value::from("#Test.cab"), Value::Null, Value::Null],
    ]).unwrap();

    // === InstallExecuteSequence ===
    builder.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("InstallValidate"), Value::Null, Value::Int(1400)],
        vec![Value::from("InstallInitialize"), Value::Null, Value::Int(1500)],
        vec![Value::from("InstallFiles"), Value::Null, Value::Int(2000)],
        vec![Value::from("RegisterProduct"), Value::Null, Value::Int(6100)],
        vec![Value::from("PublishFeatures"), Value::Null, Value::Int(6300)],
        vec![Value::from("PublishProduct"), Value::Null, Value::Int(6400)],
        vec![Value::from("InstallFinalize"), Value::Null, Value::Int(6600)],
    ]).unwrap();

    // === InstallUISequence ===
    builder.create_table("InstallUISequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("InstallUISequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("ExecuteAction"), Value::Null, Value::Int(1300)],
    ]).unwrap();

    // Embed the real cabinet
    builder.add_stream("Test.cab".to_string(), cab_data);

    // Build the MSI
    let msi_data = builder.build().unwrap();
    std::fs::write(out_path, &msi_data).unwrap();
    println!("Wrote: {} ({} bytes)", out_path, msi_data.len());

    // List streams
    println!("\n--- Streams ---");
    let mut comp = cfb::CompoundFile::open(std::io::Cursor::new(&msi_data)).unwrap();
    let stream_names: Vec<String> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.name().to_string())
        .collect();
    for name in &stream_names {
        let stream = comp.open_stream(name).unwrap();
        println!("  {} ({} bytes)", name, stream.len());
    }
    drop(comp);

    // Test with msiexec
    println!("\n--- msiexec install ---");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", log_path, "/norestart"])
        .output()
        .unwrap();
    let code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", code);

    if code == 0 {
        println!("\nSUCCESS! Checking installed files...");
        // Check if files were actually installed
        let f1 = format!("{}\\test1.txt", install_dir);
        let f2 = format!("{}\\test2.txt", install_dir);

        for path in &[&f1, &f2] {
            if std::path::Path::new(path).exists() {
                let content = std::fs::read_to_string(path).unwrap();
                println!("  FOUND: {} ({} bytes)", path, content.len());
            } else {
                println!("  MISSING: {}", path);
            }
        }

        // Search more broadly
        println!("\nSearching for installed files...");
        if std::path::Path::new(&install_dir).exists() {
            println!("  Dir exists: {}", install_dir);
            if let Ok(entries) = std::fs::read_dir(&install_dir) {
                for entry in entries.flatten() {
                    println!("    {}", entry.path().display());
                }
            }
        } else {
            println!("  Install dir does not exist: {}", install_dir);
        }

        // Uninstall using MSI path
        println!("\n--- msiexec uninstall ---");
        let output = std::process::Command::new("msiexec")
            .args(&["/x", out_path, "/qn", "/norestart"])
            .output()
            .unwrap();
        let code = output.status.code().unwrap_or(-1);
        println!("Uninstall exit code: {}", code);
        if code != 0 {
            // Try via product code
            println!("Trying uninstall via product code...");
            let output2 = std::process::Command::new("msiexec")
                .args(&["/x", &pc, "/qn", "/norestart"])
                .output()
                .unwrap();
            let code2 = output2.status.code().unwrap_or(-1);
            println!("Uninstall via product code exit: {}", code2);
        }

        // Verify files removed
        if std::path::Path::new(&install_dir).exists() {
            let remaining: Vec<_> = std::fs::read_dir(&install_dir)
                .map(|e| e.filter_map(|e| e.ok()).map(|e| e.path().display().to_string()).collect())
                .unwrap_or_default();
            if remaining.is_empty() {
                println!("Files removed successfully (empty dir)");
            } else {
                println!("Remaining files: {:?}", remaining);
            }
        } else {
            println!("Install dir removed successfully!");
        }
    } else {
        println!("FAILED: {}", code);
        // Read log for errors
        if let Ok(log) = std::fs::read_to_string(log_path) {
            println!("\n--- Log highlights ---");
            for line in log.lines() {
                if line.contains("return value 3")
                    || line.contains("Error")
                    || line.contains("2219")
                    || line.contains("2203")
                    || line.contains("2705")
                    || line.contains("cabinet")
                    || line.contains("Cabinet")
                {
                    println!("  {}", line.trim());
                }
            }
        }
    }

    println!("\n=== DONE ===");
}
