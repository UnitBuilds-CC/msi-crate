/// Test: open our MSI with msi crate, save it, test if the re-saved version works.
/// This isolates whether the issue is in our OLE writer or our table data.
/// cargo run --example roundtrip_test -p velocity-msi
use std::io::Cursor;

fn main() {
    let src = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\installer.msi";
    let dst = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\roundtrip.msi";
    
    let msi_data = std::fs::read(src).expect("read MSI");
    println!("Original size: {} bytes", msi_data.len());
    
    // Open with msi crate
    let cursor = Cursor::new(msi_data.clone());
    let mut pkg = msi::Package::open(cursor).expect("open MSI");
    
    // Read all tables and print summary
    println!("\n=== Tables ===");
    for row in pkg.select_rows(msi::Select::table("_Tables")).expect("read _Tables") {
        let name = row[0].as_str().unwrap_or("?");
        println!("  {}", name);
    }
    
    // Check Component table data
    println!("\n=== Component table ===");
    for row in pkg.select_rows(msi::Select::table("Component")).expect("read Component") {
        let comp = row[0].as_str().unwrap_or("?");
        let id = row[1].as_str().map(|s| s.to_string()).unwrap_or("Null".to_string());
        let dir = row[2].as_str().unwrap_or("?");
        let attrs = row[3].as_int().unwrap_or(-9999);
        let cond = row[4].as_str().map(|s| s.to_string()).unwrap_or("Null".to_string());
        let keypath = row[5].as_str().map(|s| s.to_string()).unwrap_or("Null".to_string());
        println!("  {} | id={} | dir={} | attrs={} | cond={} | keypath={}", comp, id, dir, attrs, cond, keypath);
    }
    
    // Check Feature table
    println!("\n=== Feature table ===");
    for row in pkg.select_rows(msi::Select::table("Feature")).expect("read Feature") {
        let feat = row[0].as_str().unwrap_or("?");
        let parent = row[1].as_str().map(|s| s.to_string()).unwrap_or("Null".to_string());
        let level = row[5].as_int().unwrap_or(-9999);
        let attrs = row[7].as_int().unwrap_or(-9999);
        println!("  {} | parent={} | level={} | attrs={}", feat, parent, level, attrs);
    }
    
    // Check FeatureComponents
    println!("\n=== FeatureComponents (first 5) ===");
    let mut count = 0;
    for row in pkg.select_rows(msi::Select::table("FeatureComponents")).expect("read FC") {
        if count >= 5 { break; }
        println!("  {} -> {}", row[0].as_str().unwrap_or("?"), row[1].as_str().unwrap_or("?"));
        count += 1;
    }
    
    // Save via msi crate
    let mut output = Cursor::new(Vec::new());
    pkg.write(&mut output).expect("write MSI");
    let resaved = output.into_inner();
    std::fs::write(dst, &resaved).expect("write file");
    println!("\nRe-saved size: {} bytes", resaved.len());
    println!("Written to: {}", dst);
    
    // Now compare stream-by-stream
    println!("\n=== Binary comparison ===");
    let cursor2 = Cursor::new(msi_data.clone());
    let pkg_orig = msi::Package::open(cursor2).expect("open orig");
    let cursor3 = Cursor::new(resaved.clone());
    let pkg_new = msi::Package::open(cursor3).expect("open new");
    
    // Compare _StringPool
    let orig_pool = get_stream_data(&msi_data, "_StringPool");
    let new_pool = get_stream_data(&resaved, "_StringPool");
    println!("_StringPool: orig={} bytes, new={} bytes, match={}", 
        orig_pool.as_ref().map(|d| d.len()).unwrap_or(0),
        new_pool.as_ref().map(|d| d.len()).unwrap_or(0),
        orig_pool == new_pool);
    
    // Compare _StringData
    let orig_data = get_stream_data(&msi_data, "_StringData");
    let new_data = get_stream_data(&resaved, "_StringData");
    println!("_StringData: orig={} bytes, new={} bytes, match={}", 
        orig_data.as_ref().map(|d| d.len()).unwrap_or(0),
        new_data.as_ref().map(|d| d.len()).unwrap_or(0),
        orig_data == new_data);
    
    // If string pools differ, show first difference
    if let (Some(orig), Some(new)) = (&orig_pool, &new_pool) {
        if orig != new {
            println!("\n_StringPool DIFFERS!");
            let min_len = orig.len().min(new.len());
            for i in 0..min_len {
                if orig[i] != new[i] {
                    println!("  First diff at byte {}: orig=0x{:02X}, new=0x{:02X}", i, orig[i], new[i]);
                    // Show context
                    let start = i.saturating_sub(8);
                    let end = (i + 16).min(min_len);
                    print!("  orig[{}..{}]: ", start, end);
                    for b in &orig[start..end] { print!("{:02X} ", b); }
                    println!();
                    print!("  new [{}..{}]: ", start, end);
                    for b in &new[start..end] { print!("{:02X} ", b); }
                    println!();
                    break;
                }
            }
            if orig.len() != new.len() {
                println!("  Length diff: {} vs {}", orig.len(), new.len());
            }
        }
    }
    
    if let (Some(orig), Some(new)) = (&orig_data, &new_data) {
        if orig != new {
            println!("\n_StringData DIFFERS!");
            let min_len = orig.len().min(new.len());
            for i in 0..min_len {
                if orig[i] != new[i] {
                    println!("  First diff at byte {}: orig=0x{:02X}, new=0x{:02X}", i, orig[i], new[i]);
                    let start = i.saturating_sub(16);
                    let end = (i + 32).min(min_len);
                    print!("  orig[{}..{}]: ", start, end);
                    for b in &orig[start..end] { print!("{:02X} ", b); }
                    println!();
                    print!("  new [{}..{}]: ", start, end);
                    for b in &new[start..end] { print!("{:02X} ", b); }
                    println!();
                    break;
                }
            }
        }
    }
}

fn get_stream_data(msi_data: &[u8], stream_name: &str) -> Option<Vec<u8>> {
    // Use the cfb crate to read the OLE file and extract a stream
    let cursor = Cursor::new(msi_data);
    match cfb::CompoundFile::open(cursor) {
        Ok(mut compound) => {
            // Try to find the stream with encoded name
            // MSI encodes stream names - let's try to find it
            let entries: Vec<_> = compound.walk()
                .filter(|e| e.is_stream())
                .map(|e| e.name().to_string())
                .collect();
            
            // Try the exact name first
            let path = format!("/{}", stream_name);
            if let Ok(stream) = compound.open_stream(&path) {
                return Some(stream.read_all());
            }
            
            // List all entries for debugging
            if stream_name == "_StringPool" {
                println!("  Available streams: {:?}", entries);
            }
            None
        }
        Err(e) => {
            println!("  CFB error: {}", e);
            None
        }
    }
}
