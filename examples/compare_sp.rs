/// Compare _StringPool stream between our MSI and a system MSI
use cfb::CompoundFile;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let our_path = if args.len() > 1 { &args[1] } else { "target/test_velocity.msi" };
    let sys_path = if args.len() > 2 { &args[2] } else { "C:\\Windows\\Installer\\10d16cbb.msi" };

    println!("=== OUR MSI: {} ===", our_path);
    dump_string_pool(our_path);

    println!("\n=== SYSTEM MSI: {} ===", sys_path);
    dump_string_pool(sys_path);
}

fn dump_string_pool(path: &str) {
    let file = File::open(path).unwrap();
    let mut comp = CompoundFile::open(file).unwrap();

    let entries: Vec<(PathBuf, bool)> = comp
        .walk()
        .map(|e| (e.path().to_path_buf(), e.is_stream()))
        .collect();

    for (p, is_stream) in &entries {
        if !is_stream { continue; }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let mut stream = comp.open_stream(p).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();

        // Check if this is _StringPool by looking at the codepoints
        let cps: Vec<u16> = name.encode_utf16().collect();
        let is_string_pool = has_string_pool_pattern(&cps);
        let is_string_data = has_string_data_pattern(&cps);

        if is_string_pool {
            println!("_StringPool stream ({} bytes):", data.len());
            if data.len() < 4 { println!("  Too short!"); continue; }
            let codepage = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            let long_refs = (codepage & 0x80000000) != 0;
            let actual_cp = codepage & 0x7FFFFFFF;
            println!("  Codepage: {}, long_refs: {}", actual_cp, long_refs);

            let mut offset = 4;
            let mut string_id = 1u32;
            let mut data_offset = 0u32;
            while offset + 4 <= data.len() {
                let len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
                let refcount = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);
                println!("  String {}: len={} refcount={} data_offset={}", string_id, len, refcount, data_offset);
                data_offset += len as u32;
                offset += 4;
                string_id += 1;
            }
            println!("  Total strings: {}, total data bytes: {}", string_id - 1, data_offset);
        }

        if is_string_data {
            println!("_StringData stream ({} bytes):", data.len());
            // Show first 128 bytes
            for (i, chunk) in data[..data.len().min(128)].chunks(32).enumerate() {
                print!("  {:04x}: ", i * 32);
                for b in chunk { print!("{:02x} ", b); }
                print!(" |");
                for b in chunk {
                    if *b >= 0x20 && *b < 0x7f { print!("{}", *b as char); } else { print!("."); }
                }
                println!("|");
            }
        }
    }
}

fn has_string_pool_pattern(cps: &[u16]) -> bool {
    // Look for the pattern: starts with U+4840 (TABLE_PREFIX), then contains U+3F3F U+4577 U+446C
    if cps.len() < 4 { return false; }
    cps[0] == 0x4840 && cps[1] == 0x3F3F && cps[2] == 0x4577 && cps[3] == 0x446C
        && cps.iter().any(|&c| c == 0x487F) // "gP" for Pool
}

fn has_string_data_pattern(cps: &[u16]) -> bool {
    if cps.len() < 4 { return false; }
    cps[0] == 0x4840 && cps[1] == 0x3F3F && cps[2] == 0x4577 && cps[3] == 0x446C
        && cps.iter().any(|&c| c == 0x4559) // "YP" for Data (different from Pool)
}
