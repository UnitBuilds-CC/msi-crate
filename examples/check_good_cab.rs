/// Check file names in good.cab
fn main() {
    let cab = std::fs::read("C:\\temp\\good.cab").unwrap();
    println!("good.cab: {} bytes", cab.len());
    
    // Parse CFHEADER
    let coff_files = u32::from_le_bytes([cab[16], cab[17], cab[18], cab[19]]);
    let c_files = u16::from_le_bytes([cab[28], cab[29]]);
    println!("coffFiles: {}, cFiles: {}", coff_files, c_files);
    
    // Parse CFFILE entries
    let mut off = coff_files as usize;
    for i in 0..c_files {
        let cb_file = u32::from_le_bytes([cab[off], cab[off+1], cab[off+2], cab[off+3]]);
        let uoff = u32::from_le_bytes([cab[off+4], cab[off+5], cab[off+6], cab[off+7]]);
        // Read name starting at offset 16 (after attribs field)
        let name_start = off + 16;
        let name_end = cab[name_start..].iter().position(|&b| b == 0).unwrap_or(0);
        let name = std::str::from_utf8(&cab[name_start..name_start+name_end]).unwrap_or("???");
        println!("File {}: cbFile={}, uoff={}, name='{}'", i, cb_file, uoff, name);
        
        // Also check offset 14 (without attribs)
        let name_start_14 = off + 14;
        let name_end_14 = cab[name_start_14..].iter().position(|&b| b == 0).unwrap_or(0);
        let name_14 = std::str::from_utf8(&cab[name_start_14..name_start_14+name_end_14]).unwrap_or("???");
        println!("  (without attribs): name='{}'", name_14);
        
        // Move to next file entry
        off = name_start + name_end + 1;
    }
    
    // Full hex dump
    println!("\nFull hex dump:");
    for i in (0..cab.len()).step_by(16) {
        print!("{:04x}: ", i);
        for j in 0..16 {
            if i + j < cab.len() { print!("{:02x} ", cab[i+j]); }
        }
        print!(" ");
        for j in 0..16 {
            if i + j < cab.len() {
                let b = cab[i+j];
                if b >= 0x20 && b < 0x7F { print!("{}", b as char); } else { print!("."); }
            }
        }
        println!();
    }
}
