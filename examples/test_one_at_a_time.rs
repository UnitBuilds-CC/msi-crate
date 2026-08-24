//! Test: insert properties one-at-a-time (like compiler) vs batch (like test).
//! Does the insertion pattern affect the string pool?

use std::process::Command;
use velocity_msi::{MsiBuilder, Column, Value, CabinetFile, build_cabinet};

fn build_msi_one_at_a_time() -> Vec<u8> {
    let test_content = b"Hello from Velocity MSI!";
    let cabinet = build_cabinet(&[
        CabinetFile { name: "F1".to_string(), data: test_content.to_vec() },
    ]);

    let mut b = MsiBuilder::new();
    b.set_title("Test");
    b.set_author("Test");
    b.set_subject("Test");
    b.set_comments("Test");
    b.set_template("x64", 1033);

    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).build(),
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

    // Insert properties ONE AT A TIME (like the compiler does)
    let props = vec![
        ("ProductCode", "{D5E0EEC4-CF5C-5D68-BAC7-AA26667F6C80}"),
        ("UpgradeCode", "{7B44FAB1-58DD-5368-9B0C-338B5E7519DD}"),
        ("ProductName", "Sample App"),
        ("Manufacturer", "Velocity Team"),
        ("ProductVersion", "1.0.0"),
        ("ProductLanguage", "1033"),
        ("Description", "Sample App installer package"),
    ];
    for (name, value) in &props {
        b.insert_rows("Property", vec![
            vec![Value::from(*name), Value::from(*value)],
        ]).unwrap();
    }

    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("ProgramFiles64Folder"), Value::from("TARGETDIR"), Value::from("PFiles")],
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFiles64Folder"), Value::from("SampleApp:SampleApp")],
    ]).unwrap();
    b.insert_rows("Component", vec![
        vec![Value::from("comp_0"), Value::Null, Value::from("INSTALLDIR"),
             Value::from(0i32), Value::Null, Value::from("F1")],
    ]).unwrap();
    b.insert_rows("File", vec![
        vec![Value::from("F1"), Value::from("comp_0"), Value::from("test.txt"),
             Value::from(test_content.len() as i32), Value::Null, Value::Null,
             Value::from(0i32), Value::from(1i32)],
    ]).unwrap();
    b.insert_rows("Media", vec![
        vec![Value::from(1i32), Value::from(1i32), Value::Null, Value::Null,
             Value::from("#velo.cab"), Value::Null],
    ]).unwrap();
    b.insert_rows("Feature", vec![
        vec![Value::from("Complete"), Value::Null, Value::from("Complete"),
             Value::from("All features"), Value::from(1i32), Value::from(1i32),
             Value::Null, Value::from(0i32)],
    ]).unwrap();
    b.insert_rows("FeatureComponents", vec![
        vec![Value::from("Complete"), Value::from("comp_0")],
    ]).unwrap();

    b.add_stream("velo.cab".to_string(), cabinet);
    b.build().unwrap()
}

fn main() {
    std::fs::create_dir_all("C:\\temp\\vel_msi_test").ok();

    println!("=== Test: one-at-a-time property insertion ===");
    let msi = build_msi_one_at_a_time();
    let path = "C:\\temp\\vel_msi_test\\one_at_a_time.msi";
    std::fs::write(path, &msi).unwrap();
    println!("MSI: {} bytes", msi.len());

    let log = "C:\\temp\\vel_msi_test\\one_at_a_time.log";
    let status = Command::new("msiexec")
        .args(&["/i", path, "/qn", "/l*v", log, "TARGETDIR=C:\\temp\\vel_msi_oati"])
        .status().unwrap();
    let code = status.code().unwrap_or(-1);
    println!("Exit code: {}", code);

    if code != 0 {
        if let Ok(log_content) = std::fs::read_to_string(log) {
            // Look for Product Name in the summary
            for line in log_content.lines().rev() {
                if line.contains("Product Name:") || line.contains("1708") || line.contains("Error") {
                    println!("  {}", line.trim());
                    break;
                }
            }
        }
    } else {
        println!("SUCCESS!");
        let _ = Command::new("msiexec").args(&["/x", path, "/qn"]).status();
    }
    let _ = std::fs::remove_dir_all("C:\\temp\\vel_msi_oati");
}
