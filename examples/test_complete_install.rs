//! Definitive test: Complete MSI with File table + embedded cabinet, V4 format
//! 
//! Previous tests showed that velocity-msi V4 format works with msiexec (exit 0).
//! Now test if a complete MSI with File table + cabinet also works.

use std::io::Cursor;
use std::process::Command;
use velocity_msi::{MsiBuilder, Column, Value, CabinetFile, build_cabinet};

fn main() {
    println!("=== Definitive test: Complete MSI with File table ===\n");
    
    let mut builder = MsiBuilder::new();
    builder.set_title("TestProduct");
    builder.set_author("TestCo");
    
    // Property table
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("TestProduct")],
        vec![Value::from("ProductCode"), Value::from("{B1C2D3E4-F5A6-7890-ABCD-EF1234567890}")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
        vec![Value::from("Manufacturer"), Value::from("TestCo")],
        vec![Value::from("UpgradeCode"), Value::from("{C2D3E4F5-A6B7-8901-BCDE-F12345678901}")],
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
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFilesFolder"), Value::from("TestProduct")],
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
        vec![Value::from("MainComp"), Value::from("{D3E4F5A6-B7C8-9012-CDEF-123456789012}"), Value::from("INSTALLDIR"), Value::from(0i32), Value::Null, Value::from("F1")],
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
        vec![Value::from("F1"), Value::from("MainComp"), Value::from("test.txt"), Value::from(13i32), Value::from(0i32), Value::from(1i32)],
    ]).unwrap();
    
    // Media table - embedded cabinet
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
    
    // InstallUISequence table
    builder.create_table("InstallUISequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("InstallUISequence", vec![
        vec![Value::from("CostFinalize"), Value::Null, Value::from(1000i32)],
    ]).unwrap();
    
    // Create embedded cabinet
    let file_content = b"Hello, World!";  // 13 bytes
    let cab_files = vec![
        CabinetFile {
            name: "test.txt".to_string(),
            data: file_content.to_vec(),
        },
    ];
    let cab_data = build_cabinet(&cab_files);
    println!("Cabinet size: {} bytes", cab_data.len());
    
    // Add cabinet as embedded stream
    builder.add_stream("cab1.cab".to_string(), cab_data);
    
    // Build MSI
    let msi_data = builder.build().unwrap();
    println!("MSI size: {} bytes", msi_data.len());
    
    // Write MSI
    std::fs::create_dir_all("C:\\temp\\complete_test").ok();
    let msi_path = "C:\\temp\\complete_test\\complete.msi";
    std::fs::write(msi_path, &msi_data).unwrap();
    
    // Test with msiexec
    println!("\n=== Testing complete MSI with msiexec ===");
    let log_path = "C:\\temp\\complete_test\\install.log";
    let status = Command::new("msiexec")
        .args(&["/i", msi_path, "/qn", "/norestart", "/l*v", log_path])
        .status();
    match status {
        Ok(s) => {
            let code = s.code().unwrap_or(-1);
            println!("msiexec exit code: {} ({})", code, 
                match code {
                    0 => "SUCCESS - files installed!",
                    1603 => "fatal error during installation",
                    1620 => "package not valid",
                    1613 => "package cannot be opened",
                    2725 => "UI server error",
                    _ => "other error",
                });
            
            if code == 0 {
                // Check if files were installed
                let install_dir = "C:\\Program Files\\TestProduct";
                if std::path::Path::new(install_dir).exists() {
                    println!("\nInstall directory exists: {}", install_dir);
                    if let Ok(entries) = std::fs::read_dir(install_dir) {
                        for entry in entries.flatten() {
                            println!("  {}", entry.path().display());
                        }
                    }
                    
                    let test_file = format!("{}\\test.txt", install_dir);
                    if std::path::Path::new(&test_file).exists() {
                        let content = std::fs::read_to_string(&test_file).unwrap_or_default();
                        println!("\ntest.txt content: '{}'", content);
                    } else {
                        println!("\ntest.txt NOT found at {}", test_file);
                    }
                } else {
                    println!("\nInstall directory NOT found: {}", install_dir);
                }
            }
        }
        Err(e) => println!("Failed to run msiexec: {}", e),
    }
    
    // If install succeeded, try uninstall
    println!("\n=== Testing uninstall ===");
    let status = Command::new("msiexec")
        .args(&["/x", msi_path, "/qn", "/norestart"])
        .status();
    match status {
        Ok(s) => {
            let code = s.code().unwrap_or(-1);
            println!("Uninstall msiexec exit code: {} ({})", code,
                match code {
                    0 => "SUCCESS - files removed!",
                    _ => "error",
                });
        }
        Err(e) => println!("Failed to run msiexec: {}", e),
    }
}
