//! Test: File table with proper validation categories
//! 
//! MSI spec defines specific categories for standard table columns.
//! Without these categories, Windows Installer might reject the File table.

use std::process::Command;
use velocity_msi::{MsiBuilder, Column, Value};

fn main() {
    println!("=== Test: File table with validation categories ===\n");
    
    let mut builder = MsiBuilder::new();
    builder.set_title("Test");
    builder.set_author("TestCo");
    
    // Property table
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().category("Identifier").build(),
        Column::build("Value").string(255).nullable().category("Text").build(),
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
        Column::build("Directory").string(72).primary_key().category("Identifier").build(),
        Column::build("Directory_Parent").string(72).nullable().category("Identifier").build(),
        Column::build("DefaultDir").string(255).category("DefaultDir").build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("ProgramFilesFolder"), Value::from("TARGETDIR"), Value::from("PFiles")],
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFilesFolder"), Value::from("TestProduct")],
    ]).unwrap();
    
    // Component table
    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().category("Identifier").build(),
        Column::build("ComponentId").string(38).nullable().category("RegID").build(),
        Column::build("Directory_").string(72).category("Identifier").build(),
        Column::build("Attributes").int16().nullable().build(),
        Column::build("Condition").string(255).nullable().category("Condition").build(),
        Column::build("KeyPath").string(72).nullable().category("Identifier").build(),
    ]).unwrap();
    builder.insert_rows("Component", vec![
        vec![Value::from("MainComp"), Value::from("{D3E4F5A6-B7C8-9012-CDEF-123456789012}"), Value::from("INSTALLDIR"), Value::from(0i32), Value::Null, Value::from("F1")],
    ]).unwrap();
    
    // Feature table
    builder.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().category("Identifier").build(),
        Column::build("Feature_Parent").string(38).nullable().category("Identifier").build(),
        Column::build("Title").string(64).nullable().category("Text").build(),
        Column::build("Description").string(255).nullable().category("Text").build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").string(72).nullable().category("Identifier").build(),
        Column::build("Attributes").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("Feature", vec![
        vec![Value::from("MainFeature"), Value::Null, Value::from("Main Feature"), Value::from("Installs files"), Value::from(1i32), Value::from(1i32), Value::from("INSTALLDIR"), Value::Null],
    ]).unwrap();
    
    // FeatureComponents table
    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().category("Identifier").build(),
        Column::build("Component_").string(72).primary_key().category("Identifier").build(),
    ]).unwrap();
    builder.insert_rows("FeatureComponents", vec![
        vec![Value::from("MainFeature"), Value::from("MainComp")],
    ]).unwrap();
    
    // File table - WITH CATEGORIES
    builder.create_table("File", vec![
        Column::build("File_").string(72).primary_key().category("Identifier").build(),
        Column::build("Component_").string(72).category("Identifier").build(),
        Column::build("FileName").string(255).category("Filename").build(),
        Column::build("FileSize").int32().category("Integer").build(),
        Column::build("Attributes").int16().nullable().build(),
        Column::build("Sequence").int32().category("Integer").build(),
    ]).unwrap();
    builder.insert_rows("File", vec![
        vec![Value::from("F1"), Value::from("MainComp"), Value::from("test.txt"), Value::from(13i32), Value::from(0i32), Value::from(1i32)],
    ]).unwrap();
    
    // Media table
    builder.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int32().build(),
        Column::build("Cabinet").string(255).nullable().category("CabName").build(),
        Column::build("DiskPrompt").string(64).nullable().category("Text").build(),
    ]).unwrap();
    builder.insert_rows("Media", vec![
        vec![Value::from(1i32), Value::from(1i32), Value::from("#cab1.cab"), Value::Null],
    ]).unwrap();
    
    // InstallExecuteSequence
    builder.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().category("Identifier").build(),
        Column::build("Condition").string(255).nullable().category("Condition").build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostFinalize"), Value::Null, Value::from(1000i32)],
        vec![Value::from("InstallFiles"), Value::Null, Value::from(4000i32)],
    ]).unwrap();
    
    // Build and test
    let data = builder.build().unwrap();
    let path = "C:\\temp\\category_test\\with_categories.msi";
    std::fs::create_dir_all("C:\\temp\\category_test").ok();
    std::fs::write(path, &data).unwrap();
    
    println!("Testing with categories...");
    let log = "C:\\temp\\category_test\\with_cat.log";
    let status = Command::new("msiexec")
        .args(&["/i", path, "/qn", "/norestart", "/l*v", log])
        .status();
    match status {
        Ok(s) => {
            let code = s.code().unwrap_or(-1);
            println!("Exit code: {} ({})", code,
                match code {
                    0 => "SUCCESS!",
                    1603 => "fatal error",
                    1620 => "package not valid",
                    _ => "other",
                });
        }
        Err(e) => println!("Failed: {}", e),
    }
}
