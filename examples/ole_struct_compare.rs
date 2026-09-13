/// Compare OLE structure of two MSIs byte-by-byte at the header/directory level

fn main() {
    let vel = std::fs::read("velocity_ole.msi").unwrap();
    let cfb = std::fs::read("cfb_msi.msi").unwrap();

    println!("velocity_ole.msi: {} bytes", vel.len());
    println!("cfb_msi.msi:      {} bytes", cfb.len());

    // Compare headers
    println!("\n=== Header comparison ===");
    compare_header(&vel, &cfb);

    // Compare directory entries
    println!("\n=== Directory comparison ===");
    compare_directories(&vel, &cfb);

    // Compare FAT entries
    println!("\n=== FAT comparison ===");
    compare_fat(&vel, &cfb);
}

fn read_u16(data: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([data[off], data[off + 1]])
}
fn read_u32(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([data[off], data[off+1], data[off+2], data[off+3]])
}
fn read_i32(data: &[u8], off: usize) -> i32 {
    read_u32(data, off) as i32
}

fn compare_header(vel: &[u8], cfb: &[u8]) {
    let fields = vec![
        (0, 8, "Signature"),
        (8, 16, "CLSID"),
        (24, 2, "Minor Version"),
        (26, 2, "Major Version"),
        (28, 2, "Byte Order"),
        (30, 2, "Sector Shift"),
        (32, 2, "Mini Sector Shift"),
        (34, 6, "Reserved"),
        (40, 4, "NumDirSectors"),
        (44, 4, "NumFATSectors"),
        (48, 4, "FirstDirSector"),
        (52, 4, "TransactionSig"),
        (56, 4, "MiniStreamCutoff"),
        (60, 4, "FirstMiniFATSector"),
        (64, 4, "NumMiniFATSectors"),
        (68, 4, "FirstDIFATSector"),
        (72, 4, "NumDIFATSectors"),
    ];

    for (off, len, name) in &fields {
        let v = &vel[*off..*off + *len];
        let c = &cfb[*off..*off + *len];
        let match_str = if v == c { "MATCH" } else { "DIFF" };
        let v_hex = v.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join("");
        let c_hex = c.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join("");
        println!("{}: {} = {} vs {}", match_str, name, v_hex, c_hex);
    }

    // DIFAT array (109 entries starting at offset 76)
    println!("\nDIFAT entries (header):");
    for i in 0..109 {
        let off = 76 + i * 4;
        let v = read_u32(vel, off);
        let c = read_u32(cfb, off);
        if v != c || v != 0xFFFFFFFF {
            let match_str = if v == c { "MATCH" } else { "DIFF" };
            println!("  [{}]: vel=0x{:08X} cfb=0x{:08X} {}", i, v, c, match_str);
        }
    }
}

fn compare_directories(vel: &[u8], cfb: &[u8]) {
    let vel_dir_sec = read_u32(vel, 48) as usize;
    let cfb_dir_sec = read_u32(cfb, 48) as usize;
    let vel_base = 512 + vel_dir_sec * 512;
    let cfb_base = 512 + cfb_dir_sec * 512;

    // Compare up to 16 directory entries
    for idx in 0..16 {
        let v_off = vel_base + idx * 128;
        let c_off = cfb_base + idx * 128;

        if v_off + 128 > vel.len() || c_off + 128 > cfb.len() {
            break;
        }

        // Read name
        let name_len_v = read_u16(vel, v_off + 64) as usize;
        let name_len_c = read_u16(cfb, c_off + 64) as usize;

        let name_bytes_v = name_len_v.saturating_sub(2);
        let name_bytes_c = name_len_c.saturating_sub(2);

        let mut name_v = String::new();
        for i in (0..name_bytes_v).step_by(2) {
            if let Some(ch) = char::from_u32(read_u16(vel, v_off + i) as u32) {
                name_v.push(ch);
            }
        }
        let mut name_c = String::new();
        for i in (0..name_bytes_c).step_by(2) {
            if let Some(ch) = char::from_u32(read_u16(cfb, c_off + i) as u32) {
                name_c.push(ch);
            }
        }

        if name_len_v == 0 && name_len_c == 0 {
            break; // End of directory
        }

        let obj_v = vel[v_off + 66];
        let obj_c = cfb[c_off + 66];
        let color_v = vel[v_off + 67];
        let color_c = cfb[c_off + 67];
        let left_v = read_i32(vel, v_off + 68);
        let left_c = read_i32(cfb, c_off + 68);
        let right_v = read_i32(vel, v_off + 72);
        let right_c = read_i32(cfb, c_off + 72);
        let child_v = read_i32(vel, v_off + 76);
        let child_c = read_i32(cfb, c_off + 76);
        let clsid_v = &vel[v_off + 80..v_off + 96];
        let clsid_c = &cfb[c_off + 80..c_off + 96];
        let start_v = read_u32(vel, v_off + 116);
        let start_c = read_u32(cfb, c_off + 116);
        let size_v = u64::from_le_bytes([
            vel[v_off+120], vel[v_off+121], vel[v_off+122], vel[v_off+123],
            vel[v_off+124], vel[v_off+125], vel[v_off+126], vel[v_off+127],
        ]);
        let size_c = u64::from_le_bytes([
            cfb[c_off+120], cfb[c_off+121], cfb[c_off+122], cfb[c_off+123],
            cfb[c_off+124], cfb[c_off+125], cfb[c_off+126], cfb[c_off+127],
        ]);

        let name_match = if name_v == name_c { "" } else { " NAME_DIFF!" };
        println!("\nEntry {}:{} '{}'", idx, name_match,
            if name_v == name_c { name_v.clone() } else { format!("VEL:'{}' CFB:'{}'", name_v, name_c) });
        println!("  obj_type: vel={} cfb={}", obj_v, obj_c);
        println!("  color:    vel={} cfb={}", color_v, color_c);
        println!("  left:     vel={} cfb={}", left_v, left_c);
        println!("  right:    vel={} cfb={}", right_v, right_c);
        println!("  child:    vel={} cfb={}", child_v, child_c);
        println!("  CLSID:    vel={} cfb={}",
            clsid_v.iter().any(|b| *b != 0),
            clsid_c.iter().any(|b| *b != 0));
        if clsid_v != clsid_c {
            println!("    VEL CLSID: {:02X?}", clsid_v);
            println!("    CFB CLSID: {:02X?}", clsid_c);
        }
        println!("  start:    vel={} cfb={}", start_v, start_c);
        println!("  size:     vel={} cfb={}", size_v, size_c);
    }
}

fn compare_fat(vel: &[u8], cfb: &[u8]) {
    // Get FAT sector locations from DIFAT
    let vel_fat_sec = read_u32(vel, 76) as usize; // First DIFAT entry = first FAT sector
    let cfb_fat_sec = read_u32(cfb, 76) as usize;

    let vel_fat_base = 512 + vel_fat_sec * 512;
    let cfb_fat_base = 512 + cfb_fat_sec * 512;

    let num_fat_vel = read_u32(vel, 44) as usize;
    let num_fat_cfb = read_u32(cfb, 44) as usize;

    println!("FAT sectors: vel={} cfb={}", num_fat_vel, num_fat_cfb);

    // Compare first FAT sector entries
    let max_entries = 512 / 4; // 128 entries per sector
    let mut diffs = 0;
    for i in 0..max_entries {
        let v = read_u32(vel, vel_fat_base + i * 4);
        let c = read_u32(cfb, cfb_fat_base + i * 4);
        if v != c {
            if diffs < 20 {
                println!("  FAT[{}]: vel=0x{:08X} cfb=0x{:08X}", i, v, c);
            }
            diffs += 1;
        }
    }
    if diffs > 20 {
        println!("  ... {} total FAT diffs", diffs);
    }
    if diffs == 0 {
        println!("  FAT entries identical (first sector)");
    }
}
