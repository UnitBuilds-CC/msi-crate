use cfb::CompoundFile;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

fn main() {
    let file = File::open("target/test_velocity.msi").unwrap();
    match CompoundFile::open(file) {
        Ok(mut comp) => {
            println!("cfb opened successfully!");
            let entries: Vec<(PathBuf, bool)> = comp.walk()
                .map(|e| (e.path().to_path_buf(), e.is_stream()))
                .collect();
            println!("Found {} entries", entries.len());
            for (p, is_stream) in &entries {
                if *is_stream {
                    let name = p.file_name().unwrap().to_string_lossy().to_string();
                    let mut stream = comp.open_stream(p).unwrap();
                    let mut data = Vec::new();
                    stream.read_to_end(&mut data).unwrap();
                    println!("  Stream '{}' ({} bytes)", name, data.len());
                } else {
                    println!("  Storage '{}'", p.display());
                }
            }
        }
        Err(e) => println!("cfb FAILED: {}", e),
    }
}
