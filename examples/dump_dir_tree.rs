/// Dump OLE directory entries to verify the BST structure.
use velocity_msi::{MsiBuilder, Column, Value};

const SECTOR_SIZE: usize = 512;
const HEADER_SIZE: usize = 512;
const DIR_ENTRY_SIZE: usize = 128;

fn read_u16(data: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([data[off], data[off + 1]])
}
fn read_u32(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([data[off], data[off+1], data[off+2], data[off+3]])
}
fn read_i32(data: &[u8], off: usize) -> i32 {
    read_u32(data, off) as i32
}

fn main() {
    let mut b = MsiBuilder::new();
    b.set_title("TreeDiag");
    b.set_author("V");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("TreeDiag")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("V")],
        vec![Value::from("ProductCode"), Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![Value::from("UpgradeCode"), Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    b.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
    ]).unwrap();

    b.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
    ]).unwrap();

    let msi_data = b.build().unwrap();
    
    // Parse header
    let first_dir_sector = read_u32(&msi_data, 48);
    let first_fat_sector = read_u32(&msi_data, 76);
    println!("First FAT sector: {}", first_fat_sector);
    println!("First dir sector: {}", first_dir_sector);
    
    // Read FAT entries
    let fat_base = HEADER_SIZE + first_dir_sector as usize * 0; // FAT is at sector 0
    println!("\n=== FAT entries ===");
    for i in 0..15 {
        let off = HEADER_SIZE + i * 4; // FAT at sector 0
        let val = read_u32(&msi_data, off);
        let desc = match val {
            0xFFFFFFFE => "ENDOFCHAIN",
            0xFFFFFFFD => "FATSECT",
            0xFFFFFFFF => "FREE",
            _ => &format!("→ sector {}", val),
        };
        println!("  FAT[{:2}] = 0x{:08X} ({})", i, val, desc);
    }
    
    // Parse directory entries
    let dir_base = HEADER_SIZE + first_dir_sector as usize * SECTOR_SIZE;
    println!("\n=== Directory entries (at offset 0x{:X}) ===", dir_base);
    
    for idx in 0..12 {
        let off = dir_base + idx * DIR_ENTRY_SIZE;
        if off + DIR_ENTRY_SIZE > msi_data.len() { break; }
        
        let name_len = read_u16(&msi_data, off + 64) as usize;
        if name_len == 0 { 
            println!("Entry {}: (empty)", idx);
            continue;
        }
        
        let mut name_utf16 = Vec::new();
        for j in 0..((name_len - 2) / 2) {
            name_utf16.push(read_u16(&msi_data, off + j * 2));
        }
        let name = String::from_utf16_lossy(&name_utf16);
        
        let obj_type = msi_data[off + 66];
        let color = msi_data[off + 67];
        let left = read_i32(&msi_data, off + 68);
        let right = read_i32(&msi_data, off + 72);
        let child = read_i32(&msi_data, off + 76);
        let start_sect = read_u32(&msi_data, off + 116);
        let stream_size = read_u32(&msi_data, off + 120);
        
        let type_name = match obj_type {
            0 => "Empty",
            1 => "Storage",
            2 => "Stream",
            5 => "Root",
            _ => "Unknown",
        };
        
        println!("Entry {}: name={:?} type={} color={} L={} R={} child={} start={} size={}",
            idx, name, type_name, color, left, right, child, start_sect, stream_size);
    }
    
    // Verify BST: traverse from root's child
    println!("\n=== BST traversal ===");
    let root_child = read_i32(&msi_data, dir_base + 76);
    println!("Root's child: {}", root_child);
    
    fn traverse(data: &[u8], dir_base: usize, pos: i32, depth: usize) {
        if pos < 0 { return; }
        let off = dir_base + pos as usize * DIR_ENTRY_SIZE;
        let name_len = read_u16(data, off + 64) as usize;
        let mut name_utf16 = Vec::new();
        for j in 0..((name_len - 2) / 2) {
            name_utf16.push(read_u16(data, off + j * 2));
        }
        let name = String::from_utf16_lossy(&name_utf16);
        let left = read_i32(data, off + 68);
        let right = read_i32(data, off + 72);
        
        let indent = "  ".repeat(depth);
        println!("{}[{}] {} (L={}, R={})", indent, pos, name, left, right);
        
        traverse(data, dir_base, left, depth + 1);
        traverse(data, dir_base, right, depth + 1);
    }
    
    traverse(&msi_data, dir_base, root_child, 0);
    
    // Count reachable entries
    fn count_reachable(data: &[u8], dir_base: usize, pos: i32) -> usize {
        if pos < 0 { return 0; }
        1 + count_reachable(data, dir_base, read_i32(data, dir_base + pos as usize * DIR_ENTRY_SIZE + 68))
          + count_reachable(data, dir_base, read_i32(data, dir_base + pos as usize * DIR_ENTRY_SIZE + 72))
    }
    
    let reachable = count_reachable(&msi_data, dir_base, root_child);
    println!("\nReachable entries from root's child: {}", reachable);
}
