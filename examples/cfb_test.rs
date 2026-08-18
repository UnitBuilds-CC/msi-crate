use cfb::{CompoundFile, Version};
use std::io::{Cursor, Write, Read};
use std::path::Path;

fn main() {
    // Test: write a stream with a known Unicode name and read it back
    let cursor = Cursor::new(Vec::new());
    let mut comp = CompoundFile::create_with_version(Version::V4, cursor).unwrap();
    
    // Write stream with name containing U+3E6A
    let name1 = String::from_utf16(&[0x3F3F, 0x4577, 0x446C, 0x3E6A, 0x44B2, 0x482F]).unwrap();
    println!("Writing stream with name chars:");
    for (i, ch) in name1.chars().enumerate() {
        println!("  char[{}]: U+{:04X}", i, ch as u32);
    }
    
    let mut stream = comp.create_stream(&name1).unwrap();
    stream.write_all(b"POOL_DATA").unwrap();
    drop(stream);
    
    // Write stream with name containing U+3B6A
    let name2 = String::from_utf16(&[0x3F3F, 0x4577, 0x446C, 0x3B6A, 0x45E4, 0x4824]).unwrap();
    println!("\nWriting stream with name chars:");
    for (i, ch) in name2.chars().enumerate() {
        println!("  char[{}]: U+{:04X}", i, ch as u32);
    }
    
    let mut stream = comp.create_stream(&name2).unwrap();
    stream.write_all(b"STRING_DATA").unwrap();
    drop(stream);
    
    // Read back
    let cursor = comp.into_inner();
    let data = cursor.into_inner();
    println!("\nFile size: {} bytes", data.len());
    
    let cursor2 = Cursor::new(data);
    let mut comp2 = CompoundFile::open(cursor2).unwrap();
    
    println!("\nReading back streams:");
    let entries: Vec<_> = comp2.walk().map(|e| (e.path().to_path_buf(), e.is_stream())).collect();
    for (p, is_stream) in &entries {
        if !is_stream { continue; }
        let name = p.file_name().unwrap().to_string_lossy();
        println!("Stream '{}':", name);
        for (i, ch) in name.chars().enumerate() {
            println!("  char[{}]: U+{:04X}", i, ch as u32);
        }
        let mut stream = comp2.open_stream(p).unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).unwrap();
        println!("  Data: {:?}", String::from_utf8_lossy(&buf));
    }
}
