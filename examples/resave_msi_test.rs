/// Test: Open our MSI with the msi crate and re-save (flush).
/// If the re-saved MSI works → our serialization has a bug.
/// If it still fails → our OLE structure has a bug.
use std::io::Cursor;
use std::process::Command;

fn main() {
    println!("=== MSI CRATE RESAVE TEST ===\n");

    let _ = Command::new("taskkill").args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Build our MSI
    let mut b = velocity_msi::MsiBuilder::new();
    b.set_title("ResaveTest");
    b.set_author("Velocity");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        velocity_msi::Column::build("Property").string(72).primary_key().build(),
        velocity_msi::Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("ResaveTest")],
        vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0.0")],
        vec![velocity_msi::Value::from("Manufacturer"), velocity_msi::Value::from("V")],
        vec![velocity_msi::Value::from("ProductCode"), velocity_msi::Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![velocity_msi::Value::from("UpgradeCode"), velocity_msi::Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![velocity_msi::Value::from("ProductLanguage"), velocity_msi::Value::from("1033")],
    ]).unwrap();

    b.create_table("Directory", vec![
        velocity_msi::Column::build("Directory").string(72).primary_key().build(),
        velocity_msi::Column::build("Directory_Parent").string(72).nullable().build(),
        velocity_msi::Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::Null, velocity_msi::Value::from("SourceDir")],
    ]).unwrap();

    b.create_table("InstallExecuteSequence", vec![
        velocity_msi::Column::build("Action").string(72).primary_key().build(),
        velocity_msi::Column::build("Condition").string(255).nullable().build(),
        velocity_msi::Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![velocity_msi::Value::from("CostInitialize"), velocity_msi::Value::Null, velocity_msi::Value::Int(800)],
        vec![velocity_msi::Value::from("CostFinalize"), velocity_msi::Value::Null, velocity_msi::Value::Int(1000)],
    ]).unwrap();

    let our_data = b.build().unwrap();
    let our_path = "resave_original.msi";
    std::fs::write(our_path, &our_data).unwrap();
    println!("Original MSI: {} bytes", our_data.len());

    // Open with msi crate and re-save
    let file = std::fs::File::open(our_path).unwrap();
    let mut pkg = msi::Package::open(file).unwrap();
    println!("\nOpened with msi crate successfully.");
    println!("Package type: {:?}", pkg.package_type());

    // Read and display all tables
    println!("\nTables:");
    for table_name in pkg.tables() {
        println!("  {}", table_name.name());
    }

    // Read Directory table
    println!("\nDirectory table:");
    if let Ok(rows) = pkg.select_rows(msi::Select::table("Directory")) {
        for row in rows {
            let dir = row[0].as_str().unwrap_or("?");
            let parent = row[1].as_str();
            let default = row[2].as_str().unwrap_or("?");
            println!("  {} -> {:?} (default: {})", dir, parent, default);
        }
    }

    // Flush (re-save) the MSI
    let resave_path = "resave_msi_crate.msi";
    pkg.flush().unwrap();
    let resave_data = {
        // Get the inner file back
        let inner = pkg.into_inner().unwrap();
        drop(inner);
        std::fs::read(our_path).unwrap() // re-read the original
    };

    // Actually, flush() writes back to the same file. Let me use a different approach.
    // Create a new file and write the flushed data there.
    // The msi crate's flush() writes to the original writer.
    // Since we opened from a file, it writes back to that file.
    // So the original file is now "re-saved" by the msi crate.

    // Copy the re-saved file
    std::fs::copy(our_path, resave_path).unwrap();
    println!("\nRe-saved MSI: {} bytes", std::fs::metadata(resave_path).unwrap().len());

    // Test both with msiexec
    println!("\n=== Testing with msiexec ===");

    println!("\n--- Original (before msi crate flush) ---");
    // We already wrote the original, but flush() may have modified it.
    // Let's test the copy.
    let code_orig = test_msiexec(resave_path, "resave.log");
    println!("Exit code: {}", code_orig);

    // Summary
    println!("\n=== RESULTS ===");
    println!("Re-saved MSI exit code: {}", code_orig);
}

fn test_msiexec(path: &str, log: &str) -> i32 {
    let output = Command::new("msiexec")
        .args(&["/i", path, "/qn", "/norestart", "/l*v", log])
        .output()
        .expect("msiexec failed");
    let code = output.status.code().unwrap_or(-1);
    if code == 0 {
        let _ = Command::new("msiexec")
            .args(&["/x", path, "/qn", "/norestart"]).output();
    }
    code
}
