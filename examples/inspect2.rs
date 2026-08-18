use cfb::CompoundFile;
use std::fs::File;
use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = if args.len() > 1 { &args[1] } else { "target/test_velocity_msi.msi" };
    
    let file = File::open(path).unwrap();
    let mut comp = CompoundFile::open(file).unwrap();
    
    // Collect stream paths first
    let streams: Vec<_> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.path().to_owned())
        .collect();
    
    println!("=== Streams in {} ===", path);
    for p in &streams {
        let mut stream = comp.open_stream(p).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        
        let name_str = p.to_string_lossy();
        let hex_name: Vec<String> = name_str.chars().map(|c| format!("U+{:04X}", c as u32)).collect();
        println!("  {} : {} bytes", hex_name.join(" "), data.len());
        
        if !data.is_empty() {
            print!("    ");
            for b in data.iter().take(48) {
                print!("{:02X} ", b);
            }
            if data.len() > 48 { print!("..."); }
            println!();
        }
    }
}
