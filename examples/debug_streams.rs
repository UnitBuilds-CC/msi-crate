/// Debug: list all streams in the MSI
use std::io::{Cursor, Read};

fn main() {
    let path = "C:\\temp\\complete_test.msi";
    let data = std::fs::read(path).unwrap();
    println!("MSI: {} bytes, V{}", data.len(), data[26]);

    let mut comp = cfb::CompoundFile::open(Cursor::new(&data)).unwrap();

    // Collect stream names first
    let stream_names: Vec<String> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.name().to_string())
        .collect();

    println!("\nAll streams ({}):", stream_names.len());
    for name in &stream_names {
        let mut s = comp.open_stream(name).unwrap();
        let mut buf = Vec::new();
        s.read_to_end(&mut buf).unwrap();

        let hex: String = name.encode_utf16()
            .map(|c| format!("{:04X}", c))
            .collect::<Vec<_>>().join(" ");

        let decoded = if name.starts_with('\u{4840}') {
            decode_stream_name(name)
        } else if name.starts_with('\u{0005}') {
            "\\x05SummaryInformation".to_string()
        } else {
            format!("'{}'", name)
        };

        println!("  [{}] {} ({} bytes)", hex, decoded, buf.len());
    }
}

fn decode_stream_name(encoded: &str) -> String {
    let mut chars = encoded.chars().peekable();
    let first = chars.next().unwrap();
    if first != '\u{4840}' { return encoded.to_string(); }
    let remaining: Vec<char> = chars.collect();
    let mut result = String::new();
    let mut i = 0;
    while i < remaining.len() {
        let ch = remaining[i];
        let val = (ch as u32).wrapping_sub(0x3800);
        if i + 1 < remaining.len() {
            let d1 = val % 64;
            let d2 = val / 64;
            if let Some(c1) = to_ascii(d1) { result.push(c1); }
            if let Some(c2) = to_ascii(d2) { result.push(c2); }
            i += 2;
        } else {
            let val2 = (ch as u32).wrapping_sub(0x4800);
            if let Some(c1) = to_ascii(val2) { result.push(c1); }
            i += 1;
        }
    }
    result
}

fn to_ascii(v: u32) -> Option<char> {
    if v < 10 { char::from_u32(v + b'0' as u32) }
    else if v < 36 { char::from_u32(v - 10 + b'A' as u32) }
    else if v < 62 { char::from_u32(v - 36 + b'a' as u32) }
    else if v == 62 { Some('.') }
    else if v == 63 { Some('_') }
    else { None }
}
