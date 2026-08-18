use cfb::CompoundFile;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

fn inspect(path: &str) {
    println!("\n=== {} ===", path);
    let raw = std::fs::read(path).unwrap();
    
    // OLE header layout:
    // 0-7: magic, 8-23: CLSID, 24-25: minor ver, 26-27: major ver,
    // 28-29: BOM, 30-31: sector shift, 32-33: mini sector shift,
    // 34-39: reserved, 40-43: directory sectors (V4 only), 44-47: FAT sectors,
    // 48-51: first dir sector, 52-55: dir sectors count
    println!("OLE Header:");
    let minor_ver = u16::from_le_bytes([raw[24], raw[25]]);
    let major_ver = u16::from_le_bytes([raw[26], raw[27]]);
    let bom = u16::from_le_bytes([raw[28], raw[29]]);
    let shift = u16::from_le_bytes([raw[30], raw[31]]);
    let mini_shift = u16::from_le_bytes([raw[32], raw[33]]);
    let fat_sectors = u32::from_le_bytes([raw[44], raw[45], raw[46], raw[47]]);
    let dir_start = u32::from_le_bytes([raw[48], raw[49], raw[50], raw[51]]);
    let dir_sectors = u32::from_le_bytes([raw[52], raw[53], raw[54], raw[55]]);
    println!("  Version: {}.{}, BOM: 0x{:04X}, Sector shift: {} ({}B), Mini shift: {}", major_ver, minor_ver, bom, shift, 1u64 << shift, mini_shift);
    println!("  FAT sectors: {}, Dir start sector: {}, Dir sectors: {}", fat_sectors, dir_start, dir_sectors);
    
    let file = File::open(path).unwrap();
    let mut comp = CompoundFile::open(file).unwrap();
    
    // Collect entries first to avoid borrow issues
    let entries: Vec<(PathBuf, bool)> = comp.walk().map(|e| (e.path().to_path_buf(), e.is_stream())).collect();
    
    println!("\nStreams:");
    for (p, is_stream) in &entries {
        let name = match p.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => "(root)".to_string(),
        };
        if *is_stream {
            let size = comp.open_stream(p).map(|s| s.len()).unwrap_or(0);
            let codepoints: Vec<String> = name.chars().map(|c| format!("U+{:04X}", c as u32)).collect();
            println!("  [STREAM] '{}' ({} bytes) codepoints: {}", name, size, codepoints.join(" "));
        } else {
            println!("  [STORAGE] '{}'", name);
        }
    }
    
    // Read SummaryInformation
    let si_path = Path::new("\u{0005}SummaryInformation");
    if let Ok(mut stream) = comp.open_stream(si_path) {
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        println!("\n  SummaryInformation: {} bytes", data.len());
        if data.len() >= 48 {
            // Property Set header:
            // 0-1: BOM, 2-3: version, 4-7: OS, 8-23: CLSID, 24-27: reserved,
            // 28-43: FMTID (16 bytes), 44-47: section offset
            let bom = u16::from_le_bytes([data[0], data[1]]);
            let reserved = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
            let fmtid = &data[28..44];
            let sec_off = u32::from_le_bytes([data[44], data[45], data[46], data[47]]);
            println!("    BOM: 0x{:04X}, Reserved: {}, Section offset: {}", bom, reserved, sec_off);
            print!("    FMTID: ");
            for b in fmtid { print!("{:02x}", b); }
            println!();
            
            if (sec_off as usize + 8) <= data.len() {
                let s = sec_off as usize;
                let sec_size = u32::from_le_bytes([data[s], data[s+1], data[s+2], data[s+3]]);
                let nprops = u32::from_le_bytes([data[s+4], data[s+5], data[s+6], data[s+7]]);
                println!("    Section size: {}, Properties: {}", sec_size, nprops);
                
                for i in 0..nprops as usize {
                    let eo = s + 8 + i * 8;
                    if eo + 8 > data.len() { break; }
                    let pid = u32::from_le_bytes([data[eo], data[eo+1], data[eo+2], data[eo+3]]);
                    let poff = u32::from_le_bytes([data[eo+4], data[eo+5], data[eo+6], data[eo+7]]);
                    let vo = s + poff as usize;
                    if vo + 4 <= data.len() {
                        let vt = u32::from_le_bytes([data[vo], data[vo+1], data[vo+2], data[vo+3]]);
                        let tn = match vt { 2=>"VT_I2", 3=>"VT_I4", 30=>"VT_LPSTR", 64=>"VT_FILETIME", _=>"?" };
                        print!("    PID {} @ offset={} type={}", pid, poff, tn);
                        if vt == 2 && vo + 6 <= data.len() {
                            print!(" val={}", i16::from_le_bytes([data[vo+4], data[vo+5]]));
                        } else if vt == 3 && vo + 8 <= data.len() {
                            print!(" val={}", i32::from_le_bytes([data[vo+4], data[vo+5], data[vo+6], data[vo+7]]));
                        } else if vt == 30 && vo + 8 <= data.len() {
                            let slen = u32::from_le_bytes([data[vo+4], data[vo+5], data[vo+6], data[vo+7]]) as usize;
                            let s_start = vo + 8;
                            let s_end = (s_start + slen.saturating_sub(1)).min(data.len());
                            if let Ok(s) = String::from_utf8(data[s_start..s_end].to_vec()) {
                                print!(" val=\"{}\"", s);
                            }
                        }
                        println!();
                    }
                }
            }
        }
    } else {
        println!("\n  NO SummaryInformation stream!");
    }
    
    // Read all streams to find StringPool
    for (p, is_stream) in &entries {
        if !is_stream { continue; }
        let name = match p.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };
        if let Ok(mut stream) = comp.open_stream(p) {
            let mut data = Vec::new();
            stream.read_to_end(&mut data).unwrap();
            if data.len() >= 4 {
                let first_u32 = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                if first_u32 == 1252 || first_u32 == 0x800004E4 {
                    println!("\n  StringPool stream '{}' ({} bytes):", name, data.len());
                    println!("    Codepage: 0x{:08X}", first_u32);
                    let remaining = &data[4..];
                    let entry_count = remaining.len() / 4;
                    println!("    Entries: {}", entry_count);
                    for i in 0..entry_count.min(20) {
                        let off = i * 4;
                        if off + 4 <= remaining.len() {
                            let len = u16::from_le_bytes([remaining[off], remaining[off+1]]);
                            let refs = u16::from_le_bytes([remaining[off+2], remaining[off+3]]);
                            println!("    [{}] len={}, refs={}", i, len, refs);
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    inspect("target/test_velocity_msi.msi");
    
    let system_msi = "C:\\Windows\\Installer\\10d16cbb.msi";
    if Path::new(system_msi).exists() {
        inspect(system_msi);
    }
}
