/// Decode MSI stream names from base-64 Unicode encoding
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = if args.len() > 1 { &args[1] } else { "target/test_velocity.msi" };

    let mut file = std::fs::File::open(path).unwrap();
    let mut header = vec![0u8; 4096];
    std::io::Read::read_exact(&mut file, &mut header).unwrap();

    let sector_shift = u16::from_le_bytes([header[30], header[31]]) as usize;
    let sector_size = 1usize << sector_shift;
    let dir_first_sect = u32::from_le_bytes([header[48], header[49], header[50], header[51]]) as usize;

    // Read FAT to follow directory chain
    let mut fat = vec![0u8; sector_size];
    let fat_sect = u32::from_le_bytes([header[76], header[77], header[78], header[79]]) as usize;
    let fat_off = 4096 + fat_sect * sector_size;
    file.seek_read(&mut fat, fat_off as u64).unwrap();

    // Follow directory sector chain
    let mut dir_data = Vec::new();
    let mut sect = dir_first_sect;
    while sect < 0xFFFFFFFE {
        let off = 4096 + sect * sector_size;
        let mut buf = vec![0u8; sector_size];
        file.seek_read(&mut buf, off as u64).unwrap();
        dir_data.extend_from_slice(&buf);
        let fat_idx = sect * 4;
        sect = u32::from_le_bytes([
            fat[fat_idx], fat[fat_idx + 1], fat[fat_idx + 2], fat[fat_idx + 3],
        ]) as usize;
    }

    let num_entries = dir_data.len() / 128;
    for i in 0..num_entries {
        let off = i * 128;
        let name_len_with_null = u16::from_le_bytes([dir_data[off + 64], dir_data[off + 65]]);
        if name_len_with_null == 0 { continue; }
        let name_len = (name_len_with_null / 2 - 1) as usize;

        let mut cps: Vec<u16> = Vec::new();
        for j in 0..name_len {
            let lo = dir_data[off + j * 2];
            let hi = dir_data[off + j * 2 + 1];
            cps.push(u16::from_le_bytes([lo, hi]));
        }

        let obj_type = dir_data[off + 66];
        let type_name = match obj_type {
            0 => "empty", 1 => "storage", 2 => "stream", 5 => "root", _ => "?",
        };
        if type_name == "root" || type_name == "empty" { continue; }

        let decoded = decode_name(&cps);
        let has_prefix = !cps.is_empty() && cps[0] == 0x4840;

        println!("Entry {:2}: [{}] prefix={} decoded='{}'",
            i, type_name, has_prefix, decoded);
    }
}

fn decode_name(cps: &[u16]) -> String {
    let mut result = String::new();
    let mut i = 0;

    // Skip TABLE_PREFIX if present
    if !cps.is_empty() && cps[0] == 0x4840 {
        i = 1;
    }

    while i < cps.len() {
        let cp = cps[i] as u32;
        if cp >= 0x3800 && cp <= 0x3FFF {
            // Pair encoding: always decodes to 2 characters
            let raw = cp - 0x3800;
            let v1 = (raw & 63) as u8;
            let v2 = (raw >> 6) as u8;
            if let (Some(c1), Some(c2)) = (from_b64_rev(v1), from_b64_rev(v2)) {
                result.push(c1);
                result.push(c2);
                i += 1;
                continue;
            }
        }
        // Single character encoding or literal
        if cp >= 0x4800 && cp <= 0x483F {
            let val = (cp - 0x4800) as u8;
            if let Some(c) = from_b64_rev(val) {
                result.push(c);
                i += 1;
                continue;
            }
        }
        // Literal character (like U+0005 for SummaryInformation)
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

trait SeekRead {
    fn seek_read(&mut self, buf: &mut [u8], offset: u64) -> std::io::Result<()>;
}
impl SeekRead for std::fs::File {
    fn seek_read(&mut self, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
        use std::io::{Read, Seek};
        self.seek(std::io::SeekFrom::Start(offset))?;
        self.read_exact(buf)
    }
}
