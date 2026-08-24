/// Create a minimal MSI using the msi crate (Package::create) and test with msiexec.
/// If this works, we know the msi crate produces valid MSIs and can compare format.
use std::io::{Cursor, Write};

fn main() {
    let ws_root = env!("CARGO_MANIFEST_DIR").to_string() + "/../..";
    let _ = std::process::Command::new("taskkill").args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    let path = format!("{}/msi_crate_test.msi", ws_root);
    
    // Create MSI using the msi crate
    println!("=== Creating MSI with msi crate ===");
    let mut buf = Cursor::new(Vec::new());
    {
        let mut pkg = msi::Package::create(msi::PackageType::Installer, &mut buf).unwrap();
        
        // Set SummaryInfo
        let si = pkg.summary_info_mut();
        si.set_title(Some("MSI Crate Test"));
        si.set_subject(Some("Test Product"));
        si.set_author(Some("V"));
        si.set_template(Some("Intel;1033"));
        si.set_comments(None);
        si.set_creating_app(Some("msi crate"));
        si.set_rev_number(Some("{247F8300-3914-44B1-B83E-E1F741507FA3}"));
        si.set_word_count(2);
        si.set_security(200);
        
        // Create Property table
        {
            let cols = vec![
                msi::Column::build("Property").string(72).primary_key().build(),
                msi::Column::build("Value").string(0).nullable().localizable().build(),
            ];
            pkg.create_table("Property", cols).unwrap();
        }
        
        // Insert property rows
        {
            let table = pkg.table_mut("Property").unwrap();
            table.insert_rows(vec![
                vec![msi::Value::Str("ProductName".into()), msi::Value::Str("MSI Crate Test".into())],
                vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())],
                vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("V".into())],
                vec![msi::Value::Str("ProductCode".into()), msi::Value::Str("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}".into())],
                vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())],
            ]).unwrap();
        }
        
        // Create Directory table
        {
            let cols = vec![
                msi::Column::build("Directory").string(72).primary_key().build(),
                msi::Column::build("Directory_Parent").string(72).nullable().build(),
                msi::Column::build("DefaultDir").string(255).primary_key().build(),
            ];
            pkg.create_table("Directory", cols).unwrap();
        }
        {
            let table = pkg.table_mut("Directory").unwrap();
            table.insert_rows(vec![
                vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str("SourceDir".into())],
            ]).unwrap();
        }
        
        // Create InstallExecuteSequence table
        {
            let cols = vec![
                msi::Column::build("Action").string(72).primary_key().build(),
                msi::Column::build("Condition").string(0).nullable().build(),
                msi::Column::build("Sequence").int16().nullable().build(),
            ];
            pkg.create_table("InstallExecuteSequence", cols).unwrap();
        }
        {
            let table = pkg.table_mut("InstallExecuteSequence").unwrap();
            table.insert_rows(vec![
                vec![msi::Value::Str("CostInitialize".into()), msi::Value::Null, msi::Value::Int(800)],
                vec![msi::Value::Str("CostFinalize".into()), msi::Value::Null, msi::Value::Int(1000)],
            ]).unwrap();
        }
        
        pkg.flush().unwrap();
    }
    
    let data = buf.into_inner();
    std::fs::write(&path, &data).unwrap();
    println!("Created: {} ({} bytes)", path, data.len());
    
    // Test with msiexec
    println!("\n=== Testing with msiexec ===");
    let log_path = format!("{}/msi_crate_test.log", ws_root);
    let output = std::process::Command::new("msiexec")
        .args(&["/i", &path, "/qn", "/norestart", "/l*v", &log_path])
        .output()
        .expect("msiexec failed");
    let code = output.status.code().unwrap_or(-1);
    println!("msiexec exit code: {}", code);
    
    if code != 0 {
        println!("FAILED. Checking log...");
        if let Ok(log) = std::fs::read_to_string(&log_path) {
            for line in log.lines() {
                if line.contains("Error") || line.contains("return value 3") {
                    println!("  {}", line.trim());
                }
            }
        }
    } else {
        println!("SUCCESS!");
        let _ = std::process::Command::new("msiexec").args(&["/x", &path, "/qn", "/norestart"]).output();
    }
    
    // Dump SummaryInfo
    println!("\n=== SummaryInfo ===");
    let mut comp = cfb::CompoundFile::open(Cursor::new(&data)).unwrap();
    let paths: Vec<_> = comp.walk()
        .filter(|e| e.is_stream() && e.path().to_string_lossy().contains("SummaryInformation"))
        .map(|e| e.path().to_path_buf())
        .collect();
    if let Some(p) = paths.first() {
        let mut stream = comp.open_stream(p).unwrap();
        let mut summary_data = Vec::new();
        stream.read_to_end(&mut summary_data).unwrap();
        println!("SummaryInfo: {} bytes", summary_data.len());
        // Dump first 64 bytes
        for i in (0..summary_data.len().min(320)).step_by(16) {
            let end = (i + 16).min(summary_data.len());
            print!("  {:04x}: ", i);
            for b in &summary_data[i..end] { print!("{:02x} ", b); }
            println!();
        }
    }
}
