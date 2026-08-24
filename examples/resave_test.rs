/// Test: Open our MSI with the msi crate and re-save.
/// This tests if the msi crate's serialization produces a working MSI.
use std::process::Command;

fn main() {
    println!("=== RESAVE TEST ===\n");

    let _ = Command::new("taskkill").args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Build our MSI with Property + Directory + InstallExecuteSequence
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
    
    // Save our MSI to a temp file
    let our_path = "resave_original.msi";
    std::fs::write(our_path, &our_data).unwrap();
    println!("Our MSI: {} bytes", our_data.len());

    // Test our MSI first
    println!("\n--- Testing our MSI ---");
    let code_our = test_msiexec(our_path, "resave_our.log");
    println!("Our MSI exit code: {}", code_our);

    // Now open with msi crate and re-save
    println!("\n--- Re-saving with msi crate ---");
    let resave_path = "resave_msi_crate.msi";
    
    // Copy original to resave path
    std::fs::copy(our_path, resave_path).unwrap();
    
    // Open the copy with msi crate
    let file = std::fs::File::open(resave_path).unwrap();
    let mut pkg = msi::Package::open(file).unwrap();
    println!("Opened with msi crate.");
    
    // Read tables
    let tables: Vec<String> = pkg.tables().map(|t| t.name().to_string()).collect();
    println!("Tables: {:?}", tables);
    
    // Read Directory table
    if let Ok(rows) = pkg.select_rows(msi::Select::table("Directory")) {
        for row in rows {
            println!("  Dir: {} | {:?} | {}", 
                row[0].as_str().unwrap_or("?"),
                row[1].as_str(),
                row[2].as_str().unwrap_or("?"));
        }
    }
    
    // Flush (re-save) - this writes back to the file
    pkg.flush().unwrap();
    drop(pkg);
    println!("Flushed. Re-saved MSI: {} bytes", std::fs::metadata(resave_path).unwrap().len());
    
    // Test the re-saved MSI
    println!("\n--- Testing re-saved MSI ---");
    let code_resave = test_msiexec(resave_path, "resave_crate.log");
    println!("Re-saved MSI exit code: {}", code_resave);

    // Summary
    println!("\n=== RESULTS ===");
    println!("Our MSI:     exit code {}", code_our);
    println!("Re-saved:    exit code {}", code_resave);
    
    if code_resave == 0 {
        println!("\nRe-saved MSI works! Our data is correct, serialization differs.");
    } else if code_resave == code_our {
        println!("\nSame error. Data issue confirmed.");
    }
    
    // Cleanup
    if code_our == 0 {
        let _ = Command::new("msiexec").args(&["/x", our_path, "/qn", "/norestart"]).output();
    }
    if code_resave == 0 {
        let _ = Command::new("msiexec").args(&["/x", resave_path, "/qn", "/norestart"]).output();
    }
}

fn test_msiexec(path: &str, log: &str) -> i32 {
    let output = Command::new("msiexec")
        .args(&["/i", path, "/qn", "/norestart", "/l*v", log])
        .output()
        .expect("msiexec failed");
    let code = output.status.code().unwrap_or(-1);
    if code == 0 {
        // Check log for errors
        if let Ok(log_content) = std::fs::read_to_string(log) {
            for line in log_content.lines() {
                if line.contains("2705") || line.contains("return value 3") {
                    println!("  LOG: {}", line.trim());
                }
            }
        }
    }
    code
}
