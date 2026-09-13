/// Diagnostic: write velocity-msi streams using the cfb crate instead of our
/// custom OLE writer. If the cfb-written MSI works with msiexec but our
/// ole.rs-written MSI doesn't, the bug is in src/ole.rs.
use std::io::{Cursor, Write};

fn main() {
    let velocity_msi = build_velocity_msi();
    std::fs::write("velocity_ole.msi", &velocity_msi).unwrap();
    println!("velocity_ole.msi: {} bytes (custom OLE writer)", velocity_msi.len());

    let cfb_msi = build_cfb_msi();
    std::fs::write("cfb_msi.msi", &cfb_msi).unwrap();
    println!("cfb_msi.msi: {} bytes (cfb crate writer)", cfb_msi.len());

    // Compare streams from both
    println!("\n=== Stream comparison ===");
    let vel_streams = extract_streams(&velocity_msi);
    let cfb_streams = extract_streams(&cfb_msi);

    for name in vel_streams.keys() {
        if let Some(cfb_data) = cfb_streams.get(name) {
            let vel_data = &vel_streams[name];
            if vel_data == cfb_data {
                println!("MATCH: {} ({} bytes)", name, vel_data.len());
            } else {
                println!("DIFF: {} (vel={} bytes, cfb={} bytes)", name, vel_data.len(), cfb_data.len());
            }
        } else {
            println!("ONLY IN VELOCITY: {} ({} bytes)", name, vel_streams[name].len());
        }
    }
    for name in cfb_streams.keys() {
        if !vel_streams.contains_key(name) {
            println!("ONLY IN CFB: {} ({} bytes)", name, cfb_streams[name].len());
        }
    }

    // Test both with msiexec
    println!("\n=== msiexec test ===");
    for (name, file) in &[("velocity_ole", "velocity_ole.msi"), ("cfb", "cfb_msi.msi")] {
        let log = format!("{}_log.txt", name);
        let _ = std::fs::remove_file(&log);
        let status = std::process::Command::new("msiexec")
            .args(["/i", file, "/qn", "/l*v", &log])
            .status();
        match status {
            Ok(s) => println!("{}: exit code {}", name, s.code().unwrap_or(-1)),
            Err(e) => println!("{}: failed to start: {}", name, e),
        }
    }
}

fn build_velocity_msi() -> Vec<u8> {
    use velocity_msi::*;

    let mut builder = MsiBuilder::new();
    builder.set_title("CFB Test");
    builder.set_author("Test");
    builder.set_template("Intel", 1033);

    setup_tables(&mut builder);
    builder.build().unwrap()
}

fn build_cfb_msi() -> Vec<u8> {
    use velocity_msi::*;

    let mut builder = MsiBuilder::new();
    builder.set_title("CFB Test");
    builder.set_author("Test");
    builder.set_template("Intel", 1033);

    setup_tables(&mut builder);

    // Instead of builder.build(), manually extract streams and write with cfb crate
    let msi_data = builder.build().unwrap();

    // Extract all streams from the velocity MSI
    let streams = extract_streams(&msi_data);

    // Write them into a new OLE file using the cfb crate
    let buf: Vec<u8> = Vec::new();
    let cursor = Cursor::new(buf);
    let mut comp = cfb::CompoundFile::create(cursor).unwrap();

    for (name, data) in &streams {
        // Skip the root entry
        if name == "/" { continue; }
        let path = name.trim_start_matches('/');
        if path.is_empty() { continue; }

        let mut stream = comp.create_stream(path).unwrap();
        stream.write_all(data).unwrap();
    }

    let cursor = comp.into_inner();
    let mut result = cursor.into_inner();

    // Patch the CLSID in the root directory entry to MSI CLSID
    // The root entry is the first directory entry (128 bytes)
    // Find the directory sector and patch offset 80-96 in the root entry
    patch_msi_clsid(&mut result);

    result
}

fn patch_msi_clsid(data: &mut [u8]) {
    // MSI CLSID: {000C1084-0000-0000-C000-000000000046}
    let msi_clsid: [u8; 16] = [
        0x84, 0x10, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00,
        0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
    ];

    // Find the root entry in the directory. The root entry has obj_type=5 at offset 66.
    // Directory starts after the header (512 bytes) + FAT sectors.
    // For simplicity, scan for the root entry signature.
    // The first directory sector is at the offset specified in the header at offset 48.
    let first_dir_sector = u32::from_le_bytes([data[48], data[49], data[50], data[51]]) as usize;
    let dir_offset = 512 + first_dir_sector * 512;

    // Root entry is the first entry in the directory sector
    // CLSID is at offset 80 within the directory entry
    let clsid_offset = dir_offset + 80;
    if clsid_offset + 16 <= data.len() {
        data[clsid_offset..clsid_offset + 16].copy_from_slice(&msi_clsid);
    }
}

fn setup_tables(builder: &mut velocity_msi::MsiBuilder) {
    use velocity_msi::*;

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(1024).build(),
    ]).unwrap();

    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("CFB Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Test Corp")],
        vec![Value::from("ProductCode"), Value::from("{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().category("Identifier").build(),
        Column::build("Directory_Parent").string(72).nullable().category("Identifier").build(),
        Column::build("DefaultDir").string(255).primary_key().category("DefaultDir").build(),
    ]).unwrap();

    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("CFBTest:CFBTest")],
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
        vec![Value::from("Complete"), Value::Null, Value::from("Complete"), Value::from("Full install"), Value::Null, Value::Int(1), Value::Null, Value::Null],
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

    let content = b"Hello from CFB test!\r\n";
    builder.insert_rows("File", vec![
        vec![
            Value::from("hello.txt"), Value::from("MainComp"), Value::from("hello.txt"),
            Value::Int(content.len() as i32),
            Value::Null, Value::Null, Value::Null, Value::Int(1),
        ],
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
}

fn extract_streams(msi_data: &[u8]) -> std::collections::BTreeMap<String, Vec<u8>> {
    use std::io::Read;
    let mut result = std::collections::BTreeMap::new();
    let cursor = Cursor::new(msi_data);
    let mut comp = cfb::CompoundFile::open(cursor).unwrap();
    let paths: Vec<String> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();
    for path in &paths {
        let mut stream = comp.open_stream(path).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        result.insert(path.clone(), data);
    }
    result
}
