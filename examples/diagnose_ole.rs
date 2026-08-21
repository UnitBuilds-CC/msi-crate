/// Diagnostic tool to examine OLE structure and compare with cfb.
use std::io::{Cursor, Read};

fn main() {
    std::fs::create_dir_all("C:\\temp").ok();
    
    println!("=== Building MSI with custom OLE writer ===");
    let mut builder = velocity_msi::MsiBuilder::new();
    builder.set_title("Test Product");
    builder.set_author("Test Corp");
    builder.set_template("x64", 1033);
    
    builder.create_table("Property", vec![
        velocity_msi::Column::build("Property").string(72).primary_key().build(),
        velocity_msi::Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    
    builder.insert_rows("Property", vec![
        vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("Test Product")],
        vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0.0")],
    ]).unwrap();
    
    let custom_data = builder.build().unwrap();
    let custom_path = "C:\\temp\\custom_writer.msi";
    std::fs::write(custom_path, &custom_data).unwrap();
    println!("Custom writer MSI: {} bytes", custom_data.len());
    
    // Examine header
    println!("\n=== OLE Header Analysis ===");
    println!("Magic: 0x{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        custom_data[0], custom_data[1], custom_data[2], custom_data[3],
        custom_data[4], custom_data[5], custom_data[6], custom_data[7]);
    println!("Version: {}.{}", 
        u16::from_le_bytes([custom_data[26], custom_data[27]]),
        u16::from_le_bytes([custom_data[24], custom_data[25]]));
    println!("Byte order: 0x{:02X}{:02X}", custom_data[28], custom_data[29]);
    println!("Sector size: {} bytes", 2u32.pow(u16::from_le_bytes([custom_data[30], custom_data[31]]) as u32));
    println!("Mini sector size: {} bytes", 2u32.pow(u16::from_le_bytes([custom_data[32], custom_data[33]]) as u32));
    
    // Try to open with cfb
    println!("\n=== Opening with cfb ===");
    match cfb::CompoundFile::open(Cursor::new(&custom_data)) {
        Ok(mut comp) => {
            println!("✓ cfb can open the file");
            println!("Version: {:?}", comp.version());
            
            let root = comp.root_entry();
            println!("Root CLSID: {}", root.clsid());
            println!("Root state bits: 0x{:08X}", root.state_bits());
            
            let streams: Vec<_> = comp.walk().filter(|e| e.is_stream()).collect();
            println!("Stream count: {}", streams.len());
            
            for entry in &streams {
                println!("  Stream: {} ({} bytes)", entry.name(), entry.len());
            }
            
            // Try to read SummaryInfo
            let summary_name = "\u{0005}SummaryInformation";
            if let Ok(mut stream) = comp.open_stream(summary_name) {
                let mut summary_data = Vec::new();
                stream.read_to_end(&mut summary_data).unwrap();
                println!("\nSummaryInfo: {} bytes", summary_data.len());
                
                // Parse properties
                if summary_data.len() >= 56 {
                    let num_props = u32::from_le_bytes([
                        summary_data[52], summary_data[53], 
                        summary_data[54], summary_data[55]
                    ]);
                    println!("Properties: {}", num_props);
                    
                    for i in 0..num_props {
                        let base = 56 + (i as usize) * 8;
                        if base + 8 <= summary_data.len() {
                            let pid = u32::from_le_bytes([
                                summary_data[base], summary_data[base+1],
                                summary_data[base+2], summary_data[base+3]
                            ]);
                            println!("  PID {}", pid);
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("✗ cfb cannot open: {}", e);
        }
    }
    
    // Test with msiexec
    println!("\n=== Testing with msiexec ===");
    let out = std::process::Command::new("msiexec.exe")
        .args(&["/i", custom_path, "/quiet", "/norestart", "/l*v", "C:\\temp\\custom_log.txt"])
        .output()
        .unwrap();
    
    let exit_code = out.status.code().unwrap_or(-1);
    println!("msiexec exit code: {}", exit_code);
    
    match exit_code {
        0 => println!("✓ Installation succeeded!"),
        1613 => println!("✗ MSI opens but cannot install (error 1613)"),
        1620 => println!("✗ Cannot open MSI (error 1620)"),
        _ => println!("✗ Unexpected exit code: {}", exit_code),
    }
    
    // Check log for key errors
    if let Ok(log) = std::fs::read_to_string("C:\\temp\\custom_log.txt") {
        println!("\n=== Key log entries ===");
        for line in log.lines() {
            if line.contains("Note: 1:") || line.contains("Product:") || 
               line.contains("Installation success") || line.contains("1708") {
                println!("  {}", line.trim());
            }
        }
    }
}
