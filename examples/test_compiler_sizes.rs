//! Test with compiler's exact column sizes to see if string(38) vs string(72) matters.

use std::process::Command;
use velocity_msi::{MsiBuilder, Column, Value, CabinetFile, build_cabinet};

fn main() {
    println!("=== Test: compiler's exact column sizes ===\n");
    
    let test_content = b"Hello from Velocity MSI!";
    let test_file_name = "velocity_test.txt";

    let cabinet = build_cabinet(&[
        CabinetFile { name: "F1".to_string(), data: test_content.to_vec() },
    ]);

    let mut b = MsiBuilder::new();
    b.set_title("Test");
    b.set_author("Test");
    b.set_subject("Test");
    b.set_comments("Test");
    b.set_template("x64", 1033);

    // Use string(38) for Feature and FeatureComponents like the compiler does
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
        Column::build("ComponentId").string(38).nullable().build(),  // string(38) like compiler
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
    b.create_table("File", vec![
        Column::build("File").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).build(),
        Column::build("FileSize").int32().build(),
        Column::build("Version").string(72).nullable().build(),
        Column::build("Language").int16().nullable().build(),
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
    // Compiler uses string(38) for Feature!
    b.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").string(38).nullable().build(),
        Column::build("Title").string(64).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().nullable().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
    ]).unwrap();
    // Compiler uses string(38) for FeatureComponents!
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

    // Populate data
    b.insert_rows("Property", vec![
        vec![Value::from("ProductCode"), Value::from("{12345678-1234-5678-9ABC-DEF012345678}")],
        vec![Value::from("ProductName"), Value::from("Test Product")],
        vec![Value::from("Manufacturer"), Value::from("Test Mfg")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("TestDir")],
    ]).unwrap();
    b.insert_rows("Component", vec![
        vec![Value::from("MainComp"), Value::Null, Value::from("INSTALLDIR"),
             Value::from(0i32), Value::Null, Value::from("F1")],
    ]).unwrap();
    b.insert_rows("File", vec![
        vec![Value::from("F1"), Value::from("MainComp"), Value::from(test_file_name),
             Value::from(test_content.len() as i32), Value::Null, Value::Null,
             Value::from(0i32), Value::from(1i32)],
    ]).unwrap();
    b.insert_rows("Media", vec![
        vec![Value::from(1i32), Value::from(1i32), Value::Null, Value::Null,
             Value::from("#velo.cab"), Value::Null],
    ]).unwrap();
    b.insert_rows("Feature", vec![
        vec![Value::from("MainFeature"), Value::Null, Value::from("Main"),
             Value::from("Main feature"), Value::Null, Value::from(1i32),
             Value::Null, Value::Null],
    ]).unwrap();
    b.insert_rows("FeatureComponents", vec![
        vec![Value::from("MainFeature"), Value::from("MainComp")],
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::from(800i32)],
        vec![Value::from("CostFinalize"), Value::Null, Value::from(1000i32)],
        vec![Value::from("InstallValidate"), Value::Null, Value::from(1400i32)],
        vec![Value::from("InstallInitialize"), Value::Null, Value::from(1500i32)],
        vec![Value::from("InstallFiles"), Value::Null, Value::from(4000i32)],
        vec![Value::from("InstallFinalize"), Value::Null, Value::from(6600i32)],
    ]).unwrap();
    b.insert_rows("InstallUISequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::from(800i32)],
        vec![Value::from("CostFinalize"), Value::Null, Value::from(1000i32)],
    ]).unwrap();

    b.add_stream("velo.cab".to_string(), cabinet);
    let msi_data = b.build().unwrap();

    let msi_path = "C:\\temp\\vel_msi_test\\compiler_sizes.msi";
    std::fs::create_dir_all("C:\\temp\\vel_msi_test").ok();
    std::fs::write(msi_path, &msi_data).unwrap();
    println!("MSI written: {} bytes", msi_data.len());

    let install_dir = "C:\\temp\\vel_msi_install\\compiler_sizes";
    let log_path = "C:\\temp\\vel_msi_test\\compiler_sizes.log";
    let status = Command::new("msiexec")
        .args(&["/i", msi_path, "/qn", "/l*v", log_path,
                &format!("TARGETDIR={}", install_dir)])
        .status().unwrap();
    let code = status.code().unwrap_or(-1);
    println!("Exit code: {}", code);

    if code != 0 {
        if let Ok(log) = std::fs::read_to_string(log_path) {
            let lines: Vec<&str> = log.lines().collect();
            let start = lines.len().saturating_sub(20);
            for line in &lines[start..] {
                println!("  {}", line);
            }
        }
    } else {
        println!("SUCCESS with string(38)!");
        let _ = Command::new("msiexec").args(&["/x", msi_path, "/qn"]).status();
    }
    let _ = std::fs::remove_dir_all(install_dir);
}
