/// Binary search: swap streams between system MSI and ours to find the problematic one
/// Uses our OLE writer to rebuild the MSI with swapped streams
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use velocity_msi::ole;

fn main() {
    let sys_path = "C:\\Windows\\Installer\\10d16cbb.msi";
    
    // Read all streams from system MSI
    let sys_streams = read_all_streams(sys_path);
    println!("System MSI has {} streams", sys_streams.len());
    for (name, data) in &sys_streams {
        let cps: Vec<u16> = name.encode_utf16().collect();
        let prefix = if !cps.is_empty() && cps[0] == 0x4840 { "T" } else { "N" };
        println!("  [{}] '{}' ({} cps) {} bytes", prefix, name, cps.len(), data.len());
    }
    
    // Generate our MSI
    let mut builder = velocity_msi::MsiBuilder::new();
    builder.set_title("Velocity Test App");
    builder.set_author("Velocity Installer");
    builder.set_template("Intel", 1033);
    builder.create_table(
        "Property",
        vec![
            velocity_msi::Column::build("Property").string(72).primary_key().build(),
            velocity_msi::Column::build("Value").string(255).nullable().build(),
        ],
    ).unwrap();
    builder.insert_rows("Property", vec![
        vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("Velocity Test App")],
        vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0.0")],
        vec![velocity_msi::Value::from("Manufacturer"), velocity_msi::Value::from("Velocity")],
        vec![velocity_msi::Value::from("ProductCode"), velocity_msi::Value::from("{12345678-1234-1234-1234-123456789012}")],
        vec![velocity_msi::Value::from("UpgradeCode"), velocity_msi::Value::from("{87654321-4321-4321-4321-210987654321}")],
    ]).unwrap();
    let our_data = builder.build().unwrap();
    
    // Write our MSI to temp file
    std::fs::write("target/swap_test.msi", &our_data).unwrap();
    
    // Read our streams
    let our_streams = read_all_streams("target/swap_test.msi");
    println!("\nOur MSI has {} streams", our_streams.len());
    
    // Now try: use ALL system streams but replace SummaryInformation with ours
    println!("\n=== Test: System streams + Our SummaryInformation ===");
    let our_si_name = "\u{0005}SummaryInformation";
    let mut mixed_streams: Vec<ole::OleStream> = Vec::new();
    
    for (name, data) in &sys_streams {
        if name.contains("SummaryInformation") {
            // Replace with our SummaryInformation
            if let Some((_, our_si_data)) = our_streams.iter().find(|(n, _)| n.contains("SummaryInformation")) {
                mixed_streams.push(ole::OleStream { name: name.clone(), data: our_si_data.clone() });
                println!("  Replaced SummaryInformation: {} bytes -> {} bytes", data.len(), our_si_data.len());
            }
        } else {
            mixed_streams.push(ole::OleStream { name: name.clone(), data: data.clone() });
        }
    }
    
    let mixed_data = ole::build_ole_file(&mixed_streams);
    std::fs::write("target/swap_si.msi", &mixed_data).unwrap();
    println!("Created target/swap_si.msi ({} bytes)", mixed_data.len());
    
    // Test with msiexec
    test_msiexec("target/swap_si.msi", "target/swap_si_log.txt");
    
    // Also try: ALL our streams but replace SummaryInformation with system's
    println!("\n=== Test: Our streams + System SummaryInformation ===");
    let mut mixed_streams2: Vec<ole::OleStream> = Vec::new();
    
    for (name, data) in &our_streams {
        if name.contains("SummaryInformation") {
            // Replace with system SummaryInformation
            if let Some((_, sys_si_data)) = sys_streams.iter().find(|(n, _)| n.contains("SummaryInformation")) {
                mixed_streams2.push(ole::OleStream { name: name.clone(), data: sys_si_data.clone() });
                println!("  Replaced SummaryInformation: {} bytes -> {} bytes", data.len(), sys_si_data.len());
            }
        } else {
            mixed_streams2.push(ole::OleStream { name: name.clone(), data: data.clone() });
        }
    }
    
    let mixed_data2 = ole::build_ole_file(&mixed_streams2);
    std::fs::write("target/swap_si2.msi", &mixed_data2).unwrap();
    println!("Created target/swap_si2.msi ({} bytes)", mixed_data2.len());
    
    test_msiexec("target/swap_si2.msi", "target/swap_si2_log.txt");
}

fn read_all_streams(path: &str) -> Vec<(String, Vec<u8>)> {
    let file = File::open(path).unwrap();
    let mut comp = cfb::CompoundFile::open(file).unwrap();
    let entries: Vec<(PathBuf, bool)> = comp.walk()
        .map(|e| (e.path().to_path_buf(), e.is_stream()))
        .collect();
    
    let mut streams = Vec::new();
    for (p, is_stream) in entries {
        if !is_stream { continue; }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let mut stream = comp.open_stream(&p).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        streams.push((name, data));
    }
    streams
}

fn test_msiexec(msi_path: &str, log_path: &str) {
    let _ = std::fs::remove_file(log_path);
    let status = std::process::Command::new("msiexec.exe")
        .args(&["/i", msi_path, "/qn", "/l*v", log_path])
        .status()
        .unwrap();
    let code = status.code().unwrap_or(-1);
    
    // Check log for success/failure
    if let Ok(log) = std::fs::read_to_string(log_path) {
        if log.contains("Access database") {
            println!("  Result: Database accessed! (code={})", code);
        } else if log.contains("1620") {
            println!("  Result: Error 1620 (could not be opened)");
        } else {
            println!("  Result: code={}", code);
        }
    } else {
        println!("  Result: code={} (no log)", code);
    }
}
