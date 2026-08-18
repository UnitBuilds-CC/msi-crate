use cfb::CompoundFile;
use std::fs::File;
use std::io::{Read, Write};

fn main() {
    // Copy the system MSI
    std::fs::copy(
        "c:/Users/visse/OneDrive/Documentos/rust-msi/target/system_sample.msi",
        "target/modified_system.msi"
    ).unwrap();
    
    // Open the copy for modification
    let file = File::open("target/modified_system.msi").unwrap();
    let mut comp = CompoundFile::open(file).unwrap();
    
    // Add a dummy stream
    let mut stream = comp.create_stream("TestStream").unwrap();
    stream.write_all(b"test data").unwrap();
    drop(stream);
    
    // Save back - need to get inner and write to file
    // Actually cfb modifies in place when opened from a file
    println!("Modified system MSI saved");
}
