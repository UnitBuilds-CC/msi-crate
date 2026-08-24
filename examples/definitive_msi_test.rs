/// Definitive test: build a COMPLETE installable MSI using velocity-msi from scratch.
/// Tests the full pipeline: table creation, cabinet embedding, msiexec install/uninstall.
///
/// cargo run --example definitive_msi_test
use std::io::Write;
use velocity_msi::{MsiBuilder, Column, Value};

fn main() {
    println!("=== DEFINITIVE MSI TEST (velocity-msi from scratch) ===\n");

    // Kill any stale msiexec processes
    let _ = std::process::Command::new("taskkill")
        .args(&["/F", "/IM", "msiexec.exe"])
        .output();
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Step 1: Create test files
    let test_dir = "C:\\temp\\velo_def_src";
    let _ = std::fs::create_dir_all(test_dir);
    std::fs::write(format!("{}\\hello.txt", test_dir), "Hello from Velocity Installer!\n").unwrap();
    std::fs::write(format!("{}\\readme.md", test_dir), "# Velocity\nA fast installer.\n").unwrap();
    println!("[1] Created test files in {}", test_dir);

    // Step 2: Create cabinet
    let cab_path = "C:\\temp\\velo_data.cab";
    let _ = std::fs::remove_file(cab_path);
    create_cabinet(cab_path, test_dir, &["hello.txt", "readme.md"]);
    let cab_data = std::fs::read(cab_path).unwrap();
    println!("[2] Cabinet: {} bytes", cab_data.len());

    // Step 3: Build MSI using velocity-msi
    let mut builder = MsiBuilder::new();
    builder.set_title("Velocity Test App");
    builder.set_author("Velocity Team");
    builder.set_subject("Velocity Test Application");
    builder.set_template("x64", 1033);

    let product_code = "{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}";
    let upgrade_code = "{B2C3D4E5-F6A7-8901-BCDE-F12345678901}";
    let component_id = "{C3D4E5F6-A7B8-9012-CDEF-123456789012}";

    // --- Property table ---
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().localizable().build(),
    ]).unwrap();

    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Velocity Test App")],
        vec![Value::from("ProductCode"), Value::from(product_code)],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity Team")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
        vec![Value::from("UpgradeCode"), Value::from(upgrade_code)],
    ]).unwrap();
    println!("[3a] Property table: 6 rows");

    // --- Directory table ---
    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).localizable().build(),
    ]).unwrap();

    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from(".")],
        vec![Value::from("ProgramFilesFolder"), Value::from("TARGETDIR"), Value::from("PFiles")],
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFilesFolder"), Value::from("VelocityTest")],
    ]).unwrap();
    println!("[3b] Directory table: 3 rows");

    // --- Component table ---
    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(38).nullable().build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().nullable().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();

    builder.insert_rows("Component", vec![
        vec![
            Value::from("MainComponent"),
            Value::from(component_id),
            Value::from("INSTALLDIR"),
            Value::Int(0),
            Value::Null,
            Value::from("hello.txt"),
        ],
    ]).unwrap();
    println!("[3c] Component table: 1 row");

    // --- File table ---
    let hello_size = std::fs::metadata(format!("{}\\hello.txt", test_dir)).unwrap().len() as i32;
    let readme_size = std::fs::metadata(format!("{}\\readme.md", test_dir)).unwrap().len() as i32;

    builder.create_table("File", vec![
        Column::build("File").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).localizable().build(),
        Column::build("FileSize").int32().build(),
        Column::build("Attributes").int16().nullable().build(),
        Column::build("Sequence").int16().build(),
    ]).unwrap();

    builder.insert_rows("File", vec![
        vec![
            Value::from("hello.txt"),
            Value::from("MainComponent"),
            Value::from("hello.txt"),
            Value::Int(hello_size),
            Value::Int(0),
            Value::Int(1),
        ],
        vec![
            Value::from("readme.md"),
            Value::from("MainComponent"),
            Value::from("readme.md"),
            Value::Int(readme_size),
            Value::Int(0),
            Value::Int(2),
        ],
    ]).unwrap();
    println!("[3d] File table: 2 rows");

    // --- Feature table ---
    builder.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").string(38).nullable().build(),
        Column::build("Title").string(64).nullable().localizable().build(),
        Column::build("Description").string(255).nullable().localizable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
    ]).unwrap();

    builder.insert_rows("Feature", vec![
        vec![
            Value::from("Complete"),
            Value::Null,
            Value::from("Complete Installation"),
            Value::from("Install all features"),
            Value::Null,
            Value::Int(1),
            Value::from("INSTALLDIR"),
            Value::Null,
        ],
    ]).unwrap();
    println!("[3e] Feature table: 1 row");

    // --- FeatureComponents table ---
    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();

    builder.insert_rows("FeatureComponents", vec![
        vec![Value::from("Complete"), Value::from("MainComponent")],
    ]).unwrap();
    println!("[3f] FeatureComponents table: 1 row");

    // --- Media table ---
    builder.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int16().build(),
        Column::build("Cabinet").string(255).nullable().build(),
        Column::build("VolumeLabel").string(32).nullable().build(),
        Column::build("Source").string(72).nullable().build(),
    ]).unwrap();

    builder.insert_rows("Media", vec![
        vec![
            Value::Int(1),
            Value::Int(2),
            Value::from("#velo_data.cab"),
            Value::Null,
            Value::Null,
        ],
    ]).unwrap();
    println!("[3g] Media table: 1 row");

    // --- InstallExecuteSequence table ---
    builder.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();

    builder.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("LaunchConditions"), Value::Null, Value::Int(100)],
        vec![Value::from("ValidateProductID"), Value::Null, Value::Int(700)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("InstallValidate"), Value::Null, Value::Int(1400)],
        vec![Value::from("InstallInitialize"), Value::Null, Value::Int(1500)],
        vec![Value::from("ProcessComponents"), Value::Null, Value::Int(1600)],
        vec![Value::from("UnpublishComponents"), Value::Null, Value::Int(1700)],
        vec![Value::from("UnpublishFeatures"), Value::Null, Value::Int(1800)],
        vec![Value::from("RegisterProduct"), Value::Null, Value::Int(5700)],
        vec![Value::from("PublishFeatures"), Value::Null, Value::Int(6300)],
        vec![Value::from("PublishProduct"), Value::Null, Value::Int(6400)],
        vec![Value::from("InstallFinalize"), Value::Null, Value::Int(6600)],
    ]).unwrap();
    println!("[3h] InstallExecuteSequence table: 12 rows");

    // --- InstallUISequence table ---
    builder.create_table("InstallUISequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();

    builder.insert_rows("InstallUISequence", vec![
        vec![Value::from("LaunchConditions"), Value::Null, Value::Int(100)],
        vec![Value::from("ValidateProductID"), Value::Null, Value::Int(700)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
    ]).unwrap();
    println!("[3i] InstallUISequence table: 3 rows");

    // --- Build the MSI ---
    let mut msi_data = builder.build().unwrap();
    println!("\n[4] MSI built: {} bytes", msi_data.len());

    // Step 5: Embed cabinet into the MSI using cfb crate
    let out_path = "C:\\temp\\velocity_definitive.msi";
    std::fs::write(out_path, &msi_data).unwrap();
    println!("[5] Wrote MSI to {}", out_path);

    // Open with cfb and add cabinet stream
    {
        let file = std::fs::OpenOptions::new()
            .read(true).write(true).open(out_path).unwrap();
        let mut comp = cfb::CompoundFile::open(file).unwrap();

        // List existing streams
        let existing: Vec<String> = comp.walk().map(|e| e.name().to_string()).collect();
        println!("[6] Existing streams: {} entries", existing.len());
        for name in &existing {
            let safe: String = name.chars()
                .map(|c| if c.is_ascii() && !c.is_control() { c } else { '.' })
                .collect();
            println!("  {}", safe);
        }

        // Add cabinet stream (name must match Media table entry without the # prefix)
        let mut cab_stream = comp.create_stream("velo_data.cab").unwrap();
        cab_stream.write_all(&cab_data).unwrap();
        drop(cab_stream);

        comp.flush().unwrap();
        println!("[6] Cabinet embedded ({} bytes)", cab_data.len());
    }

    let final_size = std::fs::metadata(out_path).unwrap().len();
    println!("[7] Final MSI size: {} bytes", final_size);

    // Step 8: Clean up any previous install
    let _ = std::process::Command::new("msiexec")
        .args(&["/x", product_code, "/qn"])
        .output();
    std::thread::sleep(std::time::Duration::from_secs(2));

    let install_dir = "C:\\Program Files\\VelocityTest";
    let _ = std::fs::remove_dir_all(install_dir);

    // Step 9: Test with msiexec
    println!("\n=== msiexec install test ===");
    let log_path = "C:\\temp\\velocity_definitive.log";
    let _ = std::fs::remove_file(log_path);

    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", log_path])
        .output()
        .expect("Failed to run msiexec");
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);
    match exit_code {
        0 => println!("SUCCESS! MSI installed!"),
        1603 => println!("1603 (fatal error during install)"),
        1605 => println!("1605 (not installed)"),
        1613 => println!("1613 (invalid package)"),
        1619 => println!("1619 (invalid installation package)"),
        1620 => println!("1620 (could not open)"),
        _ => println!("Error code {}", exit_code),
    }

    // Check files
    for fname in &["hello.txt", "readme.md"] {
        let fpath = format!("{}\\{}", install_dir, fname);
        if std::path::Path::new(&fpath).exists() {
            println!("FILE: {} INSTALLED!", fname);
        } else {
            println!("FILE: {} NOT found", fname);
        }
    }

    // Read log
    if let Ok(log) = std::fs::read_to_string(log_path) {
        let lines: Vec<&str> = log.lines().collect();
        println!("\n--- Log highlights ---");
        for line in &lines {
            if line.contains("Error") || line.contains("error") ||
               line.contains("return value 3") || line.contains("2219") ||
               line.contains("Product:") || line.contains("Installation successful") ||
               line.contains("cabinet") || line.contains("Cabinet") ||
               line.contains("string pool") || line.contains("codepage") {
                println!("  {}", line);
            }
        }
        let start = if lines.len() > 30 { lines.len() - 30 } else { 0 };
        println!("\n--- Log (last 30 lines) ---");
        for line in &lines[start..] {
            println!("  {}", line);
        }
    }

    // Uninstall if succeeded
    if exit_code == 0 {
        println!("\n=== msiexec uninstall test ===");
        let output = std::process::Command::new("msiexec")
            .args(&["/x", product_code, "/qn", "/l*v", "C:\\temp\\velocity_def_uninstall.log"])
            .output()
            .expect("Failed to run msiexec");
        let uninst_code = output.status.code().unwrap_or(-1);
        println!("Uninstall exit code: {}", uninst_code);

        std::thread::sleep(std::time::Duration::from_secs(1));
        for fname in &["hello.txt", "readme.md"] {
            let fpath = format!("{}\\{}", install_dir, fname);
            if !std::path::Path::new(&fpath).exists() {
                println!("UNINSTALL: {} REMOVED!", fname);
            } else {
                println!("UNINSTALL: {} STILL EXISTS", fname);
            }
        }
    }

    println!("\n=== DONE ===");
}

fn create_cabinet(cab_path: &str, source_dir: &str, files: &[&str]) {
    let ddf_path = "C:\\temp\\velo_def_cabinet.ddf";
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
