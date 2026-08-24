/// Definitive data diagnostic: dump ALL stream data from our MSI and verify
/// every byte against the MSI specification.
///
/// The goal: find exactly what's wrong with the Directory table data that
/// causes msiexec error 2705 ("Could not be linked as tree").
use std::io::Cursor;

fn main() {
    println!("=== DEFINITIVE DATA DIAGNOSTIC ===\n");

    // Build the exact same MSI as the failing test
    let mut b = velocity_msi::MsiBuilder::new();
    b.set_title("DataDiag");
    b.set_author("Velocity");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        velocity_msi::Column::build("Property").string(72).primary_key().build(),
        velocity_msi::Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("DataDiag")],
        vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0.0")],
        vec![velocity_msi::Value::from("Manufacturer"), velocity_msi::Value::from("V")],
        vec![velocity_msi::Value::from("ProductCode"), velocity_msi::Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![velocity_msi::Value::from("UpgradeCode"), velocity_msi::Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![velocity_msi::Value::from("ProductLanguage"), velocity_msi::Value::from("1033")],
    ]).unwrap();

    b.create_table("Directory", vec![
        velocity_msi::Column::build("Directory").string(72).primary_key().build(),
        velocity_msi::Column::build("Directory_Parent").string(72).nullable().build(),
        velocity_msi::Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::Null, velocity_msi::Value::from("SourceDir")],
    ]).unwrap();

    b.create_table("InstallExecuteSequence", vec![
        velocity_msi::Column::build("Action").string(72).primary_key().build(),
        velocity_msi::Column::build("Condition").string(255).nullable().build(),
        velocity_msi::Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![velocity_msi::Value::from("CostInitialize"), velocity_msi::Value::Null, velocity_msi::Value::Int(800)],
    ]).unwrap();

    let msi_data = b.build().unwrap();
    println!("MSI size: {} bytes\n", msi_data.len());

    // Open with cfb to extract all streams
    let comp = cfb::CompoundFile::open(Cursor::new(&msi_data)).unwrap();

    // Collect all stream names and data
    let stream_names: Vec<String> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.name().to_string())
        .collect();

    let mut streams: Vec<(String, Vec<u8>)> = Vec::new();
    for name in &stream_names {
        let mut comp2 = cfb::CompoundFile::open(Cursor::new(&msi_data)).unwrap();
        let mut stream = comp2.open_stream(name.as_str()).unwrap();
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut stream, &mut data).unwrap();
        streams.push((name.clone(), data));
    }

    // Find and decode _StringPool
    let pool_stream = streams.iter().find(|(n, _)| n.contains("_StringPool") || n.contains("\u{4840}\u{457f}\u{46f2}\u{4762}\u{436e}\u{4568}\u{4176}\u{4836}"));
    let data_stream = streams.iter().find(|(n, _)| n.contains("_StringData") || n.contains("\u{4840}\u{457f}\u{46f2}\u{4762}\u{436e}\u{4168}\u{4376}\u{4836}"));

    if let Some((pool_name, pool_data)) = pool_stream {
        println!("=== _StringPool ===");
        println!("Stream name: {:?}", pool_name);
        println!("Stream name chars: {:?}", pool_name.chars().map(|c| format!("U+{:04X}", c as u32)).collect::<Vec<_>>());
        println!("Size: {} bytes", pool_data.len());

        if pool_data.len() >= 4 {
            let header = u32::from_le_bytes([pool_data[0], pool_data[1], pool_data[2], pool_data[3]]);
            let codepage = header & 0xFFFF;
            let long_refs = (header >> 31) & 1;
            println!("Header: 0x{:08X}", header);
            println!("  Codepage: {}", codepage);
            println!("  Long refs: {}", long_refs);

            // Decode entries
            let mut offset = 4;
            let mut id = 1u32;
            let mut total_name_len = 0usize;
            println!("\n  Entries:");
            while offset + 4 <= pool_data.len() {
                let len = u16::from_le_bytes([pool_data[offset], pool_data[offset + 1]]) as usize;
                let refcount = u16::from_le_bytes([pool_data[offset + 2], pool_data[offset + 3]]);
                offset += 4;
                total_name_len += len;

                // Get the actual string from _StringData
                let string_val = if let Some((_, sd)) = data_stream {
                    // We need to compute the offset in _StringData
                    // This is the cumulative length of all strings before this one
                    // For now, just show the length and refcount
                    format!("(len={}, refcount={})", len, refcount)
                } else {
                    format!("(len={}, refcount={})", len, refcount)
                };

                if id <= 60 || refcount > 0 {
                    println!("    ID {:2}: len={:2} refcount={:2} {}", id, len, refcount, string_val);
                }
                id += 1;
            }
            println!("  Total entries: {}", id - 1);
            println!("  Total name bytes: {}", total_name_len);
            println!("  Pool data consumed: {} of {} bytes", offset, pool_data.len());

            // Now decode _StringData and show actual strings
            if let Some((data_name, sd)) = data_stream {
                println!("\n=== _StringData ===");
                println!("Stream name: {:?}", data_name);
                println!("Size: {} bytes", sd.len());

                // Re-read pool entries to get lengths
                let mut offset2 = 4;
                let mut data_offset = 0;
                let mut id2 = 1u32;
                println!("\n  Strings:");
                while offset2 + 4 <= pool_data.len() && data_offset < sd.len() {
                    let len = u16::from_le_bytes([pool_data[offset2], pool_data[offset2 + 1]]) as usize;
                    offset2 += 4;

                    let end = (data_offset + len).min(sd.len());
                    let bytes = &sd[data_offset..end];
                    let s = String::from_utf8_lossy(bytes);
                    let win1252: String = bytes.iter().map(|&b| {
                        if b < 0x80 { b as char }
                        else if b <= 0xFF { char::from(b) }
                        else { '?' }
                    }).collect();

                    if id2 <= 60 {
                        println!("    ID {:2}: [{:2}] {:?} → win1252: {:?}", id2, len, s.as_ref(), win1252);
                    }
                    data_offset = end;
                    id2 += 1;
                }
                println!("  Data consumed: {} of {} bytes", data_offset, sd.len());
            }
        }
    } else {
        println!("ERROR: _StringPool stream not found!");
        println!("Available streams:");
        for (n, d) in &streams {
            println!("  {:?} ({} bytes)", n, d.len());
        }
    }

    // Find and decode Directory stream
    println!("\n=== Directory Stream ===");
    let dir_stream = streams.iter().find(|(n, _)| {
        // Directory encoded with TABLE_PREFIX
        let enc = velocity_msi::encode_stream_name("Directory", true);
        n == &enc
    });
    if let Some((name, data)) = dir_stream {
        println!("Stream name: {:?}", name);
        println!("Size: {} bytes", data.len());
        println!("Hex: {}", data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "));

        // Decode: column-major, 2 rows × 3 columns
        // Column 0 (Directory): 2 × u16 = 4 bytes
        // Column 1 (Directory_Parent): 2 × u16 = 4 bytes
        // Column 2 (DefaultDir): 2 × u16 = 4 bytes
        if data.len() >= 12 {
            let dir_id0 = u16::from_le_bytes([data[0], data[1]]);
            let dir_id1 = u16::from_le_bytes([data[2], data[3]]);
            let par_id0 = u16::from_le_bytes([data[4], data[5]]);
            let par_id1 = u16::from_le_bytes([data[6], data[7]]);
            let def_id0 = u16::from_le_bytes([data[8], data[9]]);
            let def_id1 = u16::from_le_bytes([data[10], data[11]]);
            println!("Row 0: Directory={}, Parent={}, DefaultDir={}", dir_id0, par_id0, def_id0);
            if data.len() > 12 {
                println!("Row 1: Directory={}, Parent={}, DefaultDir={}", dir_id1, par_id1, def_id1);
            }
            println!("Expected: Row 0: Directory=TARGETDIR, Parent=NULL(0), DefaultDir=SourceDir");
        }
    } else {
        println!("Directory stream not found!");
    }

    // Find and decode _Tables stream
    println!("\n=== _Tables Stream ===");
    let tables_stream = streams.iter().find(|(n, _)| {
        let enc = velocity_msi::encode_stream_name("_Tables", true);
        n == &enc
    });
    if let Some((name, data)) = tables_stream {
        println!("Stream name: {:?}", name);
        println!("Size: {} bytes", data.len());
        println!("Hex: {}", data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "));
        // _Tables has 1 column (Name, string), so each row is 2 bytes (1 pool ID)
        for (i, chunk) in data.chunks(2).enumerate() {
            if chunk.len() == 2 {
                let id = u16::from_le_bytes([chunk[0], chunk[1]]);
                println!("  Row {}: pool ID {}", i, id);
            }
        }
    }

    // Find and decode _Columns stream
    println!("\n=== _Columns Stream (Directory entries only) ===");
    let cols_stream = streams.iter().find(|(n, _)| {
        let enc = velocity_msi::encode_stream_name("_Columns", true);
        n == &enc
    });
    if let Some((name, data)) = cols_stream {
        println!("Stream name: {:?}", name);
        println!("Size: {} bytes", data.len());
        // _Columns has 4 columns: Table(string), Number(int16), Name(string), Type(int16)
        // Column-major: Table_ids | Number_vals | Name_ids | Type_vals
        // We need to know the row count to parse column-major data
        // Row count = total entries in _Tables + _Columns + _Validation
        // For now, just dump the hex
        println!("Hex: {}", data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "));
    }

    // Now use the msi crate to read our MSI and verify what it sees
    println!("\n=== msi crate reading our MSI ===");
    let path = "data_diag_test.msi";
    std::fs::write(path, &msi_data).unwrap();

    match msi::Package::open(std::fs::File::open(path).unwrap()) {
        Ok(mut pkg) => {
            // Read Directory table
            println!("Reading Directory table:");
            if let Ok(rows) = pkg.select_rows(msi::Select::table("Directory")) {
                for row in rows {
                    println!("  {:?}", (0..3).map(|i| {
                        let v = &row[i];
                        if let Some(s) = v.as_str() { format!("Str({})", s) }
                        else if let Some(n) = v.as_int() { format!("Int({})", n) }
                        else { "Null".to_string() }
                    }).collect::<Vec<_>>());
                }
            }

            // Read _Tables
            println!("\nReading _Tables:");
            if let Ok(rows2) = pkg.select_rows(msi::Select::table("_Tables")) {
                for row in rows2 {
                    let name = row[0].as_str().unwrap_or("?");
                    println!("  {}", name);
                }
            }

            // Read _Columns for Directory
            println!("\nReading _Columns for Directory:");
            if let Ok(rows3) = pkg.select_rows(msi::Select::table("_Columns")) {
                for row in rows3 {
                    let table = row[0].as_str().unwrap_or("?");
                    if table == "Directory" {
                        let number = row[1].as_int().unwrap_or(0);
                        let name = row[2].as_str().unwrap_or("?");
                        let typ = row[3].as_int().unwrap_or(0);
                        println!("  Table={}, Number={}, Name={}, Type=0x{:04X}", table, number, name, typ);
                    }
                }
            }
        }
        Err(e) => {
            println!("ERROR opening MSI with msi crate: {:?}", e);
        }
    }

    // Clean up
    let _ = std::fs::remove_file(path);

    println!("\n=== DONE ===");
}
