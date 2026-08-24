/// Dump all OLE directory entries (stream names) from our MSI.
fn main() {
    let msi_path = r"c:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\installer.msi";
    let data = std::fs::read(msi_path).expect("read MSI");
    
    println!("MSI size: {} bytes", data.len());
    
    // Parse OLE header
    if &data[0..8] != b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1" {
        println!("NOT an OLE file!");
        return;
    }
    
    let sector_size = 512u32; // V3
    let mini_sector_size = 64u32;
    
    // Read header fields
    let num_fat_sectors = u32::from_le_bytes([data[44], data[45], data[46], data[47]]);
    let dir_start = u32::from_le_bytes([data[48], data[49], data[50], data[51]]);
    let mini_stream_cutoff = u32::from_le_bytes([data[56], data[57], data[58], data[59]]);
    let mini_fat_start = u32::from_le_bytes([data[60], data[61], data[62], data[63]]);
    
    println!("FAT sectors: {}", num_fat_sectors);
    println!("Dir start sector: {}", dir_start);
    println!("Mini stream cutoff: {}", mini_stream_cutoff);
    println!("Mini FAT start sector: {}", mini_fat_start);
    
    // Read DIFAT (first 109 entries in header)
    let mut fat_sectors = Vec::new();
    for i in 0..109 {
        let offset = 76 + i * 4;
        let sector = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
        if sector != 0xFFFFFFFF && sector != 0xFFFFFFFE {
            fat_sectors.push(sector);
        }
    }
    println!("FAT sector list: {:?}", fat_sectors);
    
    // Build full FAT
    let mut fat = Vec::new();
    for &sector in &fat_sectors {
        let offset = (sector + 1) as usize * sector_size as usize;
        for j in 0..(sector_size / 4) {
            let entry_offset = offset + j as usize * 4;
            if entry_offset + 4 <= data.len() {
                let entry = u32::from_le_bytes([data[entry_offset], data[entry_offset+1], data[entry_offset+2], data[entry_offset+3]]);
                fat.push(entry);
            }
        }
    }
    
    // Read directory chain starting from dir_start
    let mut dir_data = Vec::new();
    let mut sector = dir_start;
    let max_iters = 1000;
    let mut iters = 0;
    while sector != 0xFFFFFFFE && sector != 0xFFFFFFFF && iters < max_iters {
        let offset = (sector + 1) as usize * sector_size as usize;
        if offset + sector_size as usize <= data.len() {
            dir_data.extend_from_slice(&data[offset..offset + sector_size as usize]);
        }
        if (sector as usize) < fat.len() {
            sector = fat[sector as usize];
        } else {
            break;
        }
        iters += 1;
    }
    
    // Parse directory entries (128 bytes each)
    let num_entries = dir_data.len() / 128;
    println!("\n=== DIRECTORY ENTRIES ({} entries) ===", num_entries);
    
    for i in 0..num_entries {
        let entry_offset = i * 128;
        if entry_offset + 128 > dir_data.len() { break; }
        
        let entry = &dir_data[entry_offset..entry_offset + 128];
        
        // Name (UTF-16LE, up to 64 bytes = 32 chars)
        let name_len = u16::from_le_bytes([entry[64], entry[65]]) as usize;
        if name_len <= 2 { continue; } // Empty entry
        
        let mut name = String::new();
        for j in (0..name_len-2).step_by(2) {
            if j + 1 < name_len {
                let ch = u16::from_le_bytes([entry[j], entry[j+1]]);
                if let Some(c) = char::from_u32(ch as u32) {
                    name.push(c);
                }
            }
        }
        
        let entry_type = entry[66];
        let start_sector = u32::from_le_bytes([entry[116], entry[117], entry[118], entry[119]]);
        let size = u32::from_le_bytes([entry[120], entry[121], entry[122], entry[123]]);
        
        let type_name = match entry_type {
            0 => "Empty",
            1 => "Storage",
            2 => "Stream",
            5 => "Root",
            _ => "Unknown",
        };
        
        // Show name as both text and hex (to see encoded names)
        let name_hex: Vec<String> = name.chars().map(|c| format!("U+{:04X}", c as u32)).collect();
        
        println!("  [{}] Type={}, Name=\"{}\" ({}), Start={}, Size={}",
            i, type_name, name, name_hex.join(" "), start_sector, size);
    }
}
