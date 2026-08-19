use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

fn main() {
    println!("=== Comparing Real MSI vs Our MSI ===\n");
    
    // Read real MSI
    let real_streams = read_streams("C:\\Windows\\Installer\\10d16cbb.msi");
    println!("Real MSI: {} streams", real_streams.len());
    
    // Read our MSI
    let our_streams = read_streams("test_ultra.msi");
    println!("Our MSI: {} streams\n", our_streams.len());
    
    // Compare stream counts
    println!("=== Stream Count Comparison ===");
    println!("Real: {} streams", real_streams.len());
    println!("Ours: {} streams", our_streams.len());
    
    // Find _Tables stream in both
    let tables_name = "\u{4840}\u{3f7f}\u{4164}\u{422f}\u{4836}";
    
    println!("\n=== _Tables Stream Comparison ===");
    if let Some((_, real_tables)) = real_streams.iter().find(|(n, _)| n == tables_name) {
        println!("Real _Tables: {} bytes", real_tables.len());
        print!("  Data: ");
        for b in real_tables.iter().take(32) { print!("{:02x} ", b); }
        println!();
        
        // Calculate row count (assuming 2-byte string refs)
        let row_count = real_tables.len() / 2;
        println!("  Row count: {} (assuming 2-byte refs)", row_count);
    }
    
    if let Some((_, our_tables)) = our_streams.iter().find(|(n, _)| n == tables_name) {
        println!("Our _Tables: {} bytes", our_tables.len());
        print!("  Data: ");
        for b in our_tables.iter().take(32) { print!("{:02x} ", b); }
        println!();
        
        let row_count = our_tables.len() / 2;
        println!("  Row count: {} (assuming 2-byte refs)", row_count);
    }
    
    // Find _Columns stream in both
    let columns_name = "\u{4840}\u{3b3f}\u{43f2}\u{4438}\u{45b1}";
    
    println!("\n=== _Columns Stream Comparison ===");
    if let Some((_, real_cols)) = real_streams.iter().find(|(n, _)| n == columns_name) {
        println!("Real _Columns: {} bytes", real_cols.len());
        print!("  First 64 bytes: ");
        for b in real_cols.iter().take(64) { print!("{:02x} ", b); }
        println!();
    }
    
    if let Some((_, our_cols)) = our_streams.iter().find(|(n, _)| n == columns_name) {
        println!("Our _Columns: {} bytes", our_cols.len());
        print!("  First 64 bytes: ");
        for b in our_cols.iter().take(64) { print!("{:02x} ", b); }
        println!();
    }
    
    // Compare string pools
    let pool_name = "\u{4840}\u{3F3F}\u{4577}\u{446C}\u{3E6A}\u{44B2}\u{482F}";
    
    println!("\n=== String Pool Comparison ===");
    if let Some((_, real_pool)) = real_streams.iter().find(|(n, _)| n == pool_name) {
        println!("Real _StringPool: {} bytes", real_pool.len());
        if real_pool.len() >= 4 {
            let cp = u32::from_le_bytes([real_pool[0], real_pool[1], real_pool[2], real_pool[3]]);
            let long_refs = (cp & 0x80000000) != 0;
            let cp_val = cp & 0x7FFFFFFF;
            println!("  Codepage: {} (long_refs={})", cp_val, long_refs);
            
            // Count entries
            let entry_count = (real_pool.len() - 4) / 4;
            println!("  Entry count: {}", entry_count);
        }
    }
    
    if let Some((_, our_pool)) = our_streams.iter().find(|(n, _)| n == pool_name) {
        println!("Our _StringPool: {} bytes", our_pool.len());
        if our_pool.len() >= 4 {
            let cp = u32::from_le_bytes([our_pool[0], our_pool[1], our_pool[2], our_pool[3]]);
            let long_refs = (cp & 0x80000000) != 0;
            let cp_val = cp & 0x7FFFFFFF;
            println!("  Codepage: {} (long_refs={})", cp_val, long_refs);
            
            let entry_count = (our_pool.len() - 4) / 4;
            println!("  Entry count: {}", entry_count);
        }
    }
    
    // List all stream names
    println!("\n=== Stream Names ===");
    println!("Real MSI streams:");
    for (name, data) in &real_streams {
        let cps: Vec<u16> = name.encode_utf16().collect();
        let prefix = if !cps.is_empty() && cps[0] == 0x4840 { "T" } else { "N" };
        println!("  [{}] {} ({} bytes)", prefix, name, data.len());
    }
    
    println!("\nOur MSI streams:");
    for (name, data) in &our_streams {
        let cps: Vec<u16> = name.encode_utf16().collect();
        let prefix = if !cps.is_empty() && cps[0] == 0x4840 { "T" } else { "N" };
        println!("  [{}] {} ({} bytes)", prefix, name, data.len());
    }
}

fn read_streams(path: &str) -> Vec<(String, Vec<u8>)> {
    let mut file = File::open(path).unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    
    let cursor = std::io::Cursor::new(&data);
    let mut comp = cfb::CompoundFile::open(cursor).unwrap();
    
    let entries: Vec<(PathBuf, bool)> = comp.walk()
        .map(|e| (e.path().to_path_buf(), e.is_stream()))
        .collect();
    
    let mut streams = Vec::new();
    for (path, is_stream) in entries {
        if !is_stream { continue; }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let mut stream = comp.open_stream(&path).unwrap();
        let mut stream_data = Vec::new();
        stream.read_to_end(&mut stream_data).unwrap();
        streams.push((name, stream_data));
    }
    streams
}
