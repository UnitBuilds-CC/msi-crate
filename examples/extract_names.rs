/// Extract exact codepoints for _StringPool and _StringData from a system MSI
use std::fs::File;
use std::io::Read;

fn main() {
    let path = "C:\\Windows\\Installer\\10d16cbb.msi";
    let mut file = File::open(path).unwrap();
    let mut header = vec![0u8; 4096];
    file.read_exact(&mut header).unwrap();

    let sector_size = 4096usize;
    let dir_first_sect = u32::from_le_bytes([header[48], header[49], header[50], header[51]]) as usize;

    // Read FAT
    let fat_sect = u32::from_le_bytes([header[76], header[77], header[78], header[79]]) as usize;
    let mut fat = vec![0u8; sector_size];
    seek_read(&mut file, &mut fat, 4096 + fat_sect * sector_size);

    // Read all directory sectors
    let mut dir_data = Vec::new();
    let mut sect = dir_first_sect;
    while sect < 0xFFFFFFFE {
        let mut buf = vec![0u8; sector_size];
        seek_read(&mut file, &mut buf, 4096 + sect * sector_size);
        dir_data.extend_from_slice(&buf);
        let idx = sect * 4;
        sect = u32::from_le_bytes([fat[idx], fat[idx+1], fat[idx+2], fat[idx+3]]) as usize;
    }

    let num_entries = dir_data.len() / 128;
    for i in 0..num_entries {
        let off = i * 128;
        let name_len_null = u16::from_le_bytes([dir_data[off+64], dir_data[off+65]]);
        if name_len_null == 0 { continue; }
        let name_len = (name_len_null / 2 - 1) as usize;

        let mut cps: Vec<u16> = Vec::new();
        for j in 0..name_len {
            cps.push(u16::from_le_bytes([dir_data[off+j*2], dir_data[off+j*2+1]]));
        }

        // Show ALL entries with TABLE_PREFIX
        if cps.len() >= 2 && cps[0] == 0x4840 {
            let obj_type = dir_data[off + 66];
            if obj_type != 2 { continue; } // only streams

            // Decode the name
            let decoded = decode_name(&cps);

            // Check if this is _StringPool or _StringData
            if decoded.contains("Pool") || decoded.contains("Data") || decoded.starts_with("_S") {
                print!("Entry {}: decoded='{}' codepoints=", i, decoded);
                for cp in &cps {
                    print!("U+{:04X} ", cp);
                }
                println!();
                print!("  Rust: \"");
                for cp in &cps {
                    print!("\\u{{{:04X}}}", cp);
                }
                println!("\"");
            }
        }
    }
}

fn seek_read(file: &mut File, buf: &mut [u8], offset: usize) {
    use std::io::Seek;
    file.seek(std::io::SeekFrom::Start(offset as u64)).unwrap();
    file.read_exact(buf).unwrap();
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
            if let (Some(c1), Some(c2)) = (from_b64(v1), from_b64(v2)) {
                result.push(c1); result.push(c2); i += 1; continue;
            }
        }
        if cp >= 0x4800 && cp <= 0x483F {
            if let Some(c) = from_b64((cp - 0x4800) as u8) { result.push(c); i += 1; continue; }
        }
        result.push(char::from_u32(cp).unwrap_or('?'));
        i += 1;
    }
    result
}

fn from_b64(val: u8) -> Option<char> {
    match val {
        0..=9 => Some((b'0' + val) as char),
        10..=35 => Some((b'A' + val - 10) as char),
        36..=61 => Some((b'a' + val - 36) as char),
        62 => Some('.'), 63 => Some('_'), _ => None,
    }
}
