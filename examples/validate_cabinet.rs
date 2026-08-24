/// Extract and validate cabinet from MSI - byte-level analysis
use std::io::{Cursor, Read};

fn main() {
    let path = "C:\\temp\\complete_test.msi";
    let data = std::fs::read(path).unwrap();
    let mut comp = cfb::CompoundFile::open(Cursor::new(&data)).unwrap();

    // Find the cabinet stream (starts with MSCF)
    let stream_names: Vec<String> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.name().to_string())
        .collect();

    for name in &stream_names {
        let mut s = comp.open_stream(name).unwrap();
        let mut buf = Vec::new();
        s.read_to_end(&mut buf).unwrap();

        if buf.len() >= 36 && &buf[0..4] == b"MSCF" {
            println!("Found cabinet: {} ({} bytes)\n", name, buf.len());
            std::fs::write("C:\\temp\\test.cab", &buf).unwrap();

            // Parse CFHEADER
            let cb_cabinet = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
            let coff_files = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);
            let c_folders = u16::from_le_bytes([buf[26], buf[27]]);
            let c_files = u16::from_le_bytes([buf[28], buf[29]]);
            println!("=== CFHEADER ===");
            println!("cbCabinet: {} (actual: {})", cb_cabinet, buf.len());
            println!("coffFiles: {}", coff_files);
            println!("cFolders: {}", c_folders);
            println!("cFiles: {}", c_files);
            println!("Version: {}.{}", buf[25], buf[24]);

            // Parse CFOLDER (at offset 36, 8 bytes)
            let folder_off = 36;
            let coff_cab_start = u32::from_le_bytes([buf[folder_off], buf[folder_off+1], buf[folder_off+2], buf[folder_off+3]]);
            let cb_cfdata = u16::from_le_bytes([buf[folder_off+4], buf[folder_off+5]]);
            let type_compression = u16::from_le_bytes([buf[folder_off+6], buf[folder_off+7]]);
            println!("\n=== CFOLDER (offset {}) ===", folder_off);
            println!("coffCabStart: {}", coff_cab_start);
            println!("cbCFData: {}", cb_cfdata);
            println!("typeCompression: {}", type_compression);

            // Verify CFOLDER is 8 bytes
            let expected_data_start = folder_off as u32 + 8;
            println!("Expected CFDATA at: {} (coffCabStart says: {})", expected_data_start, coff_cab_start);
            if coff_cab_start != expected_data_start {
                println!("*** MISMATCH: coffCabStart should be {} but is {} ***", expected_data_start, coff_cab_start);
            }

            // Parse CFDATA
            let data_off = coff_cab_start as usize;
            println!("\n=== CFDATA (offset {}) ===", data_off);
            println!("Raw bytes [0..10]: {:02x?}", &buf[data_off..data_off+10.min(buf.len()-data_off)]);
            
            let c_checksum = u32::from_le_bytes([buf[data_off], buf[data_off+1], buf[data_off+2], buf[data_off+3]]);
            let cb_data = u16::from_le_bytes([buf[data_off+4], buf[data_off+5]]);
            let cb_uncomp = u16::from_le_bytes([buf[data_off+6], buf[data_off+7]]);
            println!("cChecksum: {}", c_checksum);
            println!("cbData: {}", cb_data);
            println!("cbUncomp: {}", cb_uncomp);
            
            if data_off + 8 + cb_data as usize <= buf.len() {
                println!("Magic: {:02x?} ({:?})", &buf[data_off+8..data_off+10],
                    std::str::from_utf8(&buf[data_off+8..data_off+10]).unwrap_or("non-ascii"));
                println!("First compressed bytes: {:02x?}", &buf[data_off+10..data_off+10+8.min(buf.len()-data_off-10)]);
            } else {
                println!("*** cbData extends beyond cabinet! ***");
            }

            // Verify cbCFData matches actual CFDATA size
            let actual_cfdata_size = 8 + cb_data as u32;
            println!("\ncbCFData says: {}, actual CFDATA size: {}", cb_cfdata, actual_cfdata_size);
            if cb_cfdata as u32 != actual_cfdata_size {
                println!("*** MISMATCH ***");
            }

            // Parse CFFILE
            let file_off = coff_files as usize;
            println!("\n=== CFFILE (offset {}) ===", file_off);
            println!("Raw bytes [0..20]: {:02x?}", &buf[file_off..file_off+20.min(buf.len()-file_off)]);
            
            let cb_file = u32::from_le_bytes([buf[file_off], buf[file_off+1], buf[file_off+2], buf[file_off+3]]);
            let uoff_start = u32::from_le_bytes([buf[file_off+4], buf[file_off+5], buf[file_off+6], buf[file_off+7]]);
            let i_folder = u16::from_le_bytes([buf[file_off+8], buf[file_off+9]]);
            let fl_time = u16::from_le_bytes([buf[file_off+10], buf[file_off+11]]);
            let fl_date = u16::from_le_bytes([buf[file_off+12], buf[file_off+13]]);
            
            let name_start = file_off + 14;
            let name_end = buf[name_start..].iter().position(|&b| b == 0).unwrap_or(0);
            let file_name = std::str::from_utf8(&buf[name_start..name_start+name_end]).unwrap_or("???");
            
            println!("cbFile: {}", cb_file);
            println!("uoffFolderStart: {}", uoff_start);
            println!("iFolder: {}", i_folder);
            println!("flTime: 0x{:04X}", fl_time);
            println!("flDate: 0x{:04X}", fl_date);
            println!("FileName: '{}' ({} bytes)", file_name, name_end);
            
            // Verify total size
            let expected_total = file_off + 14 + name_end + 1;
            println!("\n=== SIZE CHECK ===");
            println!("Expected total: {}", expected_total);
            println!("cbCabinet says: {}", cb_cabinet);
            println!("Actual size: {}", buf.len());

            // Also try to verify with Windows expand
            println!("\n=== Trying expand.exe ===");
            let output = std::process::Command::new("expand")
                .args(&["C:\\temp\\test.cab", "-F:*", "C:\\temp\\cab_extract"])
                .output();
            match output {
                Ok(o) => {
                    println!("expand exit: {}", o.status);
                    println!("stdout: {}", String::from_utf8_lossy(&o.stdout));
                    println!("stderr: {}", String::from_utf8_lossy(&o.stderr));
                }
                Err(e) => println!("expand error: {}", e),
            }
        }
    }
}
