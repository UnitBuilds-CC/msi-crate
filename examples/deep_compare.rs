/// Deep comparison of our MSI vs a system MSI
/// Uses cfb to read both files, compares stream content
use cfb::CompoundFile;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

fn main() {
    let our_path = "target/test_velocity.msi";
    let sys_path = "C:\\Windows\\Installer\\10d16cbb.msi";

    println!("=== OUR MSI: {} ===", our_path);
    let our_data = dump_all_streams(our_path);

    println!("\n=== SYSTEM MSI: {} ===", sys_path);
    let sys_data = dump_all_streams(sys_path);

    // Compare string pools
    println!("\n=== STRING POOL COMPARISON ===");
    if let (Some(our_sp), Some(sys_sp)) = (find_string_pool(&our_data), find_string_pool(&sys_data)) {
        compare_string_pools(&our_sp, &sys_sp);
    } else {
        println!("Could not find string pool in one or both files!");
        println!("  Our has SP: {}", find_string_pool(&our_data).is_some());
        println!("  Sys has SP: {}", find_string_pool(&sys_data).is_some());
    }

    // Compare _Tables streams
    println!("\n=== _Tables STREAM COMPARISON ===");
    if let (Some(our_t), Some(sys_t)) = (find_stream_decoded(&our_data, "_Tables"), find_stream_decoded(&sys_data, "_Tables")) {
        println!("Our _Tables: {} bytes", our_t.data.len());
        println!("Sys _Tables: {} bytes", sys_t.data.len());
    }

    // Compare _Columns streams
    println!("\n=== _Columns STREAM COMPARISON ===");
    if let (Some(our_c), Some(sys_c)) = (find_stream_decoded(&our_data, "_Columns"), find_stream_decoded(&sys_data, "_Columns")) {
        println!("Our _Columns: {} bytes", our_c.data.len());
        println!("Sys _Columns: {} bytes", sys_c.data.len());
    }
}

struct StreamInfo {
    name_raw: String,     // raw name as stored in OLE
    decoded_name: String, // decoded human-readable name
    data: Vec<u8>,
    has_table_prefix: bool,
}

fn dump_all_streams(path: &str) -> Vec<StreamInfo> {
    let file = File::open(path).unwrap();
    let mut comp = CompoundFile::open(file).unwrap();

    let entries: Vec<(PathBuf, bool)> = comp
        .walk()
        .map(|e| (e.path().to_path_buf(), e.is_stream()))
        .collect();

    let mut streams = Vec::new();

    for (p, is_stream) in &entries {
        if !is_stream { continue; }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let mut stream = comp.open_stream(p).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();

        let cps: Vec<u16> = name.encode_utf16().collect();
        let has_prefix = !cps.is_empty() && cps[0] == 0x4840;
        let decoded = if has_prefix {
            decode_name(&cps)
        } else {
            // Check for SummaryInformation
            if name.contains("SummaryInformation") || name.contains('\u{0005}') {
                "\\x05SummaryInformation".to_string()
            } else {
                decode_name_no_prefix(&cps)
            }
        };

        println!("  Stream: decoded='{}' size={}", decoded, data.len());

        // If this is a string pool, parse it
        if decoded == "_StringPool" {
            parse_string_pool_header(&data);
        }
        if decoded == "_StringData" {
            println!("    First 64 bytes:");
            for (i, chunk) in data[..data.len().min(64)].chunks(16).enumerate() {
                print!("    {:04x}: ", i * 16);
                for b in chunk { print!("{:02x} ", b); }
                print!(" |");
                for b in chunk {
                    if *b >= 0x20 && *b < 0x7f { print!("{}", *b as char); } else { print!("."); }
                }
                println!("|");
            }
        }

        streams.push(StreamInfo {
            name_raw: name,
            decoded_name: decoded,
            data,
            has_table_prefix: has_prefix,
        });
    }

    streams
}

fn find_string_pool(streams: &[StreamInfo]) -> Option<&StreamInfo> {
    streams.iter().find(|s| s.decoded_name == "_StringPool")
}

fn find_stream_decoded<'a>(streams: &'a [StreamInfo], name: &str) -> Option<&'a StreamInfo> {
    streams.iter().find(|s| s.decoded_name == name)
}

fn parse_string_pool_header(data: &[u8]) {
    if data.len() < 4 { println!("    _StringPool too short!"); return; }
    let codepage_raw = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let long_refs = (codepage_raw & 0x80000000) != 0;
    let cp = codepage_raw & 0x7FFFFFFF;
    println!("    Codepage: {}, long_refs: {}", cp, long_refs);

    let ref_size = if long_refs { 4 } else { 2 };
    let mut offset = 4;
    let mut string_id = 1u32;
    let mut data_offset = 0u32;
    let mut count = 0;
    while offset + 4 <= data.len() && count < 20 {
        let len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        let refcount = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);
        println!("    String {}: len={} refcount={} data_offset={}", string_id, len, refcount, data_offset);
        data_offset += len as u32;
        offset += 4;
        string_id += 1;
        count += 1;
    }
    if offset + 4 <= data.len() {
        println!("    ... ({} total entries)", data.len() / 4);
    }
    println!("    Total strings: {}, total data bytes: {}", string_id - 1, data_offset);
}

fn compare_string_pools(our: &StreamInfo, sys: &StreamInfo) {
    println!("Our _StringPool: {} bytes", our.data.len());
    println!("Sys _StringPool: {} bytes", sys.data.len());

    if our.data.len() < 4 || sys.data.len() < 4 { return; }

    let our_cp_raw = u32::from_le_bytes([our.data[0], our.data[1], our.data[2], our.data[3]]);
    let sys_cp_raw = u32::from_le_bytes([sys.data[0], sys.data[1], sys.data[2], sys.data[3]]);

    println!("Our codepage_raw: 0x{:08X} (cp={}, long={})", our_cp_raw, our_cp_raw & 0x7FFFFFFF, (our_cp_raw & 0x80000000) != 0);
    println!("Sys codepage_raw: 0x{:08X} (cp={}, long={})", sys_cp_raw, sys_cp_raw & 0x7FFFFFFF, (sys_cp_raw & 0x80000000) != 0);

    // Parse entries from both
    let our_entries = parse_sp_entries(&our.data);
    let sys_entries = parse_sp_entries(&sys.data);

    println!("Our entries: {}", our_entries.len());
    println!("Sys entries: {}", sys_entries.len());

    // Show first 10 entries from each
    println!("\nOur first entries:");
    for (i, e) in our_entries.iter().take(10).enumerate() {
        println!("  {}: len={} refcount={}", i + 1, e.len, e.refcount);
    }
    println!("\nSys first entries:");
    for (i, e) in sys_entries.iter().take(10).enumerate() {
        println!("  {}: len={} refcount={}", i + 1, e.len, e.refcount);
    }

    // Check if sys uses null terminators
    if !sys_entries.is_empty() {
        println!("\nSys string data analysis (checking for null terminators):");
        let sys_data = &sys.data;
        // Find the string data stream
        // We need to look at _StringData separately
    }
}

struct SpEntry {
    len: u16,
    refcount: u16,
}

fn parse_sp_entries(data: &[u8]) -> Vec<SpEntry> {
    if data.len() < 4 { return vec![]; }
    let mut entries = Vec::new();
    let mut offset = 4;
    while offset + 4 <= data.len() {
        let len = u16::from_le_bytes([data[offset], data[offset + 1]]);
        let refcount = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);
        entries.push(SpEntry { len, refcount });
        offset += 4;
    }
    entries
}

fn decode_name(cps: &[u16]) -> String {
    let mut result = String::new();
    let mut i = 0;
    if !cps.is_empty() && cps[0] == 0x4840 { i = 1; }
    while i < cps.len() {
        let cp = cps[i] as u32;
        if cp >= 0x3800 && cp <= 0x3FFF {
            let raw = cp - 0x3800;
            let v1 = (raw & 63) as u8;
            let v2 = (raw >> 6) as u8;
            if let (Some(c1), Some(c2)) = (from_b64_rev(v1), from_b64_rev(v2)) {
                result.push(c1); result.push(c2); i += 1; continue;
            }
        }
        if cp >= 0x4800 && cp <= 0x483F {
            if let Some(c) = from_b64_rev((cp - 0x4800) as u8) { result.push(c); i += 1; continue; }
        }
        result.push(char::from_u32(cp).unwrap_or('?'));
        i += 1;
    }
    result
}

fn decode_name_no_prefix(cps: &[u16]) -> String {
    let mut result = String::new();
    let mut i = 0;
    while i < cps.len() {
        let cp = cps[i] as u32;
        if cp >= 0x3800 && cp <= 0x3FFF {
            let raw = cp - 0x3800;
            let v1 = (raw & 63) as u8;
            let v2 = (raw >> 6) as u8;
            if let (Some(c1), Some(c2)) = (from_b64_rev(v1), from_b64_rev(v2)) {
                result.push(c1); result.push(c2); i += 1; continue;
            }
        }
        if cp >= 0x4800 && cp <= 0x483F {
            if let Some(c) = from_b64_rev((cp - 0x4800) as u8) { result.push(c); i += 1; continue; }
        }
        result.push(char::from_u32(cp).unwrap_or('?'));
        i += 1;
    }
    result
}

fn from_b64_rev(val: u8) -> Option<char> {
    match val {
        0..=9 => Some((b'0' + val) as char),
        10..=35 => Some((b'A' + val - 10) as char),
        36..=61 => Some((b'a' + val - 36) as char),
        62 => Some('.'),
        63 => Some('_'),
        _ => None,
    }
}
