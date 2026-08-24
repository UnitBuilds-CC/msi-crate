/// Detailed SummaryInfo binary comparison between reference and velocity-msi.
use std::io::{Read, Cursor};

fn main() {
    let ws_root = env!("CARGO_MANIFEST_DIR").to_string() + "/../..";
    let ref_path = format!("{}/python_ref.msi", ws_root);
    let our_path = format!("{}/velocity_comp.msi", ws_root);

    let ref_data = std::fs::read(&ref_path).unwrap();
    let our_data = std::fs::read(&our_path).unwrap();

    let mut ref_comp = cfb::CompoundFile::open(Cursor::new(&ref_data)).unwrap();
    let mut our_comp = cfb::CompoundFile::open(Cursor::new(&our_data)).unwrap();

    let ref_summary = find_stream(&mut ref_comp, "SummaryInformation");
    let our_summary = find_stream(&mut our_comp, "SummaryInformation");

    println!("=== Reference SummaryInfo ({} bytes) ===", ref_summary.len());
    dump_summary(&ref_summary);
    
    println!("\n=== Our SummaryInfo ({} bytes) ===", our_summary.len());
    dump_summary(&our_summary);

    // Byte-by-byte comparison
    println!("\n=== Byte-by-byte diff ===");
    let min_len = ref_summary.len().min(our_summary.len());
    for i in 0..min_len {
        if ref_summary[i] != our_summary[i] {
            println!("  Byte {}: ref=0x{:02x} our=0x{:02x}", i, ref_summary[i], our_summary[i]);
        }
    }
    if ref_summary.len() != our_summary.len() {
        println!("  Length: ref={} our={}", ref_summary.len(), our_summary.len());
        if our_summary.len() > min_len {
            print!("  Extra in ours: ");
            for b in &our_summary[min_len..] { print!("{:02x} ", b); }
            println!();
        }
        if ref_summary.len() > min_len {
            print!("  Extra in ref:  ");
            for b in &ref_summary[min_len..] { print!("{:02x} ", b); }
            println!();
        }
    }
}

fn find_stream(comp: &mut cfb::CompoundFile<Cursor<&Vec<u8>>>, name_part: &str) -> Vec<u8> {
    let paths: Vec<_> = comp.walk()
        .filter(|e| e.is_stream() && e.path().to_string_lossy().contains(name_part))
        .map(|e| e.path().to_path_buf())
        .collect();
    if paths.is_empty() { panic!("Stream not found: {}", name_part); }
    let mut stream = comp.open_stream(&paths[0]).unwrap();
    let mut data = Vec::new();
    stream.read_to_end(&mut data).unwrap();
    data
}

fn dump_summary(data: &[u8]) {
    if data.len() < 48 {
        println!("  Too short for property set header");
        return;
    }
    
    // Property Set Header
    let bom = u16::from_le_bytes([data[0], data[1]]);
    let fmt_ver = u16::from_le_bytes([data[2], data[3]]);
    let os_ver = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let section_count = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
    let section_offset = u32::from_le_bytes([data[44], data[45], data[46], data[47]]);
    
    println!("  BOM: 0x{:04x}", bom);
    println!("  Format version: {}", fmt_ver);
    println!("  OS version: 0x{:08x}", os_ver);
    println!("  Section count: {}", section_count);
    println!("  Section offset: {}", section_offset);
    
    // FMTID at offset 28..44
    print!("  FMTID: ");
    for b in &data[28..44] { print!("{:02x}", b); }
    println!();
    
    // Section
    let sec_start = section_offset as usize;
    if sec_start + 8 > data.len() { return; }
    let sec_size = u32::from_le_bytes([data[sec_start], data[sec_start+1], data[sec_start+2], data[sec_start+3]]);
    let num_props = u32::from_le_bytes([data[sec_start+4], data[sec_start+5], data[sec_start+6], data[sec_start+7]]);
    
    println!("  Section size: {}", sec_size);
    println!("  Property count: {}", num_props);
    
    // Property index
    let idx_start = sec_start + 8;
    for i in 0..num_props as usize {
        let off = idx_start + i * 8;
        if off + 8 > data.len() { break; }
        let pid = u32::from_le_bytes([data[off], data[off+1], data[off+2], data[off+3]]);
        let prop_off = u32::from_le_bytes([data[off+4], data[off+5], data[off+6], data[off+7]]);
        
        // Read property type
        let abs_off = sec_start + prop_off as usize;
        if abs_off + 4 <= data.len() {
            let vtype = u32::from_le_bytes([data[abs_off], data[abs_off+1], data[abs_off+2], data[abs_off+3]]);
            let type_name = match vtype {
                2 => "VT_I2",
                3 => "VT_I4",
                30 => "VT_LPSTR",
                64 => "VT_FILETIME",
                _ => "unknown",
            };
            
            // Read value based on type
            let value_str = match vtype {
                2 if abs_off + 8 <= data.len() => {
                    let v = i16::from_le_bytes([data[abs_off+4], data[abs_off+5]]);
                    format!("{}", v)
                }
                3 if abs_off + 8 <= data.len() => {
                    let v = i32::from_le_bytes([data[abs_off+4], data[abs_off+5], data[abs_off+6], data[abs_off+7]]);
                    format!("{}", v)
                }
                30 if abs_off + 8 <= data.len() => {
                    let str_len = u32::from_le_bytes([data[abs_off+4], data[abs_off+5], data[abs_off+6], data[abs_off+7]]);
                    let str_start = abs_off + 8;
                    let str_end = (str_start + str_len as usize - 1).min(data.len());
                    let s = String::from_utf8_lossy(&data[str_start..str_end]);
                    format!("\"{}\" (len={})", s, str_len)
                }
                64 if abs_off + 12 <= data.len() => {
                    let ft = u64::from_le_bytes([data[abs_off+4], data[abs_off+5], data[abs_off+6], data[abs_off+7],
                                                  data[abs_off+8], data[abs_off+9], data[abs_off+10], data[abs_off+11]]);
                    format!("FILETIME={}", ft)
                }
                _ => format!("type={}", vtype),
            };
            
            println!("  PID {} @ offset {}: {} = {}", pid, prop_off, type_name, value_str);
        }
    }
}
