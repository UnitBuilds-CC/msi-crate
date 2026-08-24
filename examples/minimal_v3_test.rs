/// Minimal V3 CFB test: just create an empty V3 CFB with MSI CLSID
/// and test if msiexec accepts the structure.
/// cargo run --example minimal_v3_test -p velocity-msi
use std::io::{Cursor, Write};

fn main() {
    println!("=== MINIMAL V3 CFB TEST ===\n");

    // Test 1: Empty V3 CFB with MSI CLSID
    let path1 = "C:\\temp\\minimal_v3.msi";
    let _ = std::fs::remove_file(path1);
    {
        let mut buf = Vec::new();
        {
            let cursor = Cursor::new(&mut buf);
            let mut comp = cfb::CompoundFile::create_with_version(
                cfb::Version::V3, cursor
            ).unwrap();
            let msi_clsid = uuid::Uuid::from_bytes([
                0x00, 0x0C, 0x10, 0x84, 0x00, 0x00, 0x00, 0x00,
                0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
            ]);
            comp.set_storage_clsid("", msi_clsid).unwrap();
            // Write a minimal SummaryInfo stream
            let mut s = comp.create_stream("\u{0005}SummaryInformation").unwrap();
            // Minimal property set: BOM + version + OS + CLSID + count + FMTID + offset + section
            let summary_data = create_minimal_summary();
            s.write_all(&summary_data).unwrap();
            comp.flush().unwrap();
        }
        std::fs::write(path1, &buf).unwrap();
        println!("Test 1: Empty V3 CFB with CLSID: {} bytes", buf.len());
    }

    let output = std::process::Command::new("msiexec")
        .args(&["/i", path1, "/qn"])
        .output().unwrap();
    let ec = output.status.code().unwrap_or(-1);
    println!("msiexec exit code: {} ({})\n", ec, match ec {
        0 => "SUCCESS",
        1613 => "invalid package",
        1620 => "could not open",
        _ => "other",
    });

    // Test 2: Check velocity-msi output CLSID
    let velocity_data = create_velocity_msi();
    let v_path = "C:\\temp\\v_msi_test.msi";
    std::fs::write(v_path, &velocity_data).unwrap();
    
    let mut comp = cfb::CompoundFile::open(Cursor::new(&velocity_data)).unwrap();
    let root = comp.root_entry();
    println!("velocity-msi root CLSID: {}", root.clsid());
    println!("velocity-msi root name: {:?}", root.name());
    println!("velocity-msi root type: {:?}", entry_type_str(root.entry_type()));
    
    let expected = uuid::Uuid::from_bytes([
        0x00, 0x0C, 0x10, 0x84, 0x00, 0x00, 0x00, 0x00,
        0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
    ]);
    println!("Expected MSI CLSID: {}", expected);
    println!("CLSID match: {}", root.clsid() == &expected);

    // Test 3: Check what the msi crate sets as CLSID
    let msi_data = create_msi_crate_msi();
    let mut comp2 = cfb::CompoundFile::open(Cursor::new(&msi_data)).unwrap();
    let root2 = comp2.root_entry();
    println!("\nmsi crate root CLSID: {}", root2.clsid());
    println!("msi crate CLSID match: {}", root2.clsid() == &expected);

    // Test 4: Manually check CLSID bytes in the file
    println!("\n--- Raw CLSID bytes ---");
    // In CFB, root entry is at offset 1024 (first directory sector)
    // CLSID is at offset 1104 (1024 + 80 bytes into directory entry)
    // Actually, the root entry location depends on the CFB structure
    // Let me find it by scanning for the CLSID pattern
    println!("velocity-msi bytes at 1104: {:?}", &velocity_data[1104..1120]);
    println!("msi crate bytes at 1104: {:?}", &msi_data[1104..1120]);
    
    // Check CFB header for first directory sector SECID
    let first_dir_sec = u32::from_le_bytes([
        velocity_data[48], velocity_data[49], velocity_data[50], velocity_data[51]
    ]);
    let sector_size = 1 << velocity_data[30];
    let dir_offset = (first_dir_sec as usize + 1) * sector_size;
    println!("\nvelocity-msi: first dir sector={}, sector_size={}, dir_offset={}", 
             first_dir_sec, sector_size, dir_offset);
    if dir_offset + 80 + 16 <= velocity_data.len() {
        let clsid_offset = dir_offset + 80;
        println!("CLSID at offset {}: {:?}", clsid_offset, &velocity_data[clsid_offset..clsid_offset+16]);
    }

    println!("\n=== DONE ===");
}

fn entry_type_str(t: cfb::EntryType) -> &'static str {
    match t {
        cfb::EntryType::Storage => "Storage",
        cfb::EntryType::Stream => "Stream",
        cfb::EntryType::Root => "Root",
        _ => "Unknown",
    }
}

fn create_minimal_summary() -> Vec<u8> {
    let mut buf = Vec::new();
    // Property Set Header (48 bytes)
    buf.extend_from_slice(&0xFFFEu16.to_le_bytes()); // BOM
    buf.extend_from_slice(&0x0000u16.to_le_bytes()); // Version
    buf.extend_from_slice(&[10u8, 0, 2, 0]); // OS version 10.0, Win32
    buf.extend_from_slice(&[0u8; 16]); // CLSID (zeros)
    buf.extend_from_slice(&1u32.to_le_bytes()); // 1 section
    // FMTID: {F29F85E0-4FF9-1068-AB91-08002B27B3D9}
    buf.extend_from_slice(b"\xe0\x85\x9f\xf2\xf9\x4f\x68\x10\xab\x91\x08\x00\x2b\x27\xb3\xd9");
    buf.extend_from_slice(&48u32.to_le_bytes()); // Section offset = 48

    // Section: just codepage (PID 1, VT_I2, value 1252)
    let section_size: u32 = 8 + 8; // header + 1 property index entry
    let prop_data_size: u32 = 8; // VT_I2 + value + padding
    let total_section = section_size + prop_data_size;
    
    buf.extend_from_slice(&total_section.to_le_bytes()); // Section size
    buf.extend_from_slice(&1u32.to_le_bytes()); // 1 property
    buf.extend_from_slice(&1u32.to_le_bytes()); // PID 1
    buf.extend_from_slice(&(section_size).to_le_bytes()); // Offset to property data
    // Property data: VT_I2 = 2
    buf.extend_from_slice(&2u32.to_le_bytes()); // VT_I2
    buf.extend_from_slice(&1252i16.to_le_bytes()); // value
    buf.extend_from_slice(&[0u8; 2]); // padding

    buf
}

fn create_velocity_msi() -> Vec<u8> {
    use velocity_msi::{Column, MsiBuilder, Value};
    let mut builder = MsiBuilder::new();
    builder.set_title("Test");
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Test")],
    ]).unwrap();
    builder.build().unwrap()
}

fn create_msi_crate_msi() -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut pkg = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
    {
        let si = pkg.summary_info_mut();
        si.set_codepage(msi::CodePage::Windows1252);
        si.set_uuid(uuid::Uuid::from_bytes([0; 16]));
    }
    pkg.set_database_codepage(msi::CodePage::Windows1252);
    pkg.create_table("Property", vec![
        msi::Column::build("Property").primary_key().id_string(72),
        msi::Column::build("Value").nullable().formatted_string(255),
    ]).unwrap();
    pkg.insert_rows(msi::Insert::into("Property")
        .row(vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Test".into())])
    ).unwrap();
    pkg.flush().unwrap();
    let cursor = pkg.into_inner().unwrap();
    cursor.into_inner()
}
