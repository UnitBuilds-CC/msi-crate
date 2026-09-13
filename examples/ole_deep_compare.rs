/// Compare OLE structure of reference MSI (msi crate) vs velocity MSI
/// to find what's wrong with our custom OLE writer.

fn main() {
    // Build reference MSI using msi crate
    let ref_msi = create_ref_msi();
    std::fs::write("ref_struct.msi", &ref_msi).unwrap();

    // Build velocity MSI
    let vel_msi = create_vel_msi();
    std::fs::write("vel_struct.msi", &vel_msi).unwrap();

    println!("ref: {} bytes, vel: {} bytes", ref_msi.len(), vel_msi.len());

    // Compare headers
    println!("\n=== Header ===");
    dump_header(&ref_msi, "REF");
    dump_header(&vel_msi, "VEL");

    // Compare directory entries
    println!("\n=== Directory entries ===");
    dump_directory(&ref_msi, "REF");
    dump_directory(&vel_msi, "VEL");

    // Compare FAT
    println!("\n=== FAT (first 30 entries) ===");
    dump_fat(&ref_msi, "REF", 30);
    dump_fat(&vel_msi, "VEL", 30);

    // Compare MiniFAT
    println!("\n=== MiniFAT (first 80 entries) ===");
    dump_minifat(&ref_msi, "REF", 80);
    dump_minifat(&vel_msi, "VEL", 80);
}

fn read_u16(data: &[u8], off: usize) -> u16 {
    if off + 2 > data.len() { return 0; }
    u16::from_le_bytes([data[off], data[off + 1]])
}
fn read_u32(data: &[u8], off: usize) -> u32 {
    if off + 4 > data.len() { return 0; }
    u32::from_le_bytes([data[off], data[off+1], data[off+2], data[off+3]])
}
fn read_i32(data: &[u8], off: usize) -> i32 {
    read_u32(data, off) as i32
}

fn sector_size(data: &[u8]) -> usize {
    let shift = read_u16(data, 30);
    1 << shift
}

fn dump_header(data: &[u8], label: &str) {
    let ss = sector_size(data);
    println!("[{}] sector_size={}, major={}, minor={}",
        label, ss, read_u16(data, 26), read_u16(data, 24));
    println!("[{}] num_fat={}, first_dir={}, mini_cutoff={}",
        label, read_u32(data, 44), read_u32(data, 48), read_u32(data, 56));
    println!("[{}] first_minifat={}, num_minifat={}, first_difat={}, num_difat={}",
        label, read_u32(data, 60), read_u32(data, 64), read_u32(data, 68), read_u32(data, 72));
    println!("[{}] num_dir_sectors={}", label, read_u32(data, 40));

    // DIFAT
    let mut difat = Vec::new();
    for i in 0..109 {
        let v = read_u32(data, 76 + i * 4);
        if v != 0xFFFFFFFF {
            difat.push(v);
        }
    }
    println!("[{}] DIFAT: {:?}", label, difat);
}

fn dump_directory(data: &[u8], label: &str) {
    let ss = sector_size(data);
    let first_dir = read_u32(data, 48) as usize;
    let dir_base = 512 + first_dir * ss;

    for idx in 0..32 {
        let off = dir_base + idx * 128;
        if off + 128 > data.len() { break; }

        let name_len = read_u16(data, off + 64) as usize;
        if name_len == 0 { break; }

        // Name length includes null terminator, and max is 64 bytes (32 UTF-16 chars)
        let name_bytes = (name_len.saturating_sub(2)).min(62);
        let mut name = String::new();
        for i in (0..name_bytes).step_by(2) {
            if let Some(ch) = char::from_u32(read_u16(data, off + i) as u32) {
                if ch == '\0' { break; }
                name.push(ch);
            }
        }

        let obj_type = data[off + 66];
        let color = data[off + 67];
        let left = read_i32(data, off + 68);
        let right = read_i32(data, off + 72);
        let child = read_i32(data, off + 76);
        let start = read_u32(data, off + 116);
        let size = u64::from_le_bytes([
            data[off+120], data[off+121], data[off+122], data[off+123],
            data[off+124], data[off+125], data[off+126], data[off+127],
        ]);
        let clsid_nonzero = data[off + 80..off + 96].iter().any(|b| *b != 0);

        println!("[{}] dir[{}]: '{}' type={} color={} L={} R={} child={} start={} size={} clsid={}",
            label, idx, name, obj_type, color, left, right, child, start, size, clsid_nonzero);
    }
}

fn dump_fat(data: &[u8], label: &str, count: usize) {
    let ss = sector_size(data);
    let fat_sector = read_u32(data, 76) as usize; // First DIFAT entry
    let fat_base = 512 + fat_sector * ss;
    eprintln!("DEBUG dump_fat: {} fat_sector={}, fat_base={}, ss={}", label, fat_sector, fat_base, ss);

    for i in 0..count {
        let v = read_u32(data, fat_base + i * 4);
        let sym = match v {
            0xFFFFFFFD => "FATSECT",
            0xFFFFFFFE => "EOC",
            0xFFFFFFFF => "FREE",
            _ => if v as usize == i + 1 { "chain" } else { "chain" },
        };
        if v != 0xFFFFFFFF {
            println!("[{}] FAT[{}]=0x{:08X} ({})", label, i, v, sym);
        }
    }
}

fn dump_minifat(data: &[u8], label: &str, count: usize) {
    let ss = sector_size(data);
    let first_mf = read_u32(data, 60) as usize;
    let mf_base = 512 + first_mf * ss;

    for i in 0..count {
        let off = mf_base + i * 4;
        if off + 4 > data.len() { break; }
        let v = read_u32(data, off);
        if v != 0xFFFFFFFF {
            let sym = match v {
                0xFFFFFFFE => "EOC",
                0xFFFFFFFF => "FREE",
                _ => "",
            };
            println!("[{}] MiniFAT[{}]={} ({})", label, i, v, sym);
        }
    }
}

fn create_ref_msi() -> Vec<u8> {
    use msi::{Column, Insert, Package, PackageType, Value};
    use std::io::{Cursor, Write};

    let buf: Vec<u8> = Vec::new();
    let cursor = Cursor::new(buf);
    let mut pkg = Package::create(PackageType::Installer, cursor).unwrap();

    pkg.summary_info_mut().set_title("Struct Test");
    pkg.summary_info_mut().set_author("Test");

    pkg.create_table("Property", vec![
        Column::build("Property").primary_key().string(72),
        Column::build("Value").string(1024),
    ]).unwrap();
    pkg.insert_rows(Insert::into("Property")
        .row(vec![Value::Str("ProductName".into()), Value::Str("Struct Test".into())])
        .row(vec![Value::Str("ProductVersion".into()), Value::Str("1.0.0".into())])
        .row(vec![Value::Str("Manufacturer".into()), Value::Str("Test".into())])
        .row(vec![Value::Str("ProductCode".into()), Value::Str("{BBBBBBBB-BBBB-BBBB-BBBB-BBBBBBBBBBBB}".into())])
        .row(vec![Value::Str("ProductLanguage".into()), Value::Str("1033".into())])
    ).unwrap();

    pkg.create_table("Directory", vec![
        Column::build("Directory").primary_key().string(72),
        Column::build("Directory_Parent").nullable().string(72),
        Column::build("DefaultDir").string(255),
    ]).unwrap();
    pkg.insert_rows(Insert::into("Directory")
        .row(vec![Value::Str("TARGETDIR".into()), Value::Null, Value::Str("SourceDir".into())])
        .row(vec![Value::Str("INSTALLDIR".into()), Value::Str("TARGETDIR".into()), Value::Str("STest:STest".into())])
    ).unwrap();

    pkg.create_table("Component", vec![
        Column::build("Component").primary_key().string(72),
        Column::build("ComponentId").nullable().string(38),
        Column::build("Directory_").string(72),
        Column::build("Attributes").int16(),
        Column::build("Condition").nullable().string(255),
        Column::build("KeyPath").nullable().string(72),
    ]).unwrap();
    pkg.insert_rows(Insert::into("Component")
        .row(vec![Value::Str("MainComp".into()), Value::Null, Value::Str("INSTALLDIR".into()),
                  Value::Int(0), Value::Null, Value::Null])
    ).unwrap();

    pkg.create_table("Feature", vec![
        Column::build("Feature").primary_key().string(38),
        Column::build("Feature_Parent").nullable().string(38),
        Column::build("Title").nullable().string(64),
        Column::build("Description").nullable().string(255),
        Column::build("Display").nullable().int16(),
        Column::build("Level").int16(),
        Column::build("Directory_").nullable().string(72),
        Column::build("Attributes").nullable().int16(),
    ]).unwrap();
    pkg.insert_rows(Insert::into("Feature")
        .row(vec![Value::Str("Complete".into()), Value::Null, Value::Str("Complete".into()),
                  Value::Str("Full".into()), Value::Null, Value::Int(1), Value::Null, Value::Null])
    ).unwrap();

    pkg.create_table("FeatureComponents", vec![
        Column::build("Feature_").primary_key().string(38),
        Column::build("Component_").primary_key().string(72),
    ]).unwrap();
    pkg.insert_rows(Insert::into("FeatureComponents")
        .row(vec![Value::Str("Complete".into()), Value::Str("MainComp".into())])
    ).unwrap();

    pkg.create_table("File", vec![
        Column::build("File").primary_key().string(72),
        Column::build("Component_").string(72),
        Column::build("FileName").string(255),
        Column::build("FileSize").int32(),
        Column::build("Version").nullable().string(72),
        Column::build("Language").nullable().string(20),
        Column::build("Attributes").nullable().int16(),
        Column::build("Sequence").int16(),
    ]).unwrap();
    let content = b"Hello!\r\n";
    pkg.insert_rows(Insert::into("File")
        .row(vec![Value::Str("hello.txt".into()), Value::Str("MainComp".into()),
                  Value::Str("hello.txt".into()), Value::Int(content.len() as i32),
                  Value::Null, Value::Null, Value::Null, Value::Int(1)])
    ).unwrap();

    pkg.create_table("Media", vec![
        Column::build("DiskId").primary_key().int16(),
        Column::build("LastSequence").int16(),
        Column::build("DiskPrompt").nullable().string(64),
        Column::build("Cabinet").nullable().string(255),
        Column::build("VolumeLabel").nullable().string(32),
        Column::build("Source").nullable().string(72),
    ]).unwrap();
    pkg.insert_rows(Insert::into("Media")
        .row(vec![Value::Int(1), Value::Int(1), Value::Null,
                  Value::Str("#vel.cab".into()), Value::Null, Value::Null])
    ).unwrap();

    let cab_data = velocity_msi::build_cabinet(&[velocity_msi::CabinetFile {
        name: "hello.txt".to_string(),
        data: content.to_vec(),
    }]);
    {
        let mut writer = pkg.write_stream("vel.cab").unwrap();
        writer.write_all(&cab_data).unwrap();
    }

    pkg.create_table("InstallExecuteSequence", vec![
        Column::build("Action").primary_key().string(72),
        Column::build("Condition").nullable().string(255),
        Column::build("Sequence").nullable().int16(),
    ]).unwrap();
    for (action, seq) in &[
        ("CostFinalize", 1000), ("CostInitialize", 800), ("FileCost", 900),
        ("InstallFiles", 4000), ("InstallFinalize", 6600),
        ("InstallInitialize", 1500), ("InstallValidate", 1400),
    ] {
        pkg.insert_rows(Insert::into("InstallExecuteSequence")
            .row(vec![Value::Str(action.to_string()), Value::Null, Value::Int(*seq)])
        ).unwrap();
    }

    pkg.flush().unwrap();
    let cursor = pkg.into_inner().unwrap();
    cursor.into_inner()
}

fn create_vel_msi() -> Vec<u8> {
    use velocity_msi::*;

    let mut builder = MsiBuilder::new();
    builder.set_title("Struct Test");
    builder.set_author("Test");
    builder.set_template("Intel", 1033);

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(1024).build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Struct Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Test")],
        vec![Value::from("ProductCode"), Value::from("{CCCCCCCC-CCCC-CCCC-CCCC-CCCCCCCCCCCC}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().category("Identifier").build(),
        Column::build("Directory_Parent").string(72).nullable().category("Identifier").build(),
        Column::build("DefaultDir").string(255).primary_key().category("DefaultDir").build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("STest:STest")],
    ]).unwrap();

    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().category("Identifier").build(),
        Column::build("ComponentId").string(38).nullable().category("GUID").build(),
        Column::build("Directory_").string(72).category("Identifier").build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().category("Condition").build(),
        Column::build("KeyPath").string(72).nullable().category("Identifier").build(),
    ]).unwrap();
    builder.insert_rows("Component", vec![
        vec![Value::from("MainComp"), Value::Null, Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::Null],
    ]).unwrap();

    builder.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().category("Identifier").build(),
        Column::build("Feature_Parent").string(38).nullable().category("Identifier").build(),
        Column::build("Title").string(64).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("Feature", vec![
        vec![Value::from("Complete"), Value::Null, Value::from("Complete"), Value::from("Full"), Value::Null, Value::Int(1), Value::Null, Value::Null],
    ]).unwrap();

    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().category("Identifier").build(),
        Column::build("Component_").string(72).primary_key().category("Identifier").build(),
    ]).unwrap();
    builder.insert_rows("FeatureComponents", vec![
        vec![Value::from("Complete"), Value::from("MainComp")],
    ]).unwrap();

    builder.create_table("File", vec![
        Column::build("File").string(72).primary_key().category("Identifier").build(),
        Column::build("Component_").string(72).category("Identifier").build(),
        Column::build("FileName").string(255).category("Filename").build(),
        Column::build("FileSize").int32().build(),
        Column::build("Version").string(72).nullable().category("Version").build(),
        Column::build("Language").string(20).nullable().category("Language").build(),
        Column::build("Attributes").int16().nullable().build(),
        Column::build("Sequence").int16().build(),
    ]).unwrap();
    let content = b"Hello!\r\n";
    builder.insert_rows("File", vec![
        vec![Value::from("hello.txt"), Value::from("MainComp"), Value::from("hello.txt"),
             Value::Int(content.len() as i32), Value::Null, Value::Null, Value::Null, Value::Int(1)],
    ]).unwrap();

    builder.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").string(20).build(),
        Column::build("Cabinet").string(255).nullable().category("Cabinet").build(),
        Column::build("VolumeLabel").string(32).nullable().build(),
        Column::build("DiskPrompt").string(64).nullable().build(),
    ]).unwrap();
    let cab_data = build_cabinet(&[CabinetFile {
        name: "hello.txt".to_string(),
        data: content.to_vec(),
    }]);
    builder.insert_rows("Media", vec![
        vec![Value::Int(1), Value::from("1"), Value::from("#vel.cab"), Value::Null, Value::Null],
    ]).unwrap();
    builder.add_stream("vel.cab".to_string(), cab_data);

    builder.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("FileCost"), Value::Null, Value::Int(900)],
        vec![Value::from("InstallFiles"), Value::Null, Value::Int(4000)],
        vec![Value::from("InstallFinalize"), Value::Null, Value::Int(6600)],
        vec![Value::from("InstallInitialize"), Value::Null, Value::Int(1500)],
        vec![Value::from("InstallValidate"), Value::Null, Value::Int(1400)],
    ]).unwrap();

    builder.build().unwrap()
}
