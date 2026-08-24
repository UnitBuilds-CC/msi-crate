/// Repackage execseq_test.msi using cfb crate and test if it installs.
use std::io::Read;

fn main() {
    let src_path = "execseq_test.msi";
    let dst_path = "repacked_execseq.msi";

    // Read all streams from source
    let file = std::fs::File::open(src_path).unwrap();
    let comp = cfb::CompoundFile::open(file).unwrap();
    let stream_entries: Vec<String> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();
    drop(comp);

    let file2 = std::fs::File::open(src_path).unwrap();
    let mut comp2 = cfb::CompoundFile::open(file2).unwrap();

    let mut streams: Vec<(String, Vec<u8>)> = Vec::new();
    for path in &stream_entries {
        let mut r = comp2.open_stream(path.as_str()).unwrap();
        let mut buf = Vec::new();
        r.read_to_end(&mut buf).unwrap();
        streams.push((path.clone(), buf));
    }
    drop(comp2);

    // Create new compound file with cfb
    let buf = std::io::Cursor::new(Vec::new());
    let mut new_comp = cfb::CompoundFile::create_with_version(cfb::Version::V3, buf).unwrap();
    for (path, data) in &streams {
        let clean_path = path.strip_prefix('/').unwrap_or(path);
        // Split path into components
        let parts: Vec<&str> = clean_path.split('/').collect();
        // Create parent storages if needed
        for i in 1..parts.len() {
            let storage_path = parts[..i].join("/");
            // Ignore errors if storage already exists
            let _ = new_comp.create_storage(&storage_path);
        }
        // Create and write the stream
        let mut stream = new_comp.create_stream(clean_path).unwrap();
        std::io::Write::write_all(&mut stream, data).unwrap();
    }

    let cursor = new_comp.into_inner();
    std::fs::write(dst_path, cursor.into_inner()).unwrap();

    let size = std::fs::metadata(dst_path).unwrap().len();
    println!("Repacked to {} ({} bytes)", dst_path, size);

    // Verify with msi crate
    let file4 = std::fs::File::open(dst_path).unwrap();
    match cfb::CompoundFile::open(file4) {
        Ok(comp) => {
            println!("cfb open: OK, {} streams", comp.walk().filter(|e| e.is_stream()).count());
        }
        Err(e) => println!("cfb open FAILED: {}", e),
    }

    let file5 = std::fs::File::open(dst_path).unwrap();
    match msi::Package::open(file5) {
        Ok(pkg) => {
            println!("msi open: OK");
            for table in pkg.tables() {
                println!("  Table '{}': {} cols", table.name(), table.columns().len());
            }
        }
        Err(e) => println!("msi open FAILED: {}", e),
    }
}
