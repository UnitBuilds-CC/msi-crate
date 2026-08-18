use cfb::CompoundFile;
use std::fs::File;
use std::io::{Cursor, Write};

fn main() {
    // Create a minimal OLE V4 file with cfb
    let cursor = Cursor::new(Vec::new());
    let mut comp = CompoundFile::create_with_version(cfb::Version::V4, cursor).unwrap();
    
    // Set MSI CLSID
    let clsid = uuid::Uuid::parse_str("000c1084-0000-0000-c000-000000000046").unwrap();
    comp.set_storage_clsid("/", clsid).unwrap();
    
    // Add a simple stream
    let mut s = comp.create_stream("TestStream").unwrap();
    s.write_all(b"Hello, World!").unwrap();
    drop(s);
    
    let cursor = comp.into_inner();
    let data = cursor.into_inner();
    std::fs::write("target/cfb_test.msi", &data).unwrap();
    println!("Created cfb_test.msi ({} bytes)", data.len());
    
    // Dump header
    println!("\nHeader bytes (first 80):");
    for i in (0..80).step_by(16) {
        print!("  {:3}: ", i);
        for j in 0..16 {
            if i + j < data.len() {
                print!("{:02X} ", data[i + j]);
            }
        }
        println!();
    }
    
    // Parse key fields
    println!("\nParsed header:");
    println!("  Major version: {}", u16::from_le_bytes([data[26], data[27]]));
    println!("  Sector shift: {}", u16::from_le_bytes([data[30], data[31]]));
    println!("  Dir sectors: {}", u32::from_le_bytes([data[40], data[41], data[42], data[43]]));
    println!("  FAT sectors: {}", u32::from_le_bytes([data[44], data[45], data[46], data[47]]));
    println!("  First dir: {}", u32::from_le_bytes([data[48], data[49], data[50], data[51]]));
    println!("  First miniFAT: {}", u32::from_le_bytes([data[60], data[61], data[62], data[63]]));
    println!("  DIFAT[0]: {}", u32::from_le_bytes([data[76], data[77], data[78], data[79]]));
    
    // Check directory entry
    let dir_off = 4096; // sector 0 for V4
    println!("\nDirectory entry 0:");
    println!("  Type: {}", data[dir_off + 66]);
    println!("  Child: {}", i32::from_le_bytes([data[dir_off+76], data[dir_off+77], data[dir_off+78], data[dir_off+79]]));
    print!("  CLSID: ");
    for i in 0..16 { print!("{:02X}", data[dir_off + 80 + i]); }
    println!();
    
    // Now verify cfb can read it back
    let file = File::open("target/cfb_test.msi").unwrap();
    let mut comp2 = CompoundFile::open(file).unwrap();
    let entries: Vec<_> = comp2.walk().map(|e| e.path().to_path_buf()).collect();
    println!("\ncfb read back {} entries:", entries.len());
    for p in &entries {
        println!("  {}", p.display());
    }
}
