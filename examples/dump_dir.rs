/// Dump the raw directory entries from an OLE file to see the actual UTF-16LE stream names.
use std::fs::File;
use std::io::Read;

const SECTOR_SIZE: usize = 4096;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = if args.len() > 1 { &args[1] } else { "target/test_velocity.msi" };

    let mut file = File::open(path).unwrap();
    let mut header = vec![0u8; SECTOR_SIZE];
    file.read_exact(&mut header).unwrap();

    // Read header fields
    let major = u16::from_le_bytes([header[26], header[27]]);
    let sector_shift = u16::from_le_bytes([header[30], header[31]]) as usize;
    let sector_size = 1usize << sector_shift;
    let dir_first_sect = u32::from_le_bytes([header[48], header[49], header[50], header[51]]) as usize;

    println!("OLE version: {}, sector size: {}, first dir sector: {}", major, sector_size, dir_first_sect);

    // Read first directory sector
    let dir_offset = SECTOR_SIZE + dir_first_sect * sector_size;
    let mut dir_data = vec![0u8; sector_size];
    file.seek_read(&mut dir_data, dir_offset as u64).unwrap();

    // Parse directory entries (128 bytes each)
    let num_entries = sector_size / 128;
    for i in 0..num_entries {
        let off = i * 128;
        let name_len_with_null = u16::from_le_bytes([dir_data[off + 64], dir_data[off + 65]]);
        if name_len_with_null == 0 {
            continue;
        }
        let name_len = (name_len_with_null / 2 - 1) as usize; // exclude null terminator

        // Read UTF-16LE name
        let mut name_chars = Vec::new();
        for j in 0..name_len {
            let lo = dir_data[off + j * 2];
            let hi = dir_data[off + j * 2 + 1];
            name_chars.push(u16::from_le_bytes([lo, hi]));
        }
        let name = String::from_utf16(&name_chars).unwrap_or_else(|_| "<invalid>".to_string());

        let obj_type = dir_data[off + 66];
        let color = dir_data[off + 67];
        let left = i32::from_le_bytes([dir_data[off + 68], dir_data[off + 69], dir_data[off + 70], dir_data[off + 71]]);
        let right = i32::from_le_bytes([dir_data[off + 72], dir_data[off + 73], dir_data[off + 74], dir_data[off + 75]]);
        let child = i32::from_le_bytes([dir_data[off + 76], dir_data[off + 77], dir_data[off + 78], dir_data[off + 79]]);

        let type_name = match obj_type {
            0 => "empty",
            1 => "storage",
            2 => "stream",
            5 => "root",
            _ => "unknown",
        };

        let cps: Vec<String> = name.chars().map(|c| format!("U+{:04X}", c as u32)).collect();

        println!("\nEntry {}: name='{}' ({} chars)", i, name, name_len);
        println!("  Codepoints: {}", cps.join(" "));
        println!("  Type: {} ({}), Color: {}", type_name, obj_type, color);
        println!("  Left: {}, Right: {}, Child: {}", left, right, child);

        // Show raw name bytes
        print!("  Raw name bytes: ");
        for j in 0..(name_len_with_null as usize).min(40) {
            print!("{:02x} ", dir_data[off + j]);
        }
        println!();
    }
}

// Helper trait for seeking reads
trait SeekRead {
    fn seek_read(&mut self, buf: &mut [u8], offset: u64) -> std::io::Result<()>;
}

impl SeekRead for File {
    fn seek_read(&mut self, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
        use std::io::Seek;
        self.seek(std::io::SeekFrom::Start(offset))?;
        self.read_exact(buf)?;
        Ok(())
    }
}
