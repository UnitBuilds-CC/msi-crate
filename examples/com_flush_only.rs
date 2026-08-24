/// Test: Open COM-created MSI with msi crate and just flush (no changes)
/// This tests if the msi crate preserves COM-created MSIs
/// cargo run --example com_flush_only -p velocity-msi
use std::io::Cursor;

fn main() {
    println!("=== COM FLUSH ONLY TEST ===\n");

    let base_path = "C:\\temp\\com_base.msi";
    let out_path = "C:\\temp\\com_flush_only.msi";

    if !std::path::Path::new(base_path).exists() {
        println!("Run: cscript //nologo scripts\\create_com_base.vbs");
        return;
    }

    let _ = std::process::Command::new("taskkill")
        .args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Step 1: Open COM MSI with msi crate (it has no SummaryInfo, so this will fail)
    let data = std::fs::read(base_path).unwrap();
    println!("COM base: {} bytes", data.len());

    let cursor = Cursor::new(data);
    match msi::Package::open(cursor) {
        Ok(mut pkg) => {
            let tables: Vec<String> = pkg.tables().map(|t| t.name().to_string()).collect();
            println!("Opened OK! Tables: {:?}", tables);

            // Just flush without changes
            match pkg.flush() {
                Ok(_) => println!("Flush OK"),
                Err(e) => { println!("Flush failed: {:?}", e); return; }
            }

            let cursor = pkg.into_inner().unwrap();
            let msi_data = cursor.into_inner();
            std::fs::write(out_path, &msi_data).unwrap();
            println!("Wrote: {} ({} bytes)", out_path, msi_data.len());
        }
        Err(e) => {
            println!("Open failed: {:?}", e);
            println!("This is expected - COM MSI has no SummaryInfo");
            println!("The msi crate REQUIRES SummaryInfo to open");
            return;
        }
    }

    // Step 2: Test with msiexec
    println!("\n--- msiexec test ---");
    let _ = std::fs::remove_file("C:\\temp\\com_flush_only.log");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", "C:\\temp\\com_flush_only.log"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);

    println!("\n=== DONE ===");
}
