//! Create a reference MSI using PowerShell COM API, then compare with velocity-msi
//! 
//! This test:
//! 1. Creates a minimal installable MSI via PowerShell COM
//! 2. Reads it back with msi crate
//! 3. Compares _Columns, _Validation, string pool with velocity-msi output

use std::io::{Cursor, Write};
use std::process::Command;
use velocity_msi::{MsiBuilder, Column, Value};

fn main() {
    // Step 1: Create reference MSI via PowerShell
    let ps_script = r#"
$wi = New-Object -ComObject WindowsInstaller.Installer
$db = $wi.CreateDatabase("C:\temp\ref_msi_test\reference.msi", 2)

# Create Property table
$db.CreateTable("Property", "Property S72, Value S255")
# Insert properties
$view = $db.OpenView("INSERT INTO Property (Property, Value) VALUES ('ProductName', 'RefTest')")
$view.Execute()
$view = $db.OpenView("INSERT INTO Property (Property, Value) VALUES ('ProductCode', '{12345678-1234-1234-1234-123456789012}')")
$view.Execute()
$view = $db.OpenView("INSERT INTO Property (Property, Value) VALUES ('ProductVersion', '1.0.0')")
$view.Execute()
$view = $db.OpenView("INSERT INTO Property (Property, Value) VALUES ('ProductLanguage', '1033')")
$view.Execute()
$view = $db.OpenView("INSERT INTO Property (Property, Value) VALUES ('Manufacturer', 'RefCo')")
$view.Execute()
$view = $db.OpenView("INSERT INTO Property (Property, Value) VALUES ('UpgradeCode', '{87654321-4321-4321-4321-210987654321}')")
$view.Execute()

# Create Directory table
$db.CreateTable("Directory", "Directory S72, Directory_Parent S72, DefaultDir S255")
$view = $db.OpenView("INSERT INTO Directory (Directory, Directory_Parent, DefaultDir) VALUES ('TARGETDIR', '', 'SourceDir')")
$view.Execute()
$view = $db.OpenView("INSERT INTO Directory (Directory, Directory_Parent, DefaultDir) VALUES ('ProgramFilesFolder', 'TARGETDIR', 'PFiles')")
$view.Execute()
$view = $db.OpenView("INSERT INTO Directory (Directory, Directory_Parent, DefaultDir) VALUES ('INSTALLDIR', 'ProgramFilesFolder', 'RefTest')")
$view.Execute()

# Create Component table
$db.CreateTable("Component", "Component S72, ComponentId S38, Directory_ S72, Attributes I2, Condition S255, KeyPath S72")
$view = $db.OpenView("INSERT INTO Component (Component, ComponentId, Directory_, Attributes) VALUES ('MainComp', '{AAAAAAAA-1111-1111-1111-AAAAAAAAAAAA}', 'INSTALLDIR', 0)")
$view.Execute()

# Create Feature table
$db.CreateTable("Feature", "Feature S38, Feature_Parent S38, Title S64, Description S255, Display I2, Level I2, Directory_ S72, Attributes I2")
$view = $db.OpenView("INSERT INTO Feature (Feature, Title, Description, Display, Level, Directory_) VALUES ('MainFeature', 'Main Feature', 'Installs files', 1, 1, 'INSTALLDIR')")
$view.Execute()

# Create FeatureComponents table
$db.CreateTable("FeatureComponents", "Feature_ S38, Component_ S72")
$view = $db.OpenView("INSERT INTO FeatureComponents (Feature_, Component_) VALUES ('MainFeature', 'MainComp')")
$view.Execute()

# Create File table
$db.CreateTable("File", "File S72, Component_ S72, FileName S255, FileSize I4, Attributes I2, Sequence I4")
$view = $db.OpenView("INSERT INTO File (File, Component_, FileName, FileSize, Attributes, Sequence) VALUES ('F1', 'MainComp', 'test.txt', 10, 0, 1)")
$view.Execute()

# Create Media table
$db.CreateTable("Media", "DiskId I2, LastSequence I4, Cabinet S255, DiskPrompt S64")
$view = $db.OpenView("INSERT INTO Media (DiskId, LastSequence, Cabinet) VALUES (1, 1, '#cab1.cab')")
$view.Execute()

# Create InstallExecuteSequence table
$db.CreateTable("InstallExecuteSequence", "Action S72, Condition S255, Sequence I4")
$view = $db.OpenView("INSERT INTO InstallExecuteSequence (Action, Sequence) VALUES ('CostFinalize', 1000)")
$view.Execute()
$view = $db.OpenView("INSERT INTO InstallExecuteSequence (Action, Sequence) VALUES ('InstallFiles', 4000)")
$view.Execute()

# Commit
$db.Commit()

# Set summary info
$summary = $db.SummaryInformation(0)
$summary.Property(1) = "Installation Database"
$summary.Property(2) = "RefTest"
$summary.Property(4) = "RefCo"
$summary.Property(6) = "RefCo"
$summary.Property(7) = "RefTest Product"
$summary.Property(9) = "{GENERATE_GUID}"
$summary.Property(14) = 405
$summary.Property(15) = 2
$summary.Property(19) = 0
$summary.Property(1) = "Installation Database"
$summary.Commit()

Write-Host "Reference MSI created at C:\temp\ref_msi_test\reference.msi"
"#;

    // Create temp directory
    std::fs::create_dir_all("C:\\temp\\ref_msi_test").ok();
    
    // Write PowerShell script
    let script_path = "C:\\temp\\ref_msi_test\\create_ref.ps1";
    std::fs::write(script_path, ps_script).unwrap();
    
    // Run PowerShell
    let output = Command::new("powershell")
        .args(&["-ExecutionPolicy", "Bypass", "-File", script_path])
        .output();
    
    match output {
        Ok(out) => {
            println!("PowerShell stdout: {}", String::from_utf8_lossy(&out.stdout));
            println!("PowerShell stderr: {}", String::from_utf8_lossy(&out.stderr));
            println!("PowerShell exit code: {:?}", out.status.code());
        }
        Err(e) => {
            println!("Failed to run PowerShell: {}", e);
            return;
        }
    }
    
    // Step 2: Read reference MSI with msi crate
    let ref_path = "C:\\temp\\ref_msi_test\\reference.msi";
    if !std::path::Path::new(ref_path).exists() {
        println!("Reference MSI not found at {}", ref_path);
        return;
    }
    
    let ref_data = std::fs::read(ref_path).unwrap();
    println!("\n=== Reference MSI size: {} bytes ===", ref_data.len());
    
    let cursor = Cursor::new(ref_data.clone());
    let mut package = msi::Package::open(cursor).unwrap();
    
    // Read _Columns from reference
    println!("\n=== Reference _Columns for File ===");
    if let Ok(rows) = package.select_rows(msi::Select::table("_Columns")) {
        for row in rows {
            let table_name: String = match &row["Table"] {
                msi::Value::Str(s) => s.clone(),
                _ => continue,
            };
            if table_name == "File" {
                let num = match &row["Number"] {
                    msi::Value::Int(n) => *n,
                    _ => 0,
                };
                let name = match &row["Name"] {
                    msi::Value::Str(s) => s.clone(),
                    _ => "?".to_string(),
                };
                let sql_type = match &row["Type"] {
                    msi::Value::Int(n) => *n,
                    _ => 0,
                };
                println!("  Number={} Name='{}' Type=0x{:04X}", num, name, sql_type);
            }
        }
    }
    
    // Read _Validation from reference
    println!("\n=== Reference _Validation for File ===");
    if let Ok(rows) = package.select_rows(msi::Select::table("_Validation")) {
        for row in rows {
            let table_name: String = match &row["Table"] {
                msi::Value::Str(s) => s.clone(),
                _ => continue,
            };
            if table_name == "File" {
                let col_name = match &row["Column"] {
                    msi::Value::Str(s) => s.clone(),
                    _ => "?".to_string(),
                };
                let nullable = match &row["Nullable"] {
                    msi::Value::Str(s) => s.clone(),
                    _ => "?".to_string(),
                };
                let category = match &row["Category"] {
                    msi::Value::Str(s) => format!("'{}'", s),
                    _ => "Null".to_string(),
                };
                let set = match &row["Set"] {
                    msi::Value::Str(s) => format!("'{}'", s),
                    _ => "Null".to_string(),
                };
                println!("  Col='{}' Nullable={} Category={} Set={}", col_name, nullable, category, set);
            }
        }
    }
    
    // Read _Tables from reference
    println!("\n=== Reference _Tables ===");
    if let Ok(rows) = package.select_rows(msi::Select::table("_Tables")) {
        for row in rows {
            let name = match &row["Table"] {
                msi::Value::Str(s) => s.clone(),
                _ => continue,
            };
            println!("  {}", name);
        }
    }
    
    // Read string pool info
    println!("\n=== Reference Database codepage ===");
    println!("  {:?}", package.database_codepage());
    
    // Step 3: Create velocity-msi version with SAME tables
    println!("\n=== Creating velocity-msi version ===");
    let mut builder = MsiBuilder::new();
    builder.set_title("RefTest");
    builder.set_author("RefCo");
    
    // Property table
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("RefTest")],
        vec![Value::from("ProductCode"), Value::from("{12345678-1234-1234-1234-123456789012}")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
        vec![Value::from("Manufacturer"), Value::from("RefCo")],
        vec![Value::from("UpgradeCode"), Value::from("{87654321-4321-4321-4321-210987654321}")],
    ]).unwrap();
    
    // Directory table
    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("ProgramFilesFolder"), Value::from("TARGETDIR"), Value::from("PFiles")],
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFilesFolder"), Value::from("RefTest")],
    ]).unwrap();
    
    // Component table
    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(38).nullable().build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().nullable().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Component", vec![
        vec![Value::from("MainComp"), Value::from("{AAAAAAAA-1111-1111-1111-AAAAAAAAAAAA}"), Value::from("INSTALLDIR"), Value::from(0i32), Value::Null, Value::Null],
    ]).unwrap();
    
    // Feature table
    builder.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").string(38).nullable().build(),
        Column::build("Title").string(64).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("Feature", vec![
        vec![Value::from("MainFeature"), Value::Null, Value::from("Main Feature"), Value::from("Installs files"), Value::from(1i32), Value::from(1i32), Value::from("INSTALLDIR"), Value::Null],
    ]).unwrap();
    
    // FeatureComponents table
    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("FeatureComponents", vec![
        vec![Value::from("MainFeature"), Value::from("MainComp")],
    ]).unwrap();
    
    // File table
    builder.create_table("File", vec![
        Column::build("File_").string(8).primary_key().build(),
        Column::build("Component_").string(8).build(),
        Column::build("FileName").string(255).build(),
        Column::build("FileSize").int32().build(),
        Column::build("Attributes").int16().nullable().build(),
        Column::build("Sequence").int16().build(),
    ]).unwrap();
    builder.insert_rows("File", vec![
        vec![Value::from("F1"), Value::from("MainComp"), Value::from("test.txt"), Value::from(10i32), Value::from(0i32), Value::from(1i32)],
    ]).unwrap();
    
    // Media table
    builder.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int32().build(),
        Column::build("Cabinet").string(255).nullable().build(),
        Column::build("DiskPrompt").string(64).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Media", vec![
        vec![Value::from(1i32), Value::from(1i32), Value::from("#cab1.cab"), Value::Null],
    ]).unwrap();
    
    // InstallExecuteSequence table
    builder.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostFinalize"), Value::Null, Value::from(1000i32)],
        vec![Value::from("InstallFiles"), Value::Null, Value::from(4000i32)],
    ]).unwrap();
    
    let our_data = builder.build().unwrap();
    println!("velocity-msi size: {} bytes", our_data.len());
    
    // Write our MSI for comparison
    let our_path = "C:\\temp\\ref_msi_test\\velocity.msi";
    std::fs::write(our_path, &our_data).unwrap();
    println!("Wrote velocity-msi to {}", our_path);
    
    // Step 4: Compare stream by stream
    println!("\n=== Stream comparison ===");
    compare_streams(&ref_data, &our_data);
    
    // Step 5: Test both with msiexec
    println!("\n=== Testing reference MSI ===");
    let ref_log = "C:\\temp\\ref_msi_test\\ref_install.log";
    let status = Command::new("msiexec")
        .args(&["/i", ref_path, "/qn", "/norestart", "/l*v", ref_log])
        .status();
    match status {
        Ok(s) => println!("Reference MSI msiexec exit code: {}", s.code().unwrap_or(-1)),
        Err(e) => println!("Failed to run msiexec: {}", e),
    }
    
    println!("\n=== Testing velocity MSI ===");
    let our_log = "C:\\temp\\ref_msi_test\\velocity_install.log";
    let status = Command::new("msiexec")
        .args(&["/i", our_path, "/qn", "/norestart", "/l*v", our_log])
        .status();
    match status {
        Ok(s) => println!("Velocity MSI msiexec exit code: {}", s.code().unwrap_or(-1)),
        Err(e) => println!("Failed to run msiexec: {}", e),
    }
    
    // Uninstall reference
    let _ = Command::new("msiexec")
        .args(&["/x", ref_path, "/qn", "/norestart"])
        .status();
}

fn compare_streams(ref_data: &[u8], our_data: &[u8]) {
    // Use cfb to read streams from both
    let ref_streams = read_all_streams(ref_data, "ref");
    let our_streams = read_all_streams(our_data, "our");
    
    println!("\nReference streams:");
    for (name, data) in &ref_streams {
        println!("  {} ({} bytes)", name, data.len());
    }
    
    println!("\nOur streams:");
    for (name, data) in &our_streams {
        println!("  {} ({} bytes)", name, data.len());
    }
    
    // Compare matching streams
    println!("\n=== Stream-by-stream differences ===");
    for (name, ref_data) in &ref_streams {
        if let Some((_, our_data)) = our_streams.iter().find(|(n, _)| n == name) {
            if ref_data == our_data {
                println!("  {} - IDENTICAL ({} bytes)", name, ref_data.len());
            } else {
                println!("  {} - DIFFERENT (ref={} bytes, our={} bytes)", name, ref_data.len(), our_data.len());
                // Show first difference
                for (i, (r, o)) in ref_data.iter().zip(our_data.iter()).enumerate() {
                    if r != o {
                        println!("    First diff at byte {}: ref=0x{:02X} our=0x{:02X}", i, r, o);
                        // Show context
                        let start = i.saturating_sub(4);
                        let end = (i + 16).min(ref_data.len()).min(our_data.len());
                        println!("    ref[{}..{}]: {:02X?}", start, end, &ref_data[start..end]);
                        println!("    our[{}..{}]: {:02X?}", start, end, &our_data[start..end]);
                        break;
                    }
                }
            }
        } else {
            println!("  {} - ONLY IN REFERENCE ({} bytes)", name, ref_data.len());
        }
    }
    
    for (name, data) in &our_streams {
        if !ref_streams.iter().any(|(n, _)| n == name) {
            println!("  {} - ONLY IN OURS ({} bytes)", name, data.len());
        }
    }
}

fn read_all_streams(data: &[u8], label: &str) -> Vec<(String, Vec<u8>)> {
    let cursor = Cursor::new(data.to_vec());
    let mut cf = match cfb::CompoundFile::open(cursor) {
        Ok(cf) => cf,
        Err(e) => {
            println!("Failed to open {} as CFB: {}", label, e);
            return Vec::new();
        }
    };
    
    let mut streams = Vec::new();
    let paths: Vec<_> = cf.walk()
        .filter_map(|e| if e.is_stream() { Some((e.path().to_owned(), e.name().to_owned())) } else { None })
        .collect();
    
    for (path, name) in paths {
        let mut stream = match cf.open_stream(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let mut data = Vec::new();
        use std::io::Read;
        stream.read_to_end(&mut data).unwrap_or_default();
        
        if name.starts_with('\u{0005}') {
            streams.push((format!("\\u0005{}", &name[1..]), data));
        } else {
            streams.push((name, data));
        }
    }
    
    streams.sort_by(|a, b| a.0.cmp(&b.0));
    streams
}
