/// Use Windows Installer COM API to validate MSI databases
/// This bypasses msiexec's caching requirements
use std::fs::File;
use std::io::{Cursor, Read, Write as IoWrite};
use std::path::PathBuf;

fn main() {
    let sys_path = "C:\\Windows\\Installer\\10d16cbb.msi";
    let temp = std::env::temp_dir();
    
    // Read all streams from system MSI
    let file = File::open(sys_path).unwrap();
    let mut comp = cfb::CompoundFile::open(file).unwrap();
    let entries: Vec<(PathBuf, bool)> = comp.walk()
        .map(|e| (e.path().to_path_buf(), e.is_stream()))
        .collect();
    
    let mut all_streams: Vec<(String, Vec<u8>)> = Vec::new();
    for (p, is_stream) in entries {
        if !is_stream { continue; }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let mut stream = comp.open_stream(&p).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        all_streams.push((name, data));
    }
    println!("Read {} streams from system MSI", all_streams.len());
    
    // Content streams only (no signatures)
    let content_streams: Vec<_> = all_streams.iter()
        .filter(|(n, _)| !n.contains("DigitalSignature") && !n.contains("MsiDigitalSignature"))
        .collect();
    println!("Content streams (no sig): {}", content_streams.len());
    
    // Build test files
    println!("\n=== Building test files ===");
    
    // 1. cfb roundtrip (no sig)
    let cfb_path = temp.join("validate_cfb.msi");
    {
        let cursor = Cursor::new(Vec::new());
        let mut out = cfb::OpenOptions::new().create_with(cursor).unwrap();
        for (name, data) in &content_streams {
            let path = format!("/{}", name);
            let mut s = out.create_stream(&path).unwrap();
            s.write_all(data).unwrap();
        }
        let cursor = out.into_inner();
        std::fs::write(&cfb_path, cursor.into_inner()).unwrap();
        println!("cfb no-sig: {} bytes", std::fs::metadata(&cfb_path).unwrap().len());
    }
    
    // 2. Our OLE (no sig)
    let our_path = temp.join("validate_our.msi");
    {
        let ole_streams: Vec<velocity_msi::ole::OleStream> = content_streams.iter()
            .map(|(name, data)| velocity_msi::ole::OleStream {
                name: name.clone(),
                data: data.clone(),
            })
            .collect();
        let our_data = velocity_msi::ole::build_ole_file(&ole_streams);
        std::fs::write(&our_path, &our_data).unwrap();
        println!("our no-sig: {} bytes", our_data.len());
    }
    
    // 3. Our generated MSI
    let gen_path = temp.join("validate_gen.msi");
    {
        let mut builder = velocity_msi::MsiBuilder::new();
        builder.set_title("Velocity Test");
        builder.set_author("Velocity");
        builder.set_template("Intel", 1033);
        builder.create_table(
            "Property",
            vec![
                velocity_msi::Column::build("Property").string(72).primary_key().build(),
                velocity_msi::Column::build("Value").string(255).nullable().build(),
            ],
        ).unwrap();
        builder.insert_rows("Property", vec![
            vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("Velocity Test")],
            vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0.0")],
            vec![velocity_msi::Value::from("Manufacturer"), velocity_msi::Value::from("Velocity")],
            vec![velocity_msi::Value::from("ProductCode"), velocity_msi::Value::from("{12345678-1234-1234-1234-123456789012}")],
            vec![velocity_msi::Value::from("UpgradeCode"), velocity_msi::Value::from("{87654321-4321-4321-4321-210987654321}")],
        ]).unwrap();
        let gen_data = builder.build().unwrap();
        std::fs::write(&gen_path, &gen_data).unwrap();
        println!("generated: {} bytes", gen_data.len());
    }
    
    // Now validate using PowerShell COM API
    println!("\n=== COM API Validation ===");
    
    // Test original
    println!("\n--- Original system MSI ---");
    validate_com(sys_path);
    
    // Test cfb roundtrip
    println!("\n--- cfb roundtrip (no sig) ---");
    validate_com(cfb_path.to_str().unwrap());
    
    // Test our OLE
    println!("\n--- Our OLE (no sig) ---");
    validate_com(our_path.to_str().unwrap());
    
    // Test our generated
    println!("\n--- Our generated MSI ---");
    validate_com(gen_path.to_str().unwrap());
    
    // Also try to read database properties via COM
    println!("\n=== Database Property Query ===");
    query_properties(cfb_path.to_str().unwrap(), "cfb");
    query_properties(our_path.to_str().unwrap(), "our");
    query_properties(gen_path.to_str().unwrap(), "gen");
}

fn validate_com(path: &str) {
    // Use PowerShell to call MsiOpenDatabase
    let ps_script = format!(
        r#"Add-Type -TypeDefinition '
using System;
using System.Runtime.InteropServices;
public class MsiValidator {{
    [DllImport("msi.dll", CharSet = CharSet.Unicode)]
    public static extern int MsiOpenDatabaseW(string dbPath, IntPtr persist, out IntPtr hDb);
    [DllImport("msi.dll")]
    public static extern int MsiCloseHandle(IntPtr hAny);
    [DllImport("msi.dll", CharSet = CharSet.Unicode)]
    public static extern int MsiDatabaseGetPrimaryKeysW(IntPtr hDb, string table, out IntPtr hRecord);
}}' -ErrorAction SilentlyContinue
$hDb = [IntPtr]::Zero
$result = [MsiValidator]::MsiOpenDatabaseW('{}', [IntPtr]::Zero, [ref]$hDb)
Write-Output "MsiOpenDatabaseW result: $result"
if ($result -eq 0) {{
    Write-Output "SUCCESS: Database opened"
    [MsiValidator]::MsiCloseHandle($hDb) | Out-Null
}} else {{
    Write-Output "FAILED: Error code $result"
}}"#,
        path.replace('\\', "\\\\")
    );
    
    let output = std::process::Command::new("powershell")
        .args(&["-NoProfile", "-Command", &ps_script])
        .output()
        .unwrap();
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    for line in stdout.lines() {
        println!("  {}", line.trim());
    }
    if !stderr.is_empty() {
        for line in stderr.lines().take(3) {
            println!("  ERR: {}", line.trim());
        }
    }
}

fn query_properties(path: &str, label: &str) {
    // Try to open with cfb and read SummaryInformation
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => { println!("  {}: Cannot open file: {}", label, e); return; }
    };
    let mut comp = match cfb::CompoundFile::open(file) {
        Ok(c) => c,
        Err(e) => { println!("  {}: Cannot open as OLE: {}", label, e); return; }
    };
    
    // Try to read SummaryInformation
    let si_path = "/\u{0005}SummaryInformation";
    match comp.open_stream(si_path) {
        Ok(mut s) => {
            let mut data = Vec::new();
            s.read_to_end(&mut data).unwrap();
            println!("  {}: SummaryInformation = {} bytes", label, data.len());
            
            // Parse header
            if data.len() >= 48 {
                let bom = u16::from_le_bytes([data[0], data[1]]);
                let version = u16::from_le_bytes([data[2], data[3]]);
                let os_low = u16::from_le_bytes([data[4], data[5]]);
                let os_high = u16::from_le_bytes([data[6], data[7]]);
                let section_count = u32::from_le_bytes([data[32], data[33], data[34], data[35]]);
                println!("    BOM=0x{:04X} ver=0x{:04X} OS={}.{} sections={}", 
                    bom, version, os_low, os_high, section_count);
            }
        }
        Err(e) => println!("  {}: No SummaryInformation: {}", label, e),
    }
    
    // Count table streams
    let entries: Vec<_> = comp.walk().collect();
    let table_count = entries.iter()
        .filter(|e| e.is_stream() && e.path().file_name()
            .map(|n| n.to_string_lossy().encode_utf16().next() == Some(0x4840))
            .unwrap_or(false))
        .count();
    println!("  {}: {} table streams", label, table_count);
}
