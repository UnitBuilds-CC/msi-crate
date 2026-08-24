/// Test: repackage our MSI using the cfb crate.
/// If the cfb-repackaged MSI works with msiexec, the issue is in our custom OLE writer.
/// If it still fails, the issue is in the MSI data.
use std::io::Cursor;

fn main() {
    let msi_path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\definitive_test.msi";
    let out_path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\cfb_repack.msi";
    
    // Read our MSI
    let msi_data = std::fs::read(msi_path).unwrap();
    eprintln!("Original MSI: {} bytes", msi_data.len());
    
    // Parse the OLE structure to extract streams
    let streams = extract_ole_streams(&msi_data);
    eprintln!("Extracted {} streams:", streams.len());
    for (name, data) in &streams {
        eprintln!("  {} ({} bytes)", name, data.len());
    }
    
    // Build a new OLE file using the cfb crate
    let mut comp = cfb::CompoundFile::builder(cfb::Version::V3);
    for (name, data) in &streams {
        if name.starts_with('\u{0005}') {
            // SummaryInformation - special handling
            continue; // Skip for now, we'll add it separately
        }
        comp.add_stream(name, data.as_slice());
    }
    
    let mut cursor = Cursor::new(Vec::new());
    let mut built = comp.build(&mut cursor).unwrap();
    
    // Add SummaryInfo stream
    for (name, data) in &streams {
        if name.starts_with('\u{0005}') {
            built.create_stream(name).unwrap().write_all(data).unwrap();
        }
    }
    built.flush().unwrap();
    
    let cfb_data = cursor.into_inner();
    std::fs::write(out_path, &cfb_data).unwrap();
    eprintln!("\nCFB-repackaged MSI: {} bytes", cfb_data.len());
    eprintln!("Written to: {}", out_path);
}

fn extract_ole_streams(data: &[u8]) -> Vec<(String, Vec<u8>)> {
    // Simple OLE V3 parser to extract streams
    let mut streams = Vec::new();
    
    // Check OLE header
    if data.len() < 512 || &data[0..4] != b"\xD0\xCF\x11\xE0" {
        eprintln!("Not a valid OLE file");
        return streams;
    }
    
    let sector_size = 512u32; // V3
    let mini_sector_size = 64u32;
    
    // Read header fields
    let total_sectors = u32::from_le_bytes([data[32], data[33], data[34], data[35]]);
    let fat_start = u32::from_le_bytes([data[44], data[45], data[46], data[47]]);
    let minifat_start = u32::from_le_bytes([data[60], data[61], data[62], data[63]]);
    let difat_start = u32::from_le_bytes([data[68], data[69], data[70], data[71]]);
    let num_difat = u32::from_le_bytes([data[68+4], data[68+5], data[68+6], data[68+7]]);
    
    // Read DIFAT entries from header (109 entries at offset 76)
    let mut fat_sectors = Vec::new();
    for i in 0..109 {
        let off = 76 + i * 4;
        let sect = u32::from_le_bytes([data[off], data[off+1], data[off+2], data[off+3]]);
        if sect == 0xFFFFFFFE || sect == 0xFFFFFFFF { break; }
        fat_sectors.push(sect);
    }
    
    // Build FAT
    let mut fat = Vec::new();
    for &sect in &fat_sectors {
        let off = (sect + 1) as usize * sector_size as usize;
        for j in 0..(sector_size / 4) {
            let val = u32::from_le_bytes([
                data[off + j as usize * 4],
                data[off + j as usize * 4 + 1],
                data[off + j as usize * 4 + 2],
                data[off + j as usize * 4 + 3],
            ]);
            fat.push(val);
        }
    }
    
    // Read directory tree (starts at sector specified in header offset 48)
    let dir_start = u32::from_le_bytes([data[48], data[49], data[50], data[51]]);
    
    // Read all directory entries
    let mut dir_entries = Vec::new();
    let mut current = dir_start;
    while current != 0xFFFFFFFF {
        let off = (current + 1) as usize * sector_size as usize;
        for i in 0..(sector_size / 128) {
            let entry_off = off + i as usize * 128;
            if entry_off + 128 > data.len() { break; }
            
            let name_len = u16::from_le_bytes([data[entry_off + 64], data[entry_off + 65]]) as usize;
            if name_len == 0 { continue; }
            
            // Read UTF-16LE name
            let mut name = String::new();
            for j in 0..(name_len / 2 - 1) {
                let ch = u16::from_le_bytes([data[entry_off + j*2], data[entry_off + j*2 + 1]]);
                name.push(char::from_u32(ch as u32).unwrap_or('?'));
            }
            
            let entry_type = data[entry_off + 66];
            let start_sect = u32::from_le_bytes([
                data[entry_off + 116], data[entry_off + 117],
                data[entry_off + 118], data[entry_off + 119],
            ]);
            let size = u32::from_le_bytes([
                data[entry_off + 120], data[entry_off + 121],
                data[entry_off + 122], data[entry_off + 123],
            ]);
            
            // Read child directory entry
            let child_id = u32::from_le_bytes([
                data[entry_off + 68], data[entry_off + 69],
                data[entry_off + 70], data[entry_off + 71],
            ]);
            
            dir_entries.push((name, entry_type, start_sect, size, child_id));
        }
        
        // Follow the directory chain
        if (current as usize) < fat.len() {
            current = fat[current as usize];
        } else {
            break;
        }
    }
    
    // Read mini-stream data
    let minifat_data = if minifat_start != 0xFFFFFFFF {
        Some(read_chain(data, &fat, minifat_start, sector_size))
    } else {
        None
    };
    
    // The root entry's start sector points to the mini-stream container
    let root_entry = dir_entries.iter().find(|(name, _, _, _, _)| name == "Root Entry");
    let mini_stream_container_start = root_entry.map(|(_, _, start, _, _)| *start).unwrap_or(0xFFFFFFFF);
    
    // Extract streams from directory entries
    for (name, entry_type, start_sect, size, _) in &dir_entries {
        if *entry_type != 2 { continue; } // Only streams (type 2)
        if name == "Root Entry" { continue; }
        
        let stream_data = if *size < 4096 && mini_stream_container_start != 0xFFFFFFFF {
            // Mini-stream
            if let Some(ref ms_data) = minifat_data {
                // Build mini-FAT
                let mut mfat = Vec::new();
                let mut current = minifat_start;
                while current != 0xFFFFFFFE && current != 0xFFFFFFFF {
                    let off = (current + 1) as usize * sector_size as usize;
                    for j in 0..(sector_size / 4) {
                        let val = u32::from_le_bytes([
                            data[off + j as usize * 4],
                            data[off + j as usize * 4 + 1],
                            data[off + j as usize * 4 + 2],
                            data[off + j as usize * 4 + 3],
                        ]);
                        mfat.push(val);
                    }
                    if (current as usize) < fat.len() {
                        current = fat[current as usize];
                    } else {
                        break;
                    }
                }
                
                // Read from mini-stream container
                let container_off = (mini_stream_container_start + 1) as usize * sector_size as usize;
                let mut stream = Vec::new();
                let mut current_sect = *start_sect;
                while stream.len() < *size as usize 
                    && current_sect != 0xFFFFFFFE && current_sect != 0xFFFFFFFF {
                    let off = container_off + current_sect as usize * mini_sector_size as usize;
                    let remaining = (*size as usize).saturating_sub(stream.len());
                    let to_read = remaining.min(mini_sector_size as usize);
                    if off + to_read <= data.len() {
                        stream.extend_from_slice(&data[off..off + to_read]);
                    }
                    if (current_sect as usize) < mfat.len() {
                        current_sect = mfat[current_sect as usize];
                    } else {
                        break;
                    }
                }
                stream.truncate(*size as usize);
                stream
            } else {
                Vec::new()
            }
        } else {
            // Large stream (regular sectors)
            read_chain(data, &fat, *start_sect, sector_size)
                .into_iter()
                .take(*size as usize)
                .collect()
        };
        
        streams.push((name.clone(), stream_data));
    }
    
    streams
}

fn read_chain(data: &[u8], fat: &[u32], start: u32, sector_size: u32) -> Vec<u8> {
    let mut result = Vec::new();
    let mut current = start;
    while current != 0xFFFFFFFE && current != 0xFFFFFFFF {
        let off = (current + 1) as usize * sector_size as usize;
        if off + sector_size as usize <= data.len() {
            result.extend_from_slice(&data[off..off + sector_size as usize]);
        }
        if (current as usize) < fat.len() {
            current = fat[current as usize];
        } else {
            break;
        }
    }
    result
}
