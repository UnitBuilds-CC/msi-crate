/// Binary comparison: Python msilib reference MSI vs velocity-msi output.
/// Uses the cfb crate to read both MSIs and compare streams.
use std::io::{Read, Cursor};

fn main() {
    let ws_root = env!("CARGO_MANIFEST_DIR").to_string() + "/../..";
    let ref_path = format!("{}/python_ref.msi", ws_root);
    let our_path = format!("{}/velocity_comp.msi", ws_root);

    println!("=== Binary Comparison ===\n");
    println!("Reference: {} ({} bytes)", ref_path, std::fs::metadata(&ref_path).unwrap().len());
    println!("Our MSI:   {} ({} bytes)", our_path, std::fs::metadata(&our_path).unwrap().len());

    // Read both files into memory
    let ref_data = std::fs::read(ref_path).unwrap();
    let our_data = std::fs::read(our_path).unwrap();

    // Open both with cfb
    let mut ref_comp = cfb::CompoundFile::open(Cursor::new(&ref_data)).unwrap();
    let mut our_comp = cfb::CompoundFile::open(Cursor::new(&our_data)).unwrap();

    // Collect stream names and data
    let ref_streams = collect_streams(&mut ref_comp);
    let our_streams = collect_streams(&mut our_comp);

    println!("\nReference streams: {}", ref_streams.len());
    for (name, data) in &ref_streams {
        println!("  {} ({} bytes)", name, data.len());
    }

    println!("\nOur streams: {}", our_streams.len());
    for (name, data) in &our_streams {
        println!("  {} ({} bytes)", name, data.len());
    }

    // Compare
    println!("\n--- Stream comparison ---");
    let ref_names: Vec<&String> = ref_streams.iter().map(|(n, _)| n).collect();

    for (name, ref_stream_data) in &ref_streams {
        if let Some((_, our_stream_data)) = our_streams.iter().find(|(n, _)| n == name) {
            if ref_stream_data == our_stream_data {
                println!("\n{}: IDENTICAL ({} bytes)", name, ref_stream_data.len());
            } else {
                println!("\n{}: DIFFERENT", name);
                println!("  Reference: {} bytes", ref_stream_data.len());
                println!("  Our:       {} bytes", our_stream_data.len());

                let min_len = ref_stream_data.len().min(our_stream_data.len());
                for i in 0..min_len {
                    if ref_stream_data[i] != our_stream_data[i] {
                        println!("  First diff at byte {} (0x{:x})", i, i);
                        let start = i.saturating_sub(8);
                        let end_ref = (i + 48).min(ref_stream_data.len());
                        let end_our = (i + 48).min(our_stream_data.len());
                        print!("  Ref: ");
                        for b in &ref_stream_data[start..end_ref] { print!("{:02x} ", b); }
                        println!();
                        print!("  Our: ");
                        for b in &our_stream_data[start..end_our] { print!("{:02x} ", b); }
                        println!();
                        break;
                    }
                }
                if min_len > 0 && ref_stream_data[..min_len] == our_stream_data[..min_len] && ref_stream_data.len() != our_stream_data.len() {
                    println!("  Same content up to byte {}, different lengths", min_len);
                }
            }
        } else {
            println!("\n{}: ONLY IN REFERENCE ({} bytes)", name, ref_stream_data.len());
            let show = ref_stream_data.len().min(80);
            print!("  ");
            for b in &ref_stream_data[..show] { print!("{:02x} ", b); }
            println!();
        }
    }

    for (name, data) in &our_streams {
        if !ref_names.contains(&name) {
            println!("\n{}: ONLY IN OURS ({} bytes)", name, data.len());
            let show = data.len().min(80);
            print!("  ");
            for b in &data[..show] { print!("{:02x} ", b); }
            println!();
        }
    }
}

fn collect_streams(comp: &mut cfb::CompoundFile<Cursor<&Vec<u8>>>) -> Vec<(String, Vec<u8>)> {
    let mut streams = Vec::new();
    let paths: Vec<_> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.path().to_path_buf())
        .collect();
    
    for path in &paths {
        let mut stream = comp.open_stream(path).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        let name = path.to_string_lossy().to_string();
        streams.push((name, data));
    }
    streams
}
