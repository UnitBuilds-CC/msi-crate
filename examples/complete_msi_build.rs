/// Build a COMPLETE installable MSI with Directory/Component/File/Feature/Media tables
/// and an embedded cabinet file. Tests the full install chain.
/// cargo run --example complete_msi_build
use std::io::{Cursor, Write};

fn main() {
    println!("=== COMPLETE MSI BUILD TEST ===\n");

    let template_path = "C:\\Program Files\\Microsoft Office\\root\\Integration\\C2RIntLoc.en-us.16.msi";
    if !std::path::Path::new(template_path).exists() {
        println!("Template MSI not found at {}", template_path);
        return;
    }

    // Step 1: Create test files to install
    let test_dir = "C:\\temp\\velo_test_src";
    let _ = std::fs::create_dir_all(test_dir);
    std::fs::write(format!("{}\\hello.txt", test_dir), "Hello from Velocity Installer!\n").unwrap();
    std::fs::write(format!("{}\\readme.md", test_dir), "# Velocity\nA fast installer.\n").unwrap();
    println!("Created test files in {}", test_dir);

    // Step 2: Create a cabinet file containing the test files
    let cab_path = "C:\\temp\\velo_data.cab";
    create_cabinet(cab_path, test_dir, &["hello.txt", "readme.md"]);
    let cab_data = std::fs::read(cab_path).unwrap();
    println!("Cabinet: {} bytes", cab_data.len());

    // Step 3: Open template MSI and build our package
    let template_data = std::fs::read(template_path).unwrap();
    let cursor = Cursor::new(template_data);
    let mut pkg = msi::Package::open(cursor).unwrap();

    // Delete all existing user tables
    let user_tables: Vec<String> = pkg.tables()
        .map(|t| t.name().to_string())
        .filter(|n| !n.starts_with('_'))
        .collect();
    for table_name in &user_tables {
        pkg.drop_table(table_name).unwrap();
    }
    println!("Cleared {} user tables", user_tables.len());

    // Product GUIDs
    let product_code = "A1B2C3D4-E5F6-7890-ABCD-EF1234567890";

    // Step 4: Create Property table
    {
        let columns = vec![
            msi::Column::build("Property").primary_key().string(72),
            msi::Column::build("Value").nullable().localizable().string(255),
        ];
        pkg.create_table("Property".to_string(), columns).unwrap();

        pkg.insert_rows(msi::Insert::into("Property").rows(vec![
            vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Velocity Test App".into())],
            vec![msi::Value::Str("ProductCode".into()), msi::Value::Str(format!("{{{}}}", product_code))],
            vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())],
            vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Velocity Team".into())],
            vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())],
            vec![msi::Value::Str("UpgradeCode".into()), msi::Value::Str("{B2C3D4E5-F6A7-8901-BCDE-F12345678901}".into())],
        ])).unwrap();
        println!("Created Property table (6 rows)");
    }

    // Step 5: Create Directory table
    {
        let columns = vec![
            msi::Column::build("Directory").primary_key().string(72),
            msi::Column::build("Directory_Parent").nullable().string(72),
            msi::Column::build("DefaultDir").localizable().string(255),
        ];
        pkg.create_table("Directory".to_string(), columns).unwrap();

        pkg.insert_rows(msi::Insert::into("Directory").rows(vec![
            vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str(".".into())],
            vec![msi::Value::Str("ProgramFilesFolder".into()), msi::Value::Str("TARGETDIR".into()), msi::Value::Str("PFiles".into())],
            vec![msi::Value::Str("INSTALLDIR".into()), msi::Value::Str("ProgramFilesFolder".into()), msi::Value::Str("VelocityTest".into())],
        ])).unwrap();
        println!("Created Directory table (3 rows)");
    }

    // Step 6: Create Component table
    {
        let columns = vec![
            msi::Column::build("Component").primary_key().string(72),
            msi::Column::build("ComponentId").nullable().string(38),
            msi::Column::build("Directory_").string(72),
            msi::Column::build("Attributes").nullable().int16(),
            msi::Column::build("Condition").nullable().string(255),
            msi::Column::build("KeyPath").nullable().string(72),
        ];
        pkg.create_table("Component".to_string(), columns).unwrap();

        pkg.insert_rows(msi::Insert::into("Component").rows(vec![
            vec![
                msi::Value::Str("MainComponent".into()),
                msi::Value::Str("{C3D4E5F6-A7B8-9012-CDEF-123456789012}".into()),
                msi::Value::Str("INSTALLDIR".into()),
                msi::Value::Int(0),
                msi::Value::Null,
                msi::Value::Str("hello.txt".into()),
            ],
        ])).unwrap();
        println!("Created Component table (1 row)");
    }

    // Step 7: Create File table
    {
        let columns = vec![
            msi::Column::build("File").primary_key().string(72),
            msi::Column::build("Component_").string(72),
            msi::Column::build("FileName").localizable().string(255),
            msi::Column::build("FileSize").int32(),
            msi::Column::build("Attributes").nullable().int16(),
            msi::Column::build("Sequence").int16(),
        ];
        pkg.create_table("File".to_string(), columns).unwrap();

        let hello_size = std::fs::metadata(format!("{}\\hello.txt", test_dir)).unwrap().len() as i32;
        let readme_size = std::fs::metadata(format!("{}\\readme.md", test_dir)).unwrap().len() as i32;

        pkg.insert_rows(msi::Insert::into("File").rows(vec![
            vec![
                msi::Value::Str("hello.txt".into()),
                msi::Value::Str("MainComponent".into()),
                msi::Value::Str("hello.txt".into()),
                msi::Value::Int(hello_size),
                msi::Value::Int(0),
                msi::Value::Int(1),
            ],
            vec![
                msi::Value::Str("readme.md".into()),
                msi::Value::Str("MainComponent".into()),
                msi::Value::Str("readme.md".into()),
                msi::Value::Int(readme_size),
                msi::Value::Int(0),
                msi::Value::Int(2),
            ],
        ])).unwrap();
        println!("Created File table (2 rows)");
    }

    // Step 8: Create Feature table
    {
        let columns = vec![
            msi::Column::build("Feature").primary_key().string(38),
            msi::Column::build("Feature_Parent").nullable().string(38),
            msi::Column::build("Title").nullable().localizable().string(64),
            msi::Column::build("Description").nullable().localizable().string(255),
            msi::Column::build("Display").nullable().int16(),
            msi::Column::build("Level").int16(),
            msi::Column::build("Directory_").nullable().string(72),
            msi::Column::build("Attributes").nullable().int16(),
        ];
        pkg.create_table("Feature".to_string(), columns).unwrap();

        pkg.insert_rows(msi::Insert::into("Feature").rows(vec![
            vec![
                msi::Value::Str("Complete".into()),
                msi::Value::Null,
                msi::Value::Str("Complete Installation".into()),
                msi::Value::Str("Install all features".into()),
                msi::Value::Null,
                msi::Value::Int(1),
                msi::Value::Str("INSTALLDIR".into()),
                msi::Value::Null,
            ],
        ])).unwrap();
        println!("Created Feature table (1 row)");
    }

    // Step 9: Create FeatureComponents table
    {
        let columns = vec![
            msi::Column::build("Feature_").primary_key().string(38),
            msi::Column::build("Component_").primary_key().string(72),
        ];
        pkg.create_table("FeatureComponents".to_string(), columns).unwrap();

        pkg.insert_rows(msi::Insert::into("FeatureComponents").rows(vec![
            vec![
                msi::Value::Str("Complete".into()),
                msi::Value::Str("MainComponent".into()),
            ],
        ])).unwrap();
        println!("Created FeatureComponents table (1 row)");
    }

    // Step 10: Create Media table (embedded cabinet)
    {
        let columns = vec![
            msi::Column::build("DiskId").primary_key().int16(),
            msi::Column::build("LastSequence").int16(),
            msi::Column::build("Cabinet").nullable().string(255),
            msi::Column::build("VolumeLabel").nullable().string(32),
            msi::Column::build("Source").nullable().string(72),
        ];
        pkg.create_table("Media".to_string(), columns).unwrap();

        pkg.insert_rows(msi::Insert::into("Media").rows(vec![
            vec![
                msi::Value::Int(1),
                msi::Value::Int(2),
                msi::Value::Str("#velo_data.cab".into()),
                msi::Value::Null,
                msi::Value::Null,
            ],
        ])).unwrap();
        println!("Created Media table (1 row)");
    }

    // Step 11: Create InstallExecuteSequence table
    {
        let columns = vec![
            msi::Column::build("Action").primary_key().string(72),
            msi::Column::build("Condition").nullable().string(255),
            msi::Column::build("Sequence").nullable().int16(),
        ];
        pkg.create_table("InstallExecuteSequence".to_string(), columns).unwrap();

        pkg.insert_rows(msi::Insert::into("InstallExecuteSequence").rows(vec![
            vec![msi::Value::Str("LaunchConditions".into()), msi::Value::Null, msi::Value::Int(100)],
            vec![msi::Value::Str("ValidateProductID".into()), msi::Value::Null, msi::Value::Int(700)],
            vec![msi::Value::Str("CostFinalize".into()), msi::Value::Null, msi::Value::Int(1000)],
            vec![msi::Value::Str("InstallValidate".into()), msi::Value::Null, msi::Value::Int(1400)],
            vec![msi::Value::Str("InstallInitialize".into()), msi::Value::Null, msi::Value::Int(1500)],
            vec![msi::Value::Str("ProcessComponents".into()), msi::Value::Null, msi::Value::Int(1600)],
            vec![msi::Value::Str("UnpublishComponents".into()), msi::Value::Null, msi::Value::Int(1700)],
            vec![msi::Value::Str("UnpublishFeatures".into()), msi::Value::Null, msi::Value::Int(1800)],
            vec![msi::Value::Str("RegisterProduct".into()), msi::Value::Null, msi::Value::Int(5700)],
            vec![msi::Value::Str("PublishFeatures".into()), msi::Value::Null, msi::Value::Int(6300)],
            vec![msi::Value::Str("PublishProduct".into()), msi::Value::Null, msi::Value::Int(6400)],
            vec![msi::Value::Str("InstallFinalize".into()), msi::Value::Null, msi::Value::Int(6600)],
        ])).unwrap();
        println!("Created InstallExecuteSequence table (12 rows)");
    }

    // Step 12: Create InstallUISequence table
    {
        let columns = vec![
            msi::Column::build("Action").primary_key().string(72),
            msi::Column::build("Condition").nullable().string(255),
            msi::Column::build("Sequence").nullable().int16(),
        ];
        pkg.create_table("InstallUISequence".to_string(), columns).unwrap();

        pkg.insert_rows(msi::Insert::into("InstallUISequence").rows(vec![
            vec![msi::Value::Str("LaunchConditions".into()), msi::Value::Null, msi::Value::Int(100)],
            vec![msi::Value::Str("ValidateProductID".into()), msi::Value::Null, msi::Value::Int(700)],
            vec![msi::Value::Str("CostFinalize".into()), msi::Value::Null, msi::Value::Int(1000)],
        ])).unwrap();
        println!("Created InstallUISequence table (3 rows)");
    }

    // Set summary info
    pkg.summary_info_mut().set_title("Velocity Test App");
    pkg.summary_info_mut().set_author("Velocity Team");
    pkg.summary_info_mut().set_subject("Velocity Test Application");
    pkg.summary_info_mut().set_arch("x64");
    pkg.summary_info_mut().set_languages(&[msi::Language::from_code(1033)]);
    pkg.summary_info_mut().set_uuid(uuid::Uuid::parse_str(product_code).unwrap());
    pkg.summary_info_mut().set_creating_application("Velocity Installer");
    pkg.summary_info_mut().set_word_count(2);
    pkg.summary_info_mut().set_creation_time_to_now();

    // Save the MSI
    pkg.flush().unwrap();
    let cursor = pkg.into_inner().unwrap();
    let mut msi_data = cursor.into_inner().to_vec();
    println!("\nMSI before cabinet embed: {} bytes", msi_data.len());

    // Step 13: Write MSI to disk, then reopen to embed cabinet
    let out_path = "C:\\temp\\velocity_complete.msi";
    std::fs::write(out_path, &msi_data).unwrap();
    println!("Wrote {} to {} (before cabinet)", msi_data.len(), out_path);

    {
        let file = std::fs::OpenOptions::new()
            .read(true).write(true).open(out_path).unwrap();
        let mut comp = cfb::CompoundFile::open(file).unwrap();
        
        let existing: Vec<String> = comp.walk().map(|e| e.name().to_string()).collect();
        println!("Existing streams: {} entries", existing.len());
        for name in &existing {
            let safe: String = name.chars().map(|c| if c.is_ascii() && !c.is_control() { c } else { '.' }).collect();
            println!("  {}", safe);
        }
        
        let mut cab_stream = comp.create_stream("velo_data.cab").unwrap();
        cab_stream.write_all(&cab_data).unwrap();
        drop(cab_stream);
        
        comp.flush().unwrap();
        println!("Cabinet embedded ({} bytes)", cab_data.len());
    }
    let final_size = std::fs::metadata(out_path).unwrap().len();
    println!("MSI final size: {} bytes", final_size);

    // Clean up any previous install
    let product_code_braced = format!("{{{}}}", product_code);
    let _ = std::process::Command::new("msiexec")
        .args(&["/x", &product_code_braced, "/qn"])
        .output();
    std::thread::sleep(std::time::Duration::from_secs(2));

    let install_dir = "C:\\Program Files\\VelocityTest";
    let _ = std::fs::remove_dir_all(install_dir);

    println!("\n--- msiexec install test ---");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", "C:\\temp\\velocity_complete.log"])
        .output()
        .expect("Failed to run msiexec");
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);
    match exit_code {
        0 => println!("SUCCESS! MSI installed!"),
        1603 => println!("1603 (fatal error during install)"),
        1605 => println!("1605 (not installed - but data readable)"),
        1613 => println!("1613 (invalid package)"),
        1620 => println!("1620 (could not open)"),
        _ => println!("Error code {}", exit_code),
    }

    // Check if files were installed
    if std::path::Path::new(&format!("{}\\hello.txt", install_dir)).exists() {
        println!("FILE CHECK: hello.txt INSTALLED!");
        let content = std::fs::read_to_string(format!("{}\\hello.txt", install_dir)).unwrap();
        println!("  Content: {:?}", content.trim());
    } else {
        println!("FILE CHECK: hello.txt NOT found at {}", install_dir);
    }
    if std::path::Path::new(&format!("{}\\readme.md", install_dir)).exists() {
        println!("FILE CHECK: readme.md INSTALLED!");
    } else {
        println!("FILE CHECK: readme.md NOT found");
    }

    // Read log for errors
    let log_path = "C:\\temp\\velocity_complete.log";
    if let Ok(log) = std::fs::read_to_string(log_path) {
        let lines: Vec<&str> = log.lines().collect();
        println!("\n--- Log highlights ---");
        for line in &lines {
            if line.contains("Error") || line.contains("error") ||
               line.contains("return value 3") || line.contains("2219") ||
               line.contains("Product:") || line.contains("Installation successful") ||
               line.contains("cabinet") || line.contains("Cabinet") {
                println!("  {}", line);
            }
        }
        let start = if lines.len() > 20 { lines.len() - 20 } else { 0 };
        println!("\n--- Log (last 20 lines) ---");
        for line in &lines[start..] {
            println!("  {}", line);
        }
    }

    // Try uninstall if install succeeded
    if exit_code == 0 {
        println!("\n--- msiexec uninstall test ---");
        let output = std::process::Command::new("msiexec")
            .args(&["/x", &product_code_braced, "/qn", "/l*v", "C:\\temp\\velocity_uninstall.log"])
            .output()
            .expect("Failed to run msiexec");
        let uninst_code = output.status.code().unwrap_or(-1);
        println!("Uninstall exit code: {}", uninst_code);

        std::thread::sleep(std::time::Duration::from_secs(1));
        if !std::path::Path::new(&format!("{}\\hello.txt", install_dir)).exists() {
            println!("UNINSTALL CHECK: hello.txt REMOVED!");
        } else {
            println!("UNINSTALL CHECK: hello.txt STILL EXISTS");
        }
    }

    println!("\n=== DONE ===");
}

/// Create a cabinet file using Windows makecab.exe
fn create_cabinet(cab_path: &str, source_dir: &str, files: &[&str]) {
    let ddf_path = "C:\\temp\\velo_cabinet.ddf";
    let mut ddf = String::new();
    ddf.push_str(&format!(".Set CabinetName1={}\n", "velo_data.cab"));
    ddf.push_str(".Set DiskDirectory1=C:\\temp\n");
    ddf.push_str(".Set CompressionType=MSZIP\n");
    ddf.push_str(".Set Compress=ON\n");

    for file in files {
        ddf.push_str(&format!("\"{}\\{}\"\n", source_dir, file));
    }

    std::fs::write(ddf_path, &ddf).unwrap();

    let output = std::process::Command::new("makecab")
        .args(&["/f", ddf_path])
        .output()
        .expect("Failed to run makecab");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!("makecab failed: {} {}", stdout, stderr);
    }

    let _ = std::fs::remove_file(ddf_path);
    let _ = std::fs::remove_file("C:\\temp\\setup.inf");
    let _ = std::fs::remove_file("C:\\temp\\setup.rpt");
}
