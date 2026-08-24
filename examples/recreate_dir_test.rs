/// Test: Open our MSI with msi crate, drop Directory table,
/// recreate it with same data, flush. If result works → our serialization is wrong.
/// If result fails → OLE structure or system table issue.
use velocity_msi::{MsiBuilder, Column, Value};
use std::process::Command;

fn main() {
    // Build MSI with velocity-msi
    let mut b = MsiBuilder::new();
    b.set_title("Recreate Test");
    b.set_author("V");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("RecreateTest")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("V")],
        vec![Value::from("ProductCode"), Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![Value::from("UpgradeCode"), Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    b.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
    ]).unwrap();

    b.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
    ]).unwrap();

    let msi_data = b.build().unwrap();
    std::fs::write("recreate_orig.msi", &msi_data).unwrap();
    println!("Original MSI: {} bytes", msi_data.len());

    // Test original
    println!("\n--- Test 1: Original velocity-msi MSI ---");
    let code1 = test_msiexec("recreate_orig.msi", "recreate_orig.log");
    println!("Exit code: {}", code1);

    // Open with msi crate, drop Directory, recreate with same data
    println!("\n--- Test 2: msi crate recreates Directory table ---");
    {
        let file = std::fs::OpenOptions::new()
            .read(true).write(true)
            .open("recreate_orig.msi").unwrap();
        let mut pkg = msi::Package::open(file).unwrap();

        // Drop the Directory table
        pkg.drop_table("Directory").unwrap();
        println!("Dropped Directory table.");

        // Recreate with msi crate's API
        pkg.create_table("Directory", vec![
            msi::Column::build("Directory").primary_key().string(72),
            msi::Column::build("Directory_Parent").nullable().string(72),
            msi::Column::build("DefaultDir").primary_key().string(255),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Directory")
            .row(vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str("SourceDir".into())])
        ).unwrap();
        println!("Recreated Directory with msi crate API.");

        pkg.flush().unwrap();
        println!("Flushed.");
    }

    let modified_data = std::fs::read("recreate_orig.msi").unwrap();
    println!("Modified MSI: {} bytes", modified_data.len());

    let code2 = test_msiexec("recreate_orig.msi", "recreate_modified.log");
    println!("Exit code: {}", code2);

    // Summary
    println!("\n=== RESULTS ===");
    println!("Original:  exit code {}", code1);
    println!("Modified:  exit code {}", code2);

    if code1 != 0 && code2 == 0 {
        println!("\n>>> SERIALIZATION BUG in velocity-msi!");
        println!(">>> msi crate's Directory serialization works.");
    } else if code1 != 0 && code2 != 0 {
        println!("\n>>> Both fail: OLE structure or system table issue.");
        // Check logs
        for log_name in &["recreate_orig.log", "recreate_modified.log"] {
            if let Ok(log) = std::fs::read_to_string(log_name) {
                println!("\n--- {} errors ---", log_name);
                for line in log.lines() {
                    if line.contains("2705") || line.contains("1620") || line.contains("DEBUG: Error") {
                        println!("  {}", line.trim());
                    }
                }
            }
        }
    }
}

fn test_msiexec(path: &str, log: &str) -> i32 {
    let _ = std::fs::remove_file(log);
    let output = Command::new("msiexec")
        .args(&["/i", path, "/qn", "/norestart", "/lv", log])
        .output()
        .expect("msiexec failed");
    output.status.code().unwrap_or(-1)
}
