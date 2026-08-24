/// Compare valid.cab (makecab) with our cabinet format
use std::io::Read;

fn parse_cab(label: &str, buf: &[u8]) {
    println!("\n=== {} ({} bytes) ===", label, buf.len());
    if buf.len() < 36 || &buf[0..4] != b"MSCF" {
        println!("Not a valid cabinet!");
        return;
    }

    let cb_cabinet = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
    let coff_files = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);
    let c_folders = u16::from_le_bytes([buf[26], buf[27]]);
    let c_files = u16::from_le_bytes([buf[28], buf[29]]);
    println!("cbCabinet: {} (actual: {})", cb_cabinet, buf.len());
    println!("coffFiles: {}", coff_files);
    println!("cFolders: {}, cFiles: {}", c_folders, c_files);
    println!("Version: {}.{}", buf[25], buf[24]);

    // Parse CFOLDER
    let folder_off = 36;
    let coff_cab_start = u32::from_le_bytes([buf[folder_off], buf[folder_off+1], buf[folder_off+2], buf[folder_off+3]]);
    let cb_cfdata = u16::from_le_bytes([buf[folder_off+4], buf[folder_off+5]]);
    let type_compression = u16::from_le_bytes([buf[folder_off+6], buf[folder_off+7]]);
    println!("\nCFOLDER:");
    println!("  coffCabStart: {}", coff_cab_start);
    println!("  cbCFData: {}", cb_cfdata);
    println!("  typeCompression: {}", type_compression);

    // Parse CFDATA
    let data_off = coff_cab_start as usize;
    if data_off + 8 <= buf.len() {
        let cb_data = u16::from_le_bytes([buf[data_off+4], buf[data_off+5]]);
        let cb_uncomp = u16::from_le_bytes([buf[data_off+6], buf[data_off+7]]);
        println!("\nCFDATA at offset {}:", data_off);
        println!("  Raw: {:02x?}", &buf[data_off..data_off+10.min(buf.len()-data_off)]);
        println!("  cbData: {}", cb_data);
        println!("  cbUncomp: {}", cb_uncomp);
        if data_off + 10 <= buf.len() {
            println!("  Magic: {:02x?}", &buf[data_off+8..data_off+10]);
        }
    }

    // Parse CFFILE
    let file_off = coff_files as usize;
    if file_off + 14 <= buf.len() {
        let cb_file = u32::from_le_bytes([buf[file_off], buf[file_off+1], buf[file_off+2], buf[file_off+3]]);
        let uoff_start = u32::from_le_bytes([buf[file_off+4], buf[file_off+5], buf[file_off+6], buf[file_off+7]]);
        let i_folder = u16::from_le_bytes([buf[file_off+8], buf[file_off+9]]);
        let fl_time = u16::from_le_bytes([buf[file_off+10], buf[file_off+11]]);
        let fl_date = u16::from_le_bytes([buf[file_off+12], buf[file_off+13]]);
        let name_start = file_off + 14;
        let name_end = buf[name_start..].iter().position(|&b| b == 0).unwrap_or(0);
        let file_name = std::str::from_utf8(&buf[name_start..name_start+name_end]).unwrap_or("???");
        println!("\nCFFILE at offset {}:", file_off);
        println!("  Raw: {:02x?}", &buf[file_off..file_off+20.min(buf.len()-file_off)]);
        println!("  cbFile: {}", cb_file);
        println!("  uoffFolderStart: {}", uoff_start);
        println!("  iFolder: {}", i_folder);
        println!("  flTime: 0x{:04X}, flDate: 0x{:04X}", fl_time, fl_date);
        println!("  FileName: '{}'", file_name);
    }
}

fn main() {
    // Read the makecab-generated cabinet
    let valid_cab = std::fs::read("C:\\temp\\valid.cab").unwrap();
    parse_cab("makecab valid.cab", &valid_cab);

    // Read our cabinet
    let our_cab = std::fs::read("C:\\temp\\velcab.cab").unwrap();
    parse_cab("our velcab.cab", &our_cab);

    // Hex dump first 64 bytes of each for comparison
    println!("\n=== Byte comparison (first 64 bytes) ===");
    println!("makecab:");
    for i in (0..64.min(valid_cab.len())).step_by(16) {
        print!("  {:04x}: ", i);
        for j in 0..16.min(valid_cab.len()-i) {
            print!("{:02x} ", valid_cab[i+j]);
        }
        println!();
    }
    println!("ours:");
    for i in (0..64.min(our_cab.len())).step_by(16) {
        print!("  {:04x}: ", i);
        for j in 0..16.min(our_cab.len()-i) {
            print!("{:02x} ", our_cab[i+j]);
        }
        println!();
    }
}
