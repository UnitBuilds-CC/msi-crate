use std::io::{Cursor, Read};

fn decode_stream_name(encoded: &str) -> Option<String> {
    let chars: Vec<char> = encoded.chars().collect();
    if chars.is_empty() || chars[0] != '\u{4840}' { return None; }
    let mut decoded = String::new();
    let mut idx = 1;
    while idx < chars.len() {
        let c = chars[idx] as u32;
        if c >= 0x3800 && c < 0x4800 {
            let val = (c - 0x3800) as u16;
            let v1 = (val & 0x3F) as u8;
            let v2 = (val >> 6) as u8;
            decoded.push(from_b64(v1));
            decoded.push(from_b64(v2));
            idx += 1;
        } else if c >= 0x4800 && c < 0x4840 {
            decoded.push(from_b64((c - 0x4800) as u8));
            idx += 1;
        } else {
            decoded.push(chars[idx]);
            idx += 1;
        }
    }
    Some(decoded)
}

fn from_b64(val: u8) -> char {
    match val {
        0..=9 => (b'0' + val) as char,
        10..=35 => (b'A' + val - 10) as char,
        36..=61 => (b'a' + val - 36) as char,
        62 => '.',
        63 => '_',
        _ => '?',
    }
}

fn main() {
    let data = std::fs::read("velocity_install_test.msi").unwrap();
    let cursor = Cursor::new(&data);
    let mut cfb = cfb::CompoundFile::open(cursor).unwrap();

    let stream_names: Vec<String> = cfb.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.name().to_string())
        .collect();

    for name in &stream_names {
        if let Some(decoded) = decode_stream_name(name) {
            if decoded == "Directory" {
                println!("Raw OLE name: {:?}", name);
                println!("Raw name chars:");
                for (i, c) in name.chars().enumerate() {
                    println!("  [{}] = U+{:04X} ({:?})", i, c as u32, c);
                }
                let mut stream_data = Vec::new();
                let mut stream = cfb.open_stream(name).unwrap();
                stream.read_to_end(&mut stream_data).unwrap();
                println!("Directory stream: {} bytes", stream_data.len());
                println!("Hex: {:02X?}", stream_data);
                println!("Expected: 12 bytes for 2 rows × 3 columns");
            }
        }
    }
}
