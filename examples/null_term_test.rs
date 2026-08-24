/// Test: add null terminators to _StringData and see if it fixes error 2705.
use velocity_msi::{MsiBuilder, Column, Value};
use std::io::Write;

fn main() {
    // Build a minimal MSI with Property + Directory + InstallExecuteSequence
    let mut builder = MsiBuilder::new();
    builder.set_title("Null Term Test");
    builder.set_author("Velocity");
    builder.set_template("Intel", 1033);

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(1024).build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Null Term Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity Corp")],
        vec![Value::from("ProductCode"), Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![Value::from("UpgradeCode"), Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("VelTest:VelTest")],
    ]).unwrap();

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

    let msi = builder.build().unwrap();
    std::fs::write("null_term_orig.msi", &msi).unwrap();

    // Now patch the _StringData stream to add null terminators
    // Read back with cfb, modify _StringData, write new MSI
    use std::io::{Read, Cursor};

    let mut cfb_in = cfb::CompoundFile::open(Cursor::new(&msi)).unwrap();

    // Find the _StringData stream
    let data_name = velocity_msi::encode_stream_name("_StringData", true);
    let pool_name = velocity_msi::encode_stream_name("_StringPool", true);

    let mut pool_data = Vec::new();
    cfb_in.open_stream(pool_name.as_str()).unwrap().read_to_end(&mut pool_data).unwrap();

    let mut string_data = Vec::new();
    cfb_in.open_stream(data_name.as_str()).unwrap().read_to_end(&mut string_data).unwrap();

    println!("Original _StringData: {} bytes", string_data.len());

    // Parse the string pool to get lengths
    let codepage = u32::from_le_bytes([pool_data[0], pool_data[1], pool_data[2], pool_data[3]]);
    let num_strings = (pool_data.len() - 4) / 4;
    println!("String pool: cp={}, {} strings", codepage & 0xFFFF, num_strings);

    // Read each string and rebuild with null terminators
    let mut new_string_data = Vec::new();
    let mut new_pool_entries = Vec::new();
    let mut data_off = 0usize;

    for i in 0..num_strings {
        let base = 4 + i * 4;
        let len = u16::from_le_bytes([pool_data[base], pool_data[base + 1]]) as usize;
        let rc = u16::from_le_bytes([pool_data[base + 2], pool_data[base + 3]]);
        let bytes = &string_data[data_off..data_off + len];
        let text = String::from_utf8_lossy(bytes).to_string();

        // Write string bytes + null terminator
        new_string_data.write_all(bytes).unwrap();
        new_string_data.push(0); // null terminator

        // Pool entry: length stays the same (NOT including null terminator)
        new_pool_entries.push((len as u16, rc));

        if i < 5 || text.starts_with("_") || text.starts_with("TARGET") || text.starts_with("INSTALL") {
            println!("  String {}: {:3}b {:?} (with null: {}b)", i + 1, len, text, len + 1);
        }

        data_off += len;
    }

    println!("New _StringData: {} bytes (was {})", new_string_data.len(), string_data.len());

    // Rebuild _StringPool with same lengths (null terminators don't change the length field)
    let mut new_pool_data = Vec::new();
    new_pool_data.write_all(&pool_data[..4]).unwrap(); // header
    for (len, rc) in &new_pool_entries {
        new_pool_data.write_all(&len.to_le_bytes()).unwrap();
        new_pool_data.write_all(&rc.to_le_bytes()).unwrap();
    }

    // Now create a new MSI with cfb, copying all streams but replacing _StringData
    let cfb_file = std::fs::File::create("null_term_test.msi").unwrap();
    let mut cfb_out = cfb::CompoundFile::create(cfb_file).unwrap();

    let entries: Vec<(String, bool)> = cfb_in.walk()
        .map(|e| (e.path().to_string_lossy().to_string(), e.is_stream()))
        .collect();

    for (path, is_stream) in &entries {
        if !is_stream { continue; }
        let mut data = Vec::new();
        cfb_in.open_stream(path.as_str()).unwrap().read_to_end(&mut data).unwrap();

        // Replace _StringData
        if path == &format!("/{}", data_name) {
            println!("Replacing _StringData stream");
            data = new_string_data.clone();
        }
        // Also replace _StringPool if we modified it
        if path == &format!("/{}", pool_name) {
            println!("Replacing _StringPool stream");
            data = new_pool_data.clone();
        }

        cfb_out.create_stream(path.as_str()).unwrap().write_all(&data).unwrap();
    }
    drop(cfb_out);

    let size = std::fs::metadata("null_term_test.msi").unwrap().len();
    println!("\nNull-terminated MSI: {} bytes", size);

    // Test both
    for (label, fname) in &[("Original", "null_term_orig.msi"), ("Null-term", "null_term_test.msi")] {
        let _ = std::fs::remove_dir_all("C:\\VelTest");
        let log = fname.replace(".msi", ".log");
        let output = std::process::Command::new("msiexec")
            .args(&["/i", fname, "/qn", "/l*v", &log])
            .output().unwrap();
        let exit = output.status.code().unwrap_or(-1);
        println!("{:20} exit={}", label, exit);

        if exit != 0 {
            if let Ok(logtext) = std::fs::read_to_string(&log) {
                for line in logtext.lines() {
                    if line.contains("Error ") && !line.contains("Error 0") && !line.contains("2205") && !line.contains("2228") {
                        println!("  {}", line.trim());
                    }
                }
            }
        }
    }
}
