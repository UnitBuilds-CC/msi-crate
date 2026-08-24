/// Test: Does the msi crate's open+flush preserve a valid MSI?
/// 1. Copy template MSI
/// 2. Open with msi crate, do nothing, flush
/// 3. Test with msiexec
///
/// cargo run --example flush_test -p velocity-msi
use std::io::Cursor;

fn main() {
    println!("=== MSI CRATE FLUSH TEST ===\n");

    let template_path = "C:\\Program Files\\Microsoft Office\\root\\Integration\\C2RIntLoc.en-us.16.msi";
    if !std::path::Path::new(template_path).exists() {
        println!("Template not found: {}", template_path);
        return;
    }

    // Test 1: Copy template and test it works
    let copy_path = "C:\\temp\\flush_test_copy.msi";
    std::fs::copy(template_path, copy_path).unwrap();
    println!("Testing original template...");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", copy_path, "/qn", "/l*v", "C:\\temp\\flush_test_orig.log"])
        .output().unwrap();
    println!("  Original exit code: {}", output.status.code().unwrap_or(-1));

    // Test 2: Open with msi crate, don't change anything, flush
    let out_path = "C:\\temp\\flush_test_output.msi";
    {
        let data = std::fs::read(template_path).unwrap();
        let orig_len = data.len();
        let cursor = Cursor::new(data);
        let mut pkg = msi::Package::open(cursor).unwrap();
        println!("\nOpened template OK");

        // Don't change anything - just flush
        pkg.flush().unwrap();
        let cursor = pkg.into_inner().unwrap();
        let msi_data = cursor.into_inner();
        std::fs::write(out_path, &msi_data).unwrap();
        println!("Flushed: {} bytes (original: {} bytes)", msi_data.len(), orig_len);
    }

    // Test 3: Test the flushed output
    println!("\nTesting flushed output...");
    let _ = std::fs::remove_file("C:\\temp\\flush_test_output.log");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", "C:\\temp\\flush_test_output.log"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("  Flushed exit code: {}", exit_code);
    match exit_code {
        0 => println!("  SUCCESS!"),
        1613 => println!("  1613 (invalid package)"),
        1619 => println!("  1619 (not valid)"),
        1620 => println!("  1620 (could not open)"),
        _ => println!("  Error {}", exit_code),
    }

    // Check log
    if let Ok(log) = std::fs::read_to_string("C:\\temp\\flush_test_output.log") {
        for line in log.lines() {
            if line.contains("Error") || line.contains("return value 3") ||
               line.contains("Product:") || line.contains("Could not open") ||
               line.contains("2219") {
                println!("  LOG: {}", line.trim());
            }
        }
    }

    // Test 3: Open, delete one table, flush
    let out_path2 = "C:\\temp\\flush_test_del.msi";
    {
        let data = std::fs::read(template_path).unwrap();
        let cursor = Cursor::new(data);
        let mut pkg = msi::Package::open(cursor).unwrap();

        // Get list of user tables
        let user_tables: Vec<String> = pkg.tables()
            .map(|t| t.name().to_string())
            .filter(|n| !n.starts_with('_'))
            .collect();
        println!("\nUser tables: {}", user_tables.len());
        for t in &user_tables[..5.min(user_tables.len())] {
            println!("  {}", t);
        }

        // Delete all user tables
        for t in &user_tables {
            pkg.drop_table(t).unwrap();
        }
        println!("Deleted {} user tables", user_tables.len());

        pkg.flush().unwrap();
        let cursor = pkg.into_inner().unwrap();
        let msi_data = cursor.into_inner();
        std::fs::write(out_path2, &msi_data).unwrap();
        println!("Flushed after delete: {} bytes", msi_data.len());
    }

    println!("\nTesting after table deletion...");
    let _ = std::fs::remove_file("C:\\temp\\flush_test_del.log");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path2, "/qn", "/l*v", "C:\\temp\\flush_test_del.log"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("  Delete exit code: {}", exit_code);

    if let Ok(log) = std::fs::read_to_string("C:\\temp\\flush_test_del.log") {
        for line in log.lines() {
            if line.contains("Error") || line.contains("return value 3") ||
               line.contains("Product:") || line.contains("2219") {
                println!("  LOG: {}", line.trim());
            }
        }
    }

    println!("\n=== DONE ===");
}
