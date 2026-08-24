/// Progressive MSI test: start with empty V3 CFB, add streams one by one,
/// test with msiexec at each step to find what triggers errors.
/// cargo run --example progressive_msi_test -p velocity-msi
use std::io::{Cursor, Write};

fn main() {
    println!("=== PROGRESSIVE MSI TEST ===\n");
    let _ = std::fs::create_dir_all("C:\\temp");

    // Step 1: Empty V3 CFB with MSI CLSID
    test_step("step1_empty_v3", |comp| {
        let msi_clsid = uuid::Uuid::from_bytes([
            0x00, 0x0C, 0x10, 0x84, 0x00, 0x00, 0x00, 0x00,
            0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
        ]);
        comp.set_storage_clsid("", msi_clsid).unwrap();
    });

    // Step 2: V3 + SummaryInfo (minimal: codepage + security + wordcount)
    test_step("step2_summary", |comp| {
        let msi_clsid = uuid::Uuid::from_bytes([
            0x00, 0x0C, 0x10, 0x84, 0x00, 0x00, 0x00, 0x00,
            0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
        ]);
        comp.set_storage_clsid("", msi_clsid).unwrap();
        let si_data = make_summary_info();
        let mut s = comp.create_stream("\u{0005}SummaryInformation").unwrap();
        s.write_all(&si_data).unwrap();
    });

    // Step 3: V3 + SummaryInfo + use velocity-msi to build a complete MSI
    // and compare its streams
    {
        println!("\n--- Step 3: velocity-msi complete build ---");
        let msi_data = build_velocity_msi();
        let path = "C:\\temp\\step3_velocity.msi";
        std::fs::write(path, &msi_data).unwrap();
        test_msiexec(path);

        // Now read it back and examine the CLSID
        let mut comp = cfb::CompoundFile::open(Cursor::new(&msi_data)).unwrap();
        let root = comp.root_entry();
        println!("  Root CLSID: {}", root.clsid());
        println!("  Root name: {:?}", root.name());

        let expected = uuid::Uuid::from_bytes([
            0x00, 0x0C, 0x10, 0x84, 0x00, 0x00, 0x00, 0x00,
            0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
        ]);
        println!("  Expected CLSID: {}", expected);
        println!("  CLSID match: {}", root.clsid() == &expected);

        // Check raw bytes at the CLSID position in the root directory entry
        let version = msi_data[26];
        let sector_shift = u16::from_le_bytes([msi_data[30], msi_data[31]]);
        let sector_size = 1usize << sector_shift;
        let first_dir_sec = u32::from_le_bytes([msi_data[48], msi_data[49], msi_data[50], msi_data[51]]);
        let dir_offset = 512 + first_dir_sec as usize * sector_size; // V3 header is 512 bytes
        println!("  V{}, sector_size={}, first_dir_sec={}, dir_offset={}", version, sector_size, first_dir_sec, dir_offset);

        // CLSID is at offset 80 within directory entry
        if dir_offset + 96 <= msi_data.len() {
            let clsid_bytes = &msi_data[dir_offset + 80..dir_offset + 96];
            println!("  CLSID bytes: {:02x?}", clsid_bytes);
            let is_zero = clsid_bytes.iter().all(|&b| b == 0);
            println!("  CLSID is zeros: {}", is_zero);
        }
    }

    // Step 4: Use msi crate to create MSI, read its CLSID
    {
        println!("\n--- Step 4: msi crate reference ---");
        let msi_data = build_msi_crate_msi();
        println!("  msi crate V{}", msi_data[26]);

        let mut comp = cfb::CompoundFile::open(Cursor::new(&msi_data)).unwrap();
        let root = comp.root_entry();
        println!("  Root CLSID: {}", root.clsid());

        let version = msi_data[26];
        let sector_shift = u16::from_le_bytes([msi_data[30], msi_data[31]]);
        let sector_size = 1usize << sector_shift;
        let first_dir_sec = u32::from_le_bytes([msi_data[48], msi_data[49], msi_data[50], msi_data[51]]);
        let header_size = if version == 3 { 512 } else { 4096 };
        let dir_offset = header_size + first_dir_sec as usize * sector_size;
        if dir_offset + 96 <= msi_data.len() {
            let clsid_bytes = &msi_data[dir_offset + 80..dir_offset + 96];
            println!("  CLSID bytes: {:02x?}", clsid_bytes);
        }

        // List all streams from msi crate
        let stream_names: Vec<String> = comp.walk()
            .filter(|e| e.is_stream())
            .map(|e| e.name().to_string())
            .collect();
        println!("  Streams ({}):", stream_names.len());
        for name in &stream_names {
            let mut s = comp.open_stream(name).unwrap();
            let mut data = Vec::new();
            std::io::Read::read_to_end(&mut s, &mut data).unwrap();
            // Show name as hex for non-printable chars
            let hex_name: String = name.encode_utf16()
                .map(|c| format!("{:04X}", c))
                .collect::<Vec<_>>()
                .join(" ");
            println!("    [{}] {} ({} bytes)", hex_name, name.chars().map(|c| if c.is_ascii_graphic() || c == ' ' { c } else { '?' }).collect::<String>(), data.len());
        }
    }

    // Step 5: Build velocity-msi and list streams with hex names
    {
        println!("\n--- Step 5: velocity-msi streams ---");
        let msi_data = build_velocity_msi();
        let mut comp = cfb::CompoundFile::open(Cursor::new(&msi_data)).unwrap();
        let stream_names: Vec<String> = comp.walk()
            .filter(|e| e.is_stream())
            .map(|e| e.name().to_string())
            .collect();
        println!("  Streams ({}):", stream_names.len());
        for name in &stream_names {
            let mut s = comp.open_stream(name).unwrap();
            let mut data = Vec::new();
            std::io::Read::read_to_end(&mut s, &mut data).unwrap();
            let hex_name: String = name.encode_utf16()
                .map(|c| format!("{:04X}", c))
                .collect::<Vec<_>>()
                .join(" ");
            println!("    [{}] {} ({} bytes)", hex_name, name.chars().map(|c| if c.is_ascii_graphic() || c == ' ' { c } else { '?' }).collect::<String>(), data.len());
        }
    }

    println!("\n=== DONE ===");
}

fn test_step(name: &str, setup: impl Fn(&mut cfb::CompoundFile<Cursor<&mut Vec<u8>>>)) {
    let path = format!("C:\\temp\\{}.msi", name);
    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut comp = cfb::CompoundFile::create_with_version(
            cfb::Version::V3, cursor,
        ).unwrap();
        setup(&mut comp);
        comp.flush().unwrap();
    }
    std::fs::write(&path, &buf).unwrap();
    println!("--- {} ({} bytes, V{}) ---", name, buf.len(), buf[26]);
    test_msiexec(&path);
}

fn test_msiexec(path: &str) {
    let output = std::process::Command::new("msiexec")
        .args(&["/i", path, "/qn"])
        .output().unwrap();
    let ec = output.status.code().unwrap_or(-1);
    let desc = match ec {
        0 => "SUCCESS",
        1613 => "invalid package",
        1619 => "not valid",
        1620 => "could not open",
        _ => "other",
    };
    println!("  msiexec: {} ({})", ec, desc);
}

fn make_summary_info() -> Vec<u8> {
    let mut buf = Vec::new();
    // Property Set Header
    buf.extend_from_slice(&0xFFFEu16.to_le_bytes()); // BOM
    buf.extend_from_slice(&0x0000u16.to_le_bytes()); // Version
    buf.extend_from_slice(&[10u8, 0, 2, 0]); // OS 10.0, Win32
    buf.extend_from_slice(&[0u8; 16]); // CLSID
    buf.extend_from_slice(&1u32.to_le_bytes()); // 1 section
    buf.extend_from_slice(b"\xe0\x85\x9f\xf2\xf9\x4f\x68\x10\xab\x91\x08\x00\x2b\x27\xb3\xd9");
    buf.extend_from_slice(&48u32.to_le_bytes()); // offset

    // Section: codepage(1), security(14), wordcount(15) = 3 props
    let section_header_size = 8u32; // size + count
    let index_size = 3 * 8; // 3 props × 8 bytes each
    let prop_data_start = section_header_size + index_size;

    // Calculate property data sizes
    // PID 1: VT_I2 (8 bytes)
    // PID 14: VT_I4 (8 bytes)
    // PID 15: VT_I4 (8 bytes)
    let prop1_size = 8u32;
    let prop14_size = 8u32;
    let prop15_size = 8u32;

    let section_size = prop_data_start + prop1_size + prop14_size + prop15_size;

    buf.extend_from_slice(&section_size.to_le_bytes());
    buf.extend_from_slice(&3u32.to_le_bytes()); // 3 properties

    // Property index
    let off1 = prop_data_start;
    let off14 = off1 + prop1_size;
    let off15 = off14 + prop14_size;

    buf.extend_from_slice(&1u32.to_le_bytes()); // PID 1
    buf.extend_from_slice(&off1.to_le_bytes());
    buf.extend_from_slice(&14u32.to_le_bytes()); // PID 14
    buf.extend_from_slice(&off14.to_le_bytes());
    buf.extend_from_slice(&15u32.to_le_bytes()); // PID 15
    buf.extend_from_slice(&off15.to_le_bytes());

    // PID 1: Codepage (VT_I2 = 2, value = 1252)
    buf.extend_from_slice(&2u32.to_le_bytes()); // VT_I2
    buf.extend_from_slice(&1252i16.to_le_bytes());
    buf.extend_from_slice(&[0u8; 2]); // pad

    // PID 14: Security (VT_I4 = 3, value = 405)
    buf.extend_from_slice(&3u32.to_le_bytes()); // VT_I4
    buf.extend_from_slice(&405i32.to_le_bytes());

    // PID 15: WordCount (VT_I4 = 3, value = 2)
    buf.extend_from_slice(&3u32.to_le_bytes()); // VT_I4
    buf.extend_from_slice(&2i32.to_le_bytes());

    buf
}

fn build_velocity_msi() -> Vec<u8> {
    use velocity_msi::{Column, MsiBuilder, Value};

    let mut builder = MsiBuilder::new();
    builder.set_title("Test");
    builder.set_author("Test");
    builder.set_template("Intel", 1033);

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0")],
        vec![Value::from("Manufacturer"), Value::from("Test")],
        vec![Value::from("ProductCode"), Value::from("{12345678-1234-4234-8234-123456789012}")],
        vec![Value::from("UpgradeCode"), Value::from("{22345678-1234-4234-8234-123456789012}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").nullable().string(72).build(),
        Column::build("DefaultDir").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("ProgramFilesFolder"), Value::from("TARGETDIR"), Value::from("PFiles")],
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFilesFolder"), Value::from("Test")],
    ]).unwrap();

    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").nullable().string(38).build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").nullable().string(255).build(),
        Column::build("KeyPath").nullable().string(72).build(),
    ]).unwrap();
    builder.insert_rows("Component", vec![
        vec![Value::from("C1"), Value::Null, Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::Null],
    ]).unwrap();

    builder.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").nullable().string(38).build(),
        Column::build("Title").nullable().string(64).localizable().build(),
        Column::build("Description").nullable().string(255).localizable().build(),
        Column::build("Display").nullable().int16().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").nullable().string(72).build(),
        Column::build("Attributes").int16().build(),
    ]).unwrap();
    builder.insert_rows("Feature", vec![
        vec![Value::from("F1"), Value::Null, Value::Null, Value::Null, Value::Null, Value::Int(1), Value::Null, Value::Int(0)],
    ]).unwrap();

    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("FeatureComponents", vec![
        vec![Value::from("F1"), Value::from("C1")],
    ]).unwrap();

    builder.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").nullable().string(255).build(),
        Column::build("Sequence").nullable().int16().build(),
    ]).unwrap();
    builder.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("InstallValidate"), Value::Null, Value::Int(1400)],
        vec![Value::from("InstallInitialize"), Value::Null, Value::Int(1500)],
        vec![Value::from("InstallFinalize"), Value::Null, Value::Int(6600)],
    ]).unwrap();

    builder.create_table("InstallUISequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").nullable().string(255).build(),
        Column::build("Sequence").nullable().int16().build(),
    ]).unwrap();
    builder.insert_rows("InstallUISequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("ExecuteAction"), Value::Null, Value::Int(1300)],
    ]).unwrap();

    builder.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int16().build(),
        Column::build("DiskPrompt").nullable().string(64).localizable().build(),
        Column::build("Cabinet").nullable().string(255).build(),
        Column::build("VolumeLabel").nullable().string(32).localizable().build(),
        Column::build("Source").nullable().string(72).build(),
    ]).unwrap();
    builder.insert_rows("Media", vec![
        vec![Value::Int(1), Value::Int(0), Value::Null, Value::Null, Value::Null, Value::Null],
    ]).unwrap();

    builder.build().unwrap()
}

fn build_msi_crate_msi() -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut pkg = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
    {
        let si = pkg.summary_info_mut();
        si.set_title("Test");
        si.set_author("Test");
        si.set_codepage(msi::CodePage::Windows1252);
        si.set_arch("Intel");
        si.set_languages(&[msi::Language::from_code(1033)]);
        si.set_word_count(2);
        si.set_uuid(uuid::Uuid::nil());
    }
    pkg.set_database_codepage(msi::CodePage::Windows1252);

    pkg.create_table("Property", vec![
        msi::Column::build("Property").primary_key().id_string(72),
        msi::Column::build("Value").nullable().localizable().formatted_string(255),
    ]).unwrap();
    pkg.insert_rows(msi::Insert::into("Property")
        .row(vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Test".into())])
        .row(vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0".into())])
        .row(vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Test".into())])
    ).unwrap();

    pkg.flush().unwrap();
    pkg.into_inner().unwrap().into_inner()
}
