/// Binary comparison: custom OLE writer vs cfb crate
/// Build identical MSI streams, wrap with each, compare byte-by-byte
use velocity_msi::{MsiBuilder, Column, Value};

fn main() {
    println!("=== OLE Writer Binary Comparison ===\n");

    // Build a minimal MSI using the custom OLE writer (current lib.rs)
    let mut builder = MsiBuilder::new();
    builder.set_title("Test");
    builder.set_author("Test");
    builder.set_template("Intel", 1033);

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Test")],
        vec![Value::from("ProductCode"), Value::from("{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}")],
        vec![Value::from("UpgradeCode"), Value::from("{11111111-2222-3333-4444-555555555555}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    let custom_data = builder.build().unwrap();
    println!("Custom OLE: {} bytes", custom_data.len());

    // Write custom OLE output
    std::fs::write("C:\\temp\\custom_ole.msi", &custom_data).unwrap();

    // Now build the same MSI using cfb
    // We need to use the msi crate to extract streams from custom, then repack with cfb
    // Actually, let's just use cfb directly with the same stream data
    
    // Extract streams from the custom OLE file using msi crate
    let cursor = std::io::Cursor::new(&custom_data);
    let pkg = msi::Package::open(cursor).expect("Failed to open custom MSI");
    
    // Collect all stream names and data
    let mut streams: Vec<(String, Vec<u8>)> = Vec::new();
    for table in pkg.tables() {
        let name = table.name().to_string();
        // We can't easily extract the raw stream data from msi crate
        // Let me try a different approach
    }

    // Different approach: build the OLE file with cfb using the same stream-building logic
    // For this, I need to replicate what build() does but use cfb
    
    // Actually, let me just hex-dump the header and first directory sector
    println!("\n=== Custom OLE Header (first 80 bytes) ===");
    for i in (0..80).step_by(16) {
        print!("{:04X}: ", i);
        for j in 0..16 {
            print!("{:02X} ", custom_data[i + j]);
        }
        println!();
    }

    println!("\n=== Key header fields ===");
    println!("Minor version:    {}", u16::from_le_bytes([custom_data[24], custom_data[25]]));
    println!("Major version:    {}", u16::from_le_bytes([custom_data[26], custom_data[27]]));
    println!("Byte order:       {:04X}", u16::from_le_bytes([custom_data[28], custom_data[29]]));
    println!("Sector shift:     {} ({} bytes)", u16::from_le_bytes([custom_data[30], custom_data[31]]), 
        1u32 << u16::from_le_bytes([custom_data[30], custom_data[31]]));
    println!("Mini sect shift:  {} ({} bytes)", u16::from_le_bytes([custom_data[32], custom_data[33]]),
        1u32 << u16::from_le_bytes([custom_data[32], custom_data[33]]));
    println!("Num dir sectors:  {}", u32::from_le_bytes([custom_data[40], custom_data[41], custom_data[42], custom_data[43]]));
    println!("Num FAT sectors:  {}", u32::from_le_bytes([custom_data[44], custom_data[45], custom_data[46], custom_data[47]]));
    println!("First dir sector: {}", u32::from_le_bytes([custom_data[48], custom_data[49], custom_data[50], custom_data[51]]));
    println!("Mini stream cut:  {}", u32::from_le_bytes([custom_data[56], custom_data[57], custom_data[58], custom_data[59]]));
    println!("First MiniFAT:    {}", u32::from_le_bytes([custom_data[60], custom_data[61], custom_data[62], custom_data[63]]));
    println!("Num MiniFAT:      {}", u32::from_le_bytes([custom_data[64], custom_data[65], custom_data[66], custom_data[67]]));
    println!("First DIFAT:      {}", u32::from_le_bytes([custom_data[68], custom_data[69], custom_data[70], custom_data[71]]));
    println!("Num DIFAT:        {}", u32::from_le_bytes([custom_data[72], custom_data[73], custom_data[74], custom_data[75]]));

    // DIFAT array starts at offset 76
    println!("\n=== DIFAT array (first 10 entries) ===");
    for i in 0..10 {
        let off = 76 + i * 4;
        let val = u32::from_le_bytes([custom_data[off], custom_data[off+1], custom_data[off+2], custom_data[off+3]]);
        if val != 0xFFFFFFFF {
            println!("  DIFAT[{}]: {} (FAT sector {})", i, val, val);
        }
    }

    // First directory sector
    let first_dir = u32::from_le_bytes([custom_data[48], custom_data[49], custom_data[50], custom_data[51]]) as usize;
    let dir_off = 512 + first_dir * 512;
    println!("\n=== Directory sector at offset {} ===", dir_off);
    
    // Parse directory entries
    for i in 0..4 {
        let entry_off = dir_off + i * 128;
        if entry_off + 128 > custom_data.len() { break; }
        let entry = &custom_data[entry_off..entry_off + 128];
        
        // Name (UTF-16LE, up to 32 chars)
        let name_size = u16::from_le_bytes([entry[64], entry[65]]) as usize;
        let mut name = String::new();
        for j in 0..(name_size / 2).min(32) {
            let ch = u16::from_le_bytes([entry[j * 2], entry[j * 2 + 1]]);
            if ch == 0 { break; }
            if let Some(c) = char::from_u32(ch as u32) { name.push(c); }
        }
        
        let obj_type = entry[66];
        let color = entry[67];
        let left = i32::from_le_bytes([entry[68], entry[69], entry[70], entry[71]]);
        let right = i32::from_le_bytes([entry[72], entry[73], entry[74], entry[75]]);
        let child = i32::from_le_bytes([entry[76], entry[77], entry[78], entry[79]]);
        let clsid = &entry[80..96];
        let start = u32::from_le_bytes([entry[116], entry[117], entry[118], entry[119]]);
        let size = u32::from_le_bytes([entry[120], entry[121], entry[122], entry[123]]);
        
        println!("  Entry {}: name='{}' type={} color={} L={} R={} C={} start={} size={}", 
            i, name, obj_type, color, left, right, child, start, size);
        println!("    CLSID: {:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            clsid[0], clsid[1], clsid[2], clsid[3], clsid[4], clsid[5], clsid[6], clsid[7],
            clsid[8], clsid[9], clsid[10], clsid[11], clsid[12], clsid[13], clsid[14], clsid[15]);
    }

    // Test with msiexec
    println!("\n=== msiexec test ===");
    let status = std::process::Command::new("msiexec")
        .args(&["/i", "C:\\temp\\custom_ole.msi", "/qn", "/norestart", "/l*v", "C:\\temp\\custom_ole.log"])
        .status();
    let code = status.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
    println!("msiexec exit code: {}", code);
    
    if code != 0 {
        // Read log file
        if let Ok(log) = std::fs::read_to_string("C:\\temp\\custom_ole.log") {
            // Find error lines
            for line in log.lines() {
                if line.contains("Error") || line.contains("error") || line.contains("return value 3") {
                    println!("  LOG: {}", line);
                }
            }
        }
    }
}
