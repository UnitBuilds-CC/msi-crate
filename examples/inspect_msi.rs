use cfb::CompoundFile;
use std::fs::File;
use std::io::Read;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = if args.len() > 1 {
        &args[1]
    } else {
        "target/test_velocity_msi.msi"
    };

    let file = File::open(path).unwrap();
    let mut comp = CompoundFile::open(file).unwrap();

    println!("Streams in {}:", path);
    for entry in comp.walk() {
        let is_stream = entry.is_stream();
        println!(
            "  {} [{}]",
            entry.path().display(),
            if is_stream { "stream" } else { "storage" }
        );
    }

    // Read SummaryInformation if present
    let summary_path = Path::new("\u{0005}SummaryInformation");
    if let Ok(mut stream) = comp.open_stream(summary_path) {
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        println!("\nSummaryInformation: {} bytes", data.len());
        println!("First 80 bytes:");
        for (i, chunk) in data.chunks(16).enumerate().take(5) {
            print!("  {:04x}: ", i * 16);
            for b in chunk {
                print!("{:02x} ", b);
            }
            println!();
        }

        // Parse header
        if data.len() >= 48 {
            let bom = u16::from_le_bytes([data[0], data[1]]);
            let version = u16::from_le_bytes([data[2], data[3]]);
            let os_ver = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
            let num_sections = u32::from_le_bytes([data[40], data[41], data[42], data[43]]);
            let section_offset = u32::from_le_bytes([data[44], data[45], data[46], data[47]]);
            println!("\n  BOM: 0x{:04X}", bom);
            println!("  Version: {}", version);
            println!("  OS: 0x{:08X}", os_ver);
            println!("  Sections: {}", num_sections);
            println!("  Section offset: {}", section_offset);

            // Parse section
            if (section_offset as usize + 8) < data.len() {
                let sec_start = section_offset as usize;
                let sec_size = u32::from_le_bytes([
                    data[sec_start],
                    data[sec_start + 1],
                    data[sec_start + 2],
                    data[sec_start + 3],
                ]);
                let num_props = u32::from_le_bytes([
                    data[sec_start + 4],
                    data[sec_start + 5],
                    data[sec_start + 6],
                    data[sec_start + 7],
                ]);
                println!("  Section size: {}", sec_size);
                println!("  Properties: {}", num_props);

                // Parse property index
                for i in 0..num_props as usize {
                    let entry_off = sec_start + 8 + i * 8;
                    if entry_off + 8 > data.len() {
                        break;
                    }
                    let prop_id = u32::from_le_bytes([
                        data[entry_off],
                        data[entry_off + 1],
                        data[entry_off + 2],
                        data[entry_off + 3],
                    ]);
                    let prop_offset = u32::from_le_bytes([
                        data[entry_off + 4],
                        data[entry_off + 5],
                        data[entry_off + 6],
                        data[entry_off + 7],
                    ]);
                    let val_off = sec_start + prop_offset as usize;
                    if val_off + 4 <= data.len() {
                        let vtype = u32::from_le_bytes([
                            data[val_off],
                            data[val_off + 1],
                            data[val_off + 2],
                            data[val_off + 3],
                        ]);
                        let type_name = match vtype {
                            2 => "VT_I2",
                            3 => "VT_I4",
                            30 => "VT_LPSTR",
                            64 => "VT_FILETIME",
                            _ => "unknown",
                        };
                        print!(
                            "  Prop {}: type={} ({}) offset={}",
                            prop_id, vtype, type_name, prop_offset
                        );
                        if vtype == 2 && val_off + 8 <= data.len() {
                            let val =
                                i16::from_le_bytes([data[val_off + 4], data[val_off + 5]]);
                            print!(" value={}", val);
                        } else if vtype == 3 && val_off + 8 <= data.len() {
                            let val = i32::from_le_bytes([
                                data[val_off + 4],
                                data[val_off + 5],
                                data[val_off + 6],
                                data[val_off + 7],
                            ]);
                            print!(" value={}", val);
                        } else if vtype == 30 && val_off + 8 <= data.len() {
                            let slen = u32::from_le_bytes([
                                data[val_off + 4],
                                data[val_off + 5],
                                data[val_off + 6],
                                data[val_off + 7],
                            ]);
                            let str_start = val_off + 8;
                            let str_end =
                                (str_start + slen as usize - 1).min(data.len());
                            if let Ok(s) =
                                String::from_utf8(data[str_start..str_end].to_vec())
                            {
                                print!(" value=\"{}\"", s);
                            }
                        }
                        println!();
                    }
                }
            }
        }
    } else {
        println!("\nNo SummaryInformation stream found!");
    }

    // Read string pool
    let pool_path = Path::new("0StringPool");
    if let Ok(mut stream) = comp.open_stream(pool_path) {
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        println!("\n0StringPool: {} bytes", data.len());
        if data.len() >= 8 {
            let num_entries = data.len() / 8;
            println!("  Entries: {}", num_entries);
            for i in 0..num_entries.min(10) {
                let off = i * 8;
                let id = u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]);
                let refs = u16::from_le_bytes([data[off + 4], data[off + 5]]);
                let offset = u16::from_le_bytes([data[off + 6], data[off + 7]]);
                println!("  ID={}, refs={}, offset={}", id, refs, offset);
            }
        }
    }

    let data_path = Path::new("0StringData");
    if let Ok(mut stream) = comp.open_stream(data_path) {
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        println!("\n0StringData: {} bytes", data.len());
        let mut pos = 0;
        let mut count = 0;
        while pos < data.len() && count < 15 {
            let start = pos;
            while pos < data.len() && data[pos] != 0 {
                pos += 1;
            }
            let s = String::from_utf8_lossy(&data[start..pos]);
            println!("  [{}] \"{}\"", start, s);
            pos += 1;
            count += 1;
        }
    }

    // Read table streams
    for table_name in &["_Tables", "_Columns", "_Validation", "Property"] {
        let stream_name = if table_name.starts_with('_') {
            format!("0{}", &table_name[1..])
        } else {
            table_name.to_string()
        };
        let tpath = Path::new(&stream_name);
        if let Ok(mut stream) = comp.open_stream(tpath) {
            let mut data = Vec::new();
            stream.read_to_end(&mut data).unwrap();
            println!("\n{} (stream '{}'): {} bytes", table_name, stream_name, data.len());
        }
    }
}
