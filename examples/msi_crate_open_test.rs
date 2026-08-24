/// Test: Use the msi crate to open a COM-created template MSI,
/// add our tables, and save. This tests if the msi crate's open+modify+flush
/// path produces valid MSIs.
///
/// cargo run --example msi_crate_open_test -p velocity-msi
use std::io::Cursor;

fn main() {
    println!("=== MSI CRATE OPEN TEST ===\n");

    let _ = std::process::Command::new("taskkill")
        .args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(2));

    let template_path = "C:\\temp\\minimal_template.msi";

    // Check if template exists
    if !std::path::Path::new(template_path).exists() {
        println!("Template not found. Creating with PowerShell...");
        let script = r#"
$msiPath = "C:\temp\minimal_template.msi"
if (Test-Path $msiPath) { Remove-Item $msiPath -Force }
$installer = New-Object -ComObject WindowsInstaller.Installer
$database = $installer.OpenDatabase($msiPath, 3)
$view = $database.OpenView("CREATE TABLE Property (Property CHAR(72) NOT NULL, Value CHAR(255) NOT NULL LOCALIZABLE PRIMARY KEY Property)")
$view.Execute(); $view.Close()
$view = $database.OpenView("INSERT INTO Property (Property, Value) VALUES ('ProductName', 'Test')")
$view.Execute(); $view.Close()
$database.Commit()
Write-Host "Created template"
"#;
        std::fs::write("C:\\temp\\create_template.ps1", script).unwrap();
        let output = std::process::Command::new("powershell")
            .args(&["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "C:\\temp\\create_template.ps1"])
            .output().unwrap();
        println!("Create output: {}", String::from_utf8_lossy(&output.stdout));
        println!("Create errors: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Read the template
    let template_data = std::fs::read(template_path).unwrap();
    println!("Template: {} bytes", template_data.len());

    // Open with msi crate
    let pkg = match msi::Package::open(Cursor::new(&template_data)) {
        Ok(p) => {
            println!("msi crate: opened OK");
            p
        }
        Err(e) => {
            println!("msi crate: FAILED to open: {:?}", e);
            return;
        }
    };

    // List existing tables
    for table in pkg.tables() {
        println!("  Existing table: {} ({} cols)", table.name(), table.columns().len());
    }

    // Write to output file
    let out_path = "C:\\temp\\msi_crate_open.msi";
    let mut out_buf = Vec::new();
    {
        let cursor = Cursor::new(&mut out_buf);
        // Try to flush the package to a new buffer
        // The msi crate's Package doesn't have a direct "write_to" method
        // We need to use the internal write mechanism
    }

    // Actually, the msi crate's Package::open() reads from a Read+Seek source
    // but modifications are written back to the same source when dropped/flushed
    // Let me try a different approach: open from a file, modify, and the changes
    // are written back

    // Copy template to output
    std::fs::copy(template_path, out_path).unwrap();

    // Open the copy with msi crate in read-write mode
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(out_path)
        .unwrap();

    let mut pkg2 = match msi::Package::open(file) {
        Ok(p) => {
            println!("\nOpened copy for editing");
            p
        }
        Err(e) => {
            println!("Failed to open copy: {:?}", e);
            return;
        }
    };

    // Try to create a new table
    println!("\nAttempting to create Property table...");
    match pkg2.create_table("Property2", vec![
        msi::Column::build("Property2").string(72).is_primary_key().build(),
        msi::Column::build("Value2").string(255).is_nullable().build(),
    ]) {
        Ok(_) => println!("  Created Property2 table"),
        Err(e) => println!("  Failed: {:?}", e),
    }

    // Drop the package to flush changes
    drop(pkg2);

    // Check file size
    let new_size = std::fs::metadata(out_path).unwrap().len();
    println!("\nOutput: {} bytes", new_size);

    // Test with msiexec
    println!("\n--- Testing with msiexec ---");
    let _ = std::fs::remove_file("C:\\temp\\msi_crate_open.log");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", "C:\\temp\\msi_crate_open.log"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);
    match exit_code {
        0 => println!("SUCCESS!"),
        1603 => println!("1603 (accepted, install failed as expected)"),
        1619 => println!("1619 (invalid installation package)"),
        1620 => println!("1620 (could not open)"),
        _ => println!("Error {}", exit_code),
    }

    // Also test the original template
    println!("\n--- Testing original template ---");
    let _ = std::fs::remove_file("C:\\temp\\template_orig.log");
    let output2 = std::process::Command::new("msiexec")
        .args(&["/i", template_path, "/qn", "/l*v", "C:\\temp\\template_orig.log"])
        .output().unwrap();
    let exit2 = output2.status.code().unwrap_or(-1);
    println!("Template exit code: {}", exit2);

    println!("\n=== DONE ===");
}
