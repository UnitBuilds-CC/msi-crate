/// Test: Open COM-created MSI (with SummaryInfo) using msi crate, fix, and save
/// cargo run --example fix_com_msi -p velocity-msi
use std::io::Cursor;

fn main() {
    println!("=== FIX COM MSI TEST ===\n");

    // Use the test_si2.msi which has SummaryInfo set via VBScript
    let in_path = "C:\\temp\\test_si2.msi";
    let out_path = "C:\\temp\\fixed_com.msi";

    if !std::path::Path::new(in_path).exists() {
        println!("File not found: {}", in_path);
        return;
    }

    let data = std::fs::read(in_path).unwrap();
    println!("Input: {} bytes", data.len());

    // Try opening with msi crate
    let cursor = Cursor::new(data.clone());
    match msi::Package::open(cursor) {
        Ok(mut pkg) => {
            println!("msi crate opened OK!");

            // List tables
            let tables: Vec<String> = pkg.tables().map(|t| t.name().to_string()).collect();
            println!("Tables: {:?}", tables);

            // Fix SummaryInfo
            let si = pkg.summary_info_mut();
            si.set_title("Velocity Test Installation");
            si.set_subject("Velocity Test");
            si.set_author("Velocity Corp");
            si.set_codepage(msi::CodePage::Windows1252);
            si.set_arch("Intel");
            si.set_languages(&[msi::Language::from_code(1033)]);
            si.set_word_count(2);
            si.set_creating_application("Velocity Installer");
            println!("SummaryInfo fixed");

            // Flush
            match pkg.flush() {
                Ok(_) => {
                    println!("Flush OK");
                    let cursor = pkg.into_inner().unwrap();
                    let msi_data = cursor.into_inner();
                    std::fs::write(out_path, &msi_data).unwrap();
                    println!("Wrote: {} ({} bytes)", out_path, msi_data.len());
                }
                Err(e) => {
                    println!("Flush failed: {:?}", e);
                    return;
                }
            }
        }
        Err(e) => {
            println!("msi crate open failed: {:?}", e);
            println!("\nTrying to examine the OLE structure...");

            // Open with cfb to examine
            let comp = cfb::CompoundFile::open(Cursor::new(&data)).unwrap();
            let entries: Vec<_> = comp.walk().map(|e| (e.name().to_string(), e.is_stream())).collect();
            println!("Streams:");
            for (name, is_stream) in &entries {
                println!("  {} [{}]", name, if *is_stream { "S" } else { "D" });
            }
            return;
        }
    }

    // Test with msiexec
    println!("\n--- msiexec test ---");
    let _ = std::fs::remove_file("C:\\temp\\fixed_com.log");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", "C:\\temp\\fixed_com.log"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);

    if let Ok(log) = std::fs::read_to_string("C:\\temp\\fixed_com.log") {
        for line in log.lines() {
            if line.contains("Error") || line.contains("2219") || line.contains("2203") ||
               line.contains("Product:") || line.contains("successful") {
                println!("  {}", line.trim());
            }
        }
    }

    println!("\n=== DONE ===");
}
