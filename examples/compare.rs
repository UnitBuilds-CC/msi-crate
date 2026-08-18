fn main() {
    let our = std::fs::read("target/test_velocity.msi").unwrap();
    let cfb = std::fs::read("target/cfb_test.msi").unwrap();
    
    println!("Comparing headers (our vs cfb):");
    println!("Off  Our    Cfb    Field");
    let fields: Vec<(usize, usize, &str)> = vec![
        (0, 8, "Magic"),
        (24, 2, "Minor ver"),
        (26, 2, "Major ver"),
        (28, 2, "Byte order"),
        (30, 2, "Sector shift"),
        (32, 2, "Mini sect shift"),
        (40, 4, "Dir sectors"),
        (44, 4, "FAT sectors"),
        (48, 4, "First dir SECT"),
        (52, 4, "Transaction sig"),
        (56, 4, "Mini cutoff"),
        (60, 4, "First miniFAT"),
        (64, 4, "MiniFAT count"),
        (68, 4, "First DIFAT"),
        (72, 4, "DIFAT count"),
        (76, 4, "DIFAT[0]"),
    ];
    
    for (off, len, name) in &fields {
        let our_bytes = &our[*off..*off + *len];
        let cfb_bytes = &cfb[*off..*off + *len];
        let marker = if our_bytes != cfb_bytes { " <-- DIFF" } else { "" };
        if *len == 4 {
            let our_val = u32::from_le_bytes([our_bytes[0], our_bytes[1], our_bytes[2], our_bytes[3]]);
            let cfb_val = u32::from_le_bytes([cfb_bytes[0], cfb_bytes[1], cfb_bytes[2], cfb_bytes[3]]);
            println!("{:3}:  {:8} {:8} {}{} (our=0x{:X} cfb=0x{:X})", off, "", "", name, marker, our_val, cfb_val);
        } else if *len == 2 {
            let our_val = u16::from_le_bytes([our_bytes[0], our_bytes[1]]);
            let cfb_val = u16::from_le_bytes([cfb_bytes[0], cfb_bytes[1]]);
            println!("{:3}:  {:8} {:8} {}{} (our={} cfb={})", off, "", "", name, marker, our_val, cfb_val);
        } else {
            println!("{:3}:  {:?} {:?} {}{}", off, our_bytes, cfb_bytes, name, marker);
        }
    }
    
    // Compare directory entries (at sector 1 = offset 8192)
    let dir_off = 4096 + 1 * 4096; // sector 1
    println!("\nDirectory at offset {}:", dir_off);
    for did in 0..4 {
        let off = dir_off + did * 128;
        if off + 128 > our.len() { break; }
        
        let our_type = our[off + 66];
        let cfb_type = cfb[off + 66];
        
        // Read names
        let our_name_len = u16::from_le_bytes([our[off + 64], our[off + 65]]) as usize;
        let cfb_name_len = u16::from_le_bytes([cfb[off + 64], cfb[off + 65]]) as usize;
        
        println!("  DID {}: our_type={} cfb_type={} our_namelen={} cfb_namelen={}", 
            did, our_type, cfb_type, our_name_len, cfb_name_len);
        
        // Show first 16 bytes of name
        print!("    our name: ");
        for i in 0..16.min(off + 64 - off) { print!("{:02X} ", our[off + i]); }
        println!();
        print!("    cfb name: ");
        for i in 0..16 { print!("{:02X} ", cfb[off + i]); }
        println!();
        
        // Show tree links
        let our_left = i32::from_le_bytes([our[off+68], our[off+69], our[off+70], our[off+71]]);
        let our_right = i32::from_le_bytes([our[off+72], our[off+73], our[off+74], our[off+75]]);
        let our_child = i32::from_le_bytes([our[off+76], our[off+77], our[off+78], our[off+79]]);
        let cfb_left = i32::from_le_bytes([cfb[off+68], cfb[off+69], cfb[off+70], cfb[off+71]]);
        let cfb_right = i32::from_le_bytes([cfb[off+72], cfb[off+73], cfb[off+74], cfb[off+75]]);
        let cfb_child = i32::from_le_bytes([cfb[off+76], cfb[off+77], cfb[off+78], cfb[off+79]]);
        println!("    our: L={} R={} child={}", our_left, our_right, our_child);
        println!("    cfb: L={} R={} child={}", cfb_left, cfb_right, cfb_child);
        
        // Show CLSID
        print!("    our CLSID: ");
        for i in 0..16 { print!("{:02X}", our[off + 80 + i]); }
        println!();
        print!("    cfb CLSID: ");
        for i in 0..16 { print!("{:02X}", cfb[off + 80 + i]); }
        println!();
        
        // Show start sector and size
        let our_start = u32::from_le_bytes([our[off+116], our[off+117], our[off+118], our[off+119]]);
        let our_size = u64::from_le_bytes([our[off+120], our[off+121], our[off+122], our[off+123], our[off+124], our[off+125], our[off+126], our[off+127]]);
        let cfb_start = u32::from_le_bytes([cfb[off+116], cfb[off+117], cfb[off+118], cfb[off+119]]);
        let cfb_size = u64::from_le_bytes([cfb[off+120], cfb[off+121], cfb[off+122], cfb[off+123], cfb[off+124], cfb[off+125], cfb[off+126], cfb[off+127]]);
        println!("    our: start={} size={}", our_start, our_size);
        println!("    cfb: start={} size={}", cfb_start, cfb_size);
    }
    
    // Compare FAT sectors (at sector 0 = offset 4096)
    let fat_off = 4096;
    println!("\nFAT at offset {}:", fat_off);
    for i in 0..8 {
        let our_val = u32::from_le_bytes([our[fat_off+i*4], our[fat_off+i*4+1], our[fat_off+i*4+2], our[fat_off+i*4+3]]);
        let cfb_val = u32::from_le_bytes([cfb[fat_off+i*4], cfb[fat_off+i*4+1], cfb[fat_off+i*4+2], cfb[fat_off+i*4+3]]);
        let marker = if our_val != cfb_val { " <-- DIFF" } else { "" };
        println!("  FAT[{}]: our=0x{:08X} cfb=0x{:08X}{}", i, our_val, cfb_val, marker);
    }
}
