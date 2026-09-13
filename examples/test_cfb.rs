use std::fs;
use std::io::Cursor;

fn main() {
    let data = fs::read("vel_struct.msi").unwrap();
    println!("File size: {} bytes", data.len());
    
    match cfb::CompoundFile::open(Cursor::new(&data)) {
        Ok(comp) => {
            println!("OLE structure is VALID");
            println!("Root entry: {:?}", comp.root_entry().name());
            let mut count = 0;
            for entry in comp.walk() {
                count += 1;
                if count <= 20 {
                    println!("  Entry: {} (is_storage: {}, size: {})", 
                             entry.name(), 
                             entry.is_storage(),
                             entry.len());
                }
            }
            println!("Total entries: {}", count);
        }
        Err(e) => {
            println!("OLE structure is INVALID: {}", e);
        }
    }
}
