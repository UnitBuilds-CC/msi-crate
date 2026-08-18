/// Diagnose MSI content issues
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

fn main() {
    println!("=== OUR MSI ===");
    let our_streams = read_streams("target/test_velocity.msi");
    for (name, data) in &our_streams {
        let cps: Vec<u16> = name.encode_utf16().collect();
        let prefix = if !cps.is_empty() && cps[0] == 0x4840 { "T" } else { "N" };
        println!("  [{}] '{}' ({} cps) {} bytes", prefix, name, cps.len(), data.len());
    }

    println!("\n=== SYSTEM MSI ===");
    let sys_streams = read_streams("C:\\Windows\\Installer\\10d16cbb.msi");
    for (name, data) in &sys_streams {
        let cps: Vec<u16> = name.encode_utf16().collect();
        let prefix = if !cps.is_empty() && cps[0] == 0x4840 { "T" } else { "N" };
        println!("  [{}] '{}' ({} cps) {} bytes", prefix, name, cps.len(), data.len());
    }

    // Find and compare _StringPool streams
    println!("\n=== STRING POOL COMPARISON ===");
    let pool_name = "\u{4840}\u{3F3F}\u{4577}\u{446C}\u{3E6A}\u{44B2}\u{482F}";
    let data_name = "\u{4840}\u{3F3F}\u{4577}\u{446C}\u{3B6A}\u{45E4}\u{4824}";

    for (label, streams) in &[("OUR", &our_streams), ("SYS", &sys_streams)] {
        for (name, data) in streams.iter() {
            if name == pool_name {
                println!("\n{} _StringPool ({} bytes):", label, data.len());
                dump_sp(data, 20);
            }
            if name == data_name {
                println!("\n{} _StringData ({} bytes):", label, data.len());
                dump_hex(data, 128);
            }
        }
    }

    // Compare _Tables streams
    println!("\n=== _Tables STREAMS ===");
    let tables_name = "\u{4840}\u{3f7f}\u{4164}\u{422f}\u{4836}";
    for (label, streams) in &[("OUR", &our_streams), ("SYS", &sys_streams)] {
        for (name, data) in streams.iter() {
            if name == tables_name {
                println!("{} _Tables: {} bytes", label, data.len());
                dump_hex(data, 64);
            }
        }
    }

    // Compare _Columns streams
    println!("\n=== _Columns STREAMS ===");
    let columns_name = "\u{4840}\u{3b3f}\u{43f2}\u{4438}\u{45b1}";
    for (label, streams) in &[("OUR", &our_streams), ("SYS", &sys_streams)] {
        for (name, data) in streams.iter() {
            if name == columns_name {
                println!("{} _Columns: {} bytes", label, data.len());
                dump_hex(data, 128);
            }
        }
    }

    // Compare _Validation streams
    println!("\n=== _Validation STREAMS ===");
    let validation_name = "\u{4840}\u{3FFF}\u{43E4}\u{41EC}\u{4824}";
    for (label, streams) in &[("OUR", &our_streams), ("SYS", &sys_streams)] {
        for (name, data) in streams.iter() {
            if name == validation_name {
                println!("{} _Validation: {} bytes", label, data.len());
                dump_hex(data, 128);
            }
        }
    }

    // Summary Information
    println!("\n=== SummaryInformation STREAMS ===");
    for (label, streams) in &[("OUR", &our_streams), ("SYS", &sys_streams)] {
        for (name, data) in streams.iter() {
            if name.contains("SummaryInformation") {
                println!("{} SummaryInformation: {} bytes, first 32:", label, data.len());
                dump_hex(data, 32);
            }
        }
    }
}

fn read_streams(path: &str) -> Vec<(String, Vec<u8>)> {
    let file = File::open(path).unwrap();
    let mut comp = cfb::CompoundFile::open(file).unwrap();

    // Collect paths first to avoid borrow issues
    let entries: Vec<(PathBuf, bool)> = comp.walk()
        .map(|e| (e.path().to_path_buf(), e.is_stream()))
        .collect();

    let mut streams = Vec::new();
    for (p, is_stream) in entries {
        if !is_stream { continue; }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let mut stream = comp.open_stream(&p).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        streams.push((name, data));
    }
    streams
}

fn dump_sp(data: &[u8], max_entries: usize) {
    if data.len() < 4 { println!("  Too short!"); return; }
    let cp_raw = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let long_refs = (cp_raw & 0x80000000) != 0;
    let cp = cp_raw & 0x7FFFFFFF;
    println!("  Header: codepage={}, long_refs={}", cp, long_refs);

    let mut off = 4;
    let mut id = 1u32;
    let mut data_off = 0u32;
    let mut count = 0;
    while off + 4 <= data.len() && count < max_entries {
        let len = u16::from_le_bytes([data[off], data[off+1]]);
        let refcount = u16::from_le_bytes([data[off+2], data[off+3]]);
        println!("  [{}] len={:3} refcount={:3} data_off={}", id, len, refcount, data_off);
        data_off += len as u32;
        off += 4;
        id += 1;
        count += 1;
    }
    let total = (data.len() - 4) / 4;
    if total > max_entries {
        println!("  ... ({} total entries)", total);
    }
    println!("  Total data bytes: {}", data_off);
}

fn dump_hex(data: &[u8], count: usize) {
    let n = data.len().min(count);
    for (i, chunk) in data[..n].chunks(16).enumerate() {
        print!("  {:04x}: ", i * 16);
        for b in chunk { print!("{:02x} ", b); }
        print!(" |");
        for b in chunk {
            if *b >= 0x20 && *b < 0x7f { print!("{}", *b as char); } else { print!("."); }
        }
        println!("|");
    }
}
