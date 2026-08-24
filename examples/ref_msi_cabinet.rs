/// Use msi crate to create a reference MSI with embedded cabinet
/// Then check the stream names to see how cabinet streams are encoded.
use std::io::{Read, Write, Cursor};

fn main() {
    // Create a reference MSI using the msi crate
    let mut pkg = msi::Package::create(msi::Platform::X64, 1033).unwrap();
    
    // Set summary info
    {
        let si = pkg.summary_info_mut();
        si.set_title(Some("Ref Test")).unwrap();
        si.set_author(Some("Ref Author")).unwrap();
        si.set_template(Some("x64;1033")).unwrap();
        si.set_rev_number(Some("{ABCDEF01-2345-6789-ABCD-EF0123456789}")).unwrap();
    }

    // Create Property table
    let prop_cols = vec![
        msi::Column::build("Property").primary_key().string(72).build(),
        msi::Column::build("Value").nullable().string(255).build(),
    ];
    pkg.create_table("Property", prop_cols).unwrap();
    
    let product_code = "{12345678-1234-1234-1234-123456789ABC}";
    let upgrade_code = "{87654321-4321-4321-4321-CBA987654321}";
    
    pkg.insert_rows(msi::Insert::Into("Property"), vec![
        vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Ref Test".into())],
        vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())],
        vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Ref Corp".into())],
        vec![msi::Value::Str("ProductCode".into()), msi::Value::Str(product_code.into())],
        vec![msi::Value::Str("UpgradeCode".into()), msi::Value::Str(upgrade_code.into())],
        vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())],
    ]).unwrap();

    // Create Directory table
    let dir_cols = vec![
        msi::Column::build("Directory").primary_key().string(72).build(),
        msi::Column::build("Directory_Parent").nullable().string(72).build(),
        msi::Column::build("DefaultDir").primary_key().string(255).build(),
    ];
    pkg.create_table("Directory", dir_cols).unwrap();
    pkg.insert_rows(msi::Insert::Into("Directory"), vec![
        vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str("SourceDir".into())],
        vec![msi::Value::Str("INSTALLDIR".into()), msi::Value::Str("TARGETDIR".into()), msi::Value::Str("RefTest:.".into())],
    ]).unwrap();

    // Create Component table
    let comp_cols = vec![
        msi::Column::build("Component").primary_key().string(72).build(),
        msi::Column::build("ComponentId").nullable().string(38).build(),
        msi::Column::build("Directory_").string(72).build(),
        msi::Column::build("Attributes").int16().build(),
        msi::Column::build("Condition").nullable().string(255).build(),
        msi::Column::build("KeyPath").nullable().string(72).build(),
    ];
    pkg.create_table("Component", comp_cols).unwrap();
    pkg.insert_rows(msi::Insert::Into("Component"), vec![
        vec![
            msi::Value::Str("Comp1".into()),
            msi::Value::Null,
            msi::Value::Str("INSTALLDIR".into()),
            msi::Value::Int(0),
            msi::Value::Null,
            msi::Value::Str("hello.txt".into()),
        ],
    ]).unwrap();

    // Create File table
    let file_cols = vec![
        msi::Column::build("File").primary_key().string(72).build(),
        msi::Column::build("Component_").string(72).build(),
        msi::Column::build("FileName").string(255).build(),
        msi::Column::build("FileSize").int32().build(),
        msi::Column::build("Sequence").int16().build(),
    ];
    pkg.create_table("File", file_cols).unwrap();
    let content = b"Hello World\r\n";
    pkg.insert_rows(msi::Insert::Into("File"), vec![
        vec![
            msi::Value::Str("hello.txt".into()),
            msi::Value::Str("Comp1".into()),
            msi::Value::Str("hello.txt".into()),
            msi::Value::Int(content.len() as i32),
            msi::Value::Int(1),
        ],
    ]).unwrap();

    // Create Feature table
    let feat_cols = vec![
        msi::Column::build("Feature").primary_key().string(38).build(),
        msi::Column::build("Feature_Parent").nullable().string(38).build(),
        msi::Column::build("Title").nullable().string(64).build(),
        msi::Column::build("Description").nullable().string(255).build(),
        msi::Column::build("Display").nullable().int16().build(),
        msi::Column::build("Level").int16().build(),
        msi::Column::build("Directory_").nullable().string(72).build(),
        msi::Column::build("Attributes").nullable().int16().build(),
    ];
    pkg.create_table("Feature", feat_cols).unwrap();
    pkg.insert_rows(msi::Insert::Into("Feature"), vec![
        vec![
            msi::Value::Str("Feat1".into()),
            msi::Value::Null,
            msi::Value::Str("Main".into()),
            msi::Value::Str("Main feature".into()),
            msi::Value::Int(1),
            msi::Value::Int(1),
            msi::Value::Str("INSTALLDIR".into()),
            msi::Value::Null,
        ],
    ]).unwrap();

    // Create FeatureComponents table
    let fc_cols = vec![
        msi::Column::build("Feature_").primary_key().string(38).build(),
        msi::Column::build("Component_").primary_key().string(72).build(),
    ];
    pkg.create_table("FeatureComponents", fc_cols).unwrap();
    pkg.insert_rows(msi::Insert::Into("FeatureComponents"), vec![
        vec![
            msi::Value::Str("Feat1".into()),
            msi::Value::Str("Comp1".into()),
        ],
    ]).unwrap();

    // Create Media table
    let media_cols = vec![
        msi::Column::build("DiskId").primary_key().int16().build(),
        msi::Column::build("LastSequence").int16().build(),
        msi::Column::build("DiskPrompt").nullable().string(64).build(),
        msi::Column::build("Cabinet").nullable().string(255).build(),
        msi::Column::build("VolumeLabel").nullable().string(32).build(),
        msi::Column::build("Source").nullable().string(72).build(),
    ];
    pkg.create_table("Media", media_cols).unwrap();
    pkg.insert_rows(msi::Insert::Into("Media"), vec![
        vec![
            msi::Value::Int(1),
            msi::Value::Int(1),
            msi::Value::Null,
            msi::Value::Str("#vel.cab".into()),
            msi::Value::Null,
            msi::Value::Null,
        ],
    ]).unwrap();

    // Create cabinet data (simple MSZIP)
    let cab_data = build_simple_cabinet(content);
    
    // Add cabinet stream
    pkg.insert_stream("vel.cab", Cursor::new(cab_data)).unwrap();

    // Write to file
    let mut buf = Vec::new();
    pkg.write(&mut buf).unwrap();
    std::fs::write("ref_install.msi", &buf).unwrap();
    println!("Reference MSI: {} bytes", buf.len());

    // Test with msiexec
    let output = std::process::Command::new("msiexec")
        .args(&["/i", "ref_install.msi", "/qn", "/l*v", "ref_install_log.txt"])
        .output().unwrap();
    println!("msiexec exit: {}", output.status.code().unwrap_or(-1));

    // List streams
    println!("\n=== Reference MSI streams ===");
    let mut comp = cfb::CompoundFile::open(Cursor::new(&buf)).unwrap();
    let stream_paths: Vec<_> = comp.walk()
        .map(|e| (e.path().to_string_lossy().to_string(), e.is_stream()))
        .collect();
    for (path, is_stream) in &stream_paths {
        println!("  {} {}", if *is_stream { "S" } else { "D" }, path);
    }

    // Read log
    if let Ok(log) = std::fs::read_to_string("ref_install_log.txt") {
        println!("\n=== Log (key lines) ===");
        for line in log.lines() {
            if line.contains("returning") || line.contains("Note: 1:") 
                || (line.contains("Error") && !line.contains("Error 0"))
                || line.contains("cabinet") || line.contains("Cabinet") {
                println!("  {}", line.trim());
            }
        }
    }

    // Cleanup
    let _ = std::fs::remove_file("ref_install.msi");
    let _ = std::fs::remove_file("ref_install_log.txt");
}

fn build_simple_cabinet(content: &[u8]) -> Vec<u8> {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    
    let mut buf = Vec::new();
    
    // Compress
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(content).unwrap();
    let compressed = encoder.finish().unwrap();
    
    let cb_data = (2 + compressed.len()) as u16; // "CK" + compressed
    let cb_uncomp = content.len() as u16;
    let header_size: u32 = 36;
    let folder_size: u32 = 8;
    let data_offset = header_size + folder_size;
    let data_block_size = 8 + cb_data as u32;
    let file_table_offset = data_offset + data_block_size;
    let file_entry_size = 16 + 10; // hello.txt\0 = 10 bytes
    let total_size = file_table_offset + file_entry_size;
    
    // CFHEADER
    buf.write_all(b"MSCF").unwrap();
    buf.write_all(&0u32.to_le_bytes()).unwrap();
    buf.write_all(&total_size.to_le_bytes()).unwrap();
    buf.write_all(&0u32.to_le_bytes()).unwrap();
    buf.write_all(&file_table_offset.to_le_bytes()).unwrap();
    buf.write_all(&0u32.to_le_bytes()).unwrap();
    buf.push(3); buf.push(1);
    buf.write_all(&1u16.to_le_bytes()).unwrap(); // cFolders
    buf.write_all(&1u16.to_le_bytes()).unwrap(); // cFiles
    buf.write_all(&0u16.to_le_bytes()).unwrap(); // flags
    buf.write_all(&1u16.to_le_bytes()).unwrap(); // setID
    buf.write_all(&0u16.to_le_bytes()).unwrap(); // iCabinet
    
    // CFOLDER
    buf.write_all(&data_offset.to_le_bytes()).unwrap();
    buf.write_all(&cb_data.to_le_bytes()).unwrap();
    buf.write_all(&1u16.to_le_bytes()).unwrap(); // MSZIP
    
    // CFDATA
    buf.write_all(&0u32.to_le_bytes()).unwrap(); // cChecksum
    buf.write_all(&cb_data.to_le_bytes()).unwrap();
    buf.write_all(&cb_uncomp.to_le_bytes()).unwrap();
    buf.write_all(b"CK").unwrap();
    buf.write_all(&compressed).unwrap();
    
    // CFFILE
    buf.write_all(&(content.len() as u32).to_le_bytes()).unwrap();
    buf.write_all(&0u32.to_le_bytes()).unwrap(); // offset
    buf.write_all(&0u16.to_le_bytes()).unwrap(); // iFolder
    buf.write_all(&0u16.to_le_bytes()).unwrap(); // flTime
    buf.write_all(&0x3019u16.to_le_bytes()).unwrap(); // flDate
    buf.write_all(&0x20u16.to_le_bytes()).unwrap(); // attribs
    buf.write_all(b"hello.txt").unwrap();
    buf.push(0); // null terminator
    
    buf
}
