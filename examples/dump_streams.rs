/// Dump all stream contents from a velocity-msi generated MSI for debugging.
/// This reveals the actual binary data in each table stream.
use std::io::Read;

fn main() {
    println!("=== Stream Content Dump ===\n");

    let mut b = velocity_msi::MsiBuilder::new();
    b.set_title("Dump Test");
    b.set_author("Velocity");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        velocity_msi::Column::build("Property").string(72).primary_key().build(),
        velocity_msi::Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("Dump Test")],
        vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0.0")],
        vec![velocity_msi::Value::from("Manufacturer"), velocity_msi::Value::from("Velocity Corp")],
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
        vec![velocity_msi::Value::from("INSTALLDIR"), velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::from("VelTest:VelTest")],
    ]).unwrap();

    b.create_table("InstallExecuteSequence", vec![
        velocity_msi::Column::build("Action").string(72).primary_key().build(),
        velocity_msi::Column::build("Condition").string(255).nullable().build(),
        velocity_msi::Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![velocity_msi::Value::from("CostInitialize"), velocity_msi::Value::Null, velocity_msi::Value::Int(800)],
        vec![velocity_msi::Value::from("CostFinalize"), velocity_msi::Value::Null, velocity_msi::Value::Int(1000)],
    ]).unwrap();

    let msi_data = b.build().unwrap();
    println!("MSI size: {} bytes\n", msi_data.len());

    // Write to temp file and open with cfb
    let path = "dump_test.msi";
    std::fs::write(path, &msi_data).unwrap();

    let file = std::fs::File::open(path).unwrap();
    let comp = cfb::CompoundFile::open(file).unwrap();

    // Collect stream info
    let stream_entries: Vec<String> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();

    // Re-open to read streams
    drop(comp);
    let file2 = std::fs::File::open(path).unwrap();
    let mut comp2 = cfb::CompoundFile::open(file2).unwrap();

    for stream_path in &stream_entries {
        let data = {
            let mut reader = comp2.open_stream(stream_path.as_str()).unwrap();
            let mut buf = Vec::new();
            reader.read_to_end(&mut buf).unwrap();
            buf
        };

        println!("=== {} ({} bytes) ===", stream_path, data.len());

        // Decode string pool header
        if data.len() >= 4 {
            let header = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            let codepage = header & 0xFFFF;
            let long_refs = (header >> 31) & 1 == 1;
            // Heuristic: if codepage is 1252 and entry count makes sense, it's _StringPool
            if codepage == 1252 {
                let entry_count = (data.len() - 4) / 4;
                if entry_count > 0 && (data.len() - 4) % 4 == 0 {
                    println!("  [StringPool] Header: codepage={}, long_refs={}", codepage, long_refs);
                    println!("  Entries: {}", entry_count);
                    for i in 0..entry_count.min(10) {
                        let off = 4 + i * 4;
                        let len = u16::from_le_bytes([data[off], data[off + 1]]);
                        let rc = u16::from_le_bytes([data[off + 2], data[off + 3]]);
                        println!("    [{}]: len={}, refcount={}", i, len, rc);
                    }
                    if entry_count > 10 {
                        println!("    ... and {} more entries", entry_count - 10);
                    }
                }
            }
        }

        // Hex dump (first 256 bytes)
        let dump_len = data.len().min(256);
        for row in (0..dump_len).step_by(16) {
            print!("  {:04x}: ", row);
            for j in 0..16 {
                if row + j < data.len() {
                    print!("{:02x} ", data[row + j]);
                } else {
                    print!("   ");
                }
            }
            print!(" |");
            for j in 0..16 {
                if row + j < data.len() {
                    let b = data[row + j];
                    if b >= 0x20 && b < 0x7f {
                        print!("{}", b as char);
                    } else {
                        print!(".");
                    }
                }
            }
            println!("|");
        }
        if data.len() > 256 {
            println!("  ... ({} more bytes)", data.len() - 256);
        }
        println!();
    }

    // Now verify with msi crate
    println!("\n=== msi crate verification ===");
    let file3 = std::fs::File::open(path).unwrap();
    match msi::Package::open(file3) {
        Ok(pkg) => {
            for table in pkg.tables() {
                let name = table.name();
                let col_count = table.columns().len();
                println!("  Table '{}': {} cols", name, col_count);
                for (i, col) in table.columns().iter().enumerate() {
                    println!("    Col {}: '{}' type={:?}", i, col.name(), col.coltype());
                }
            }
        }
        Err(e) => println!("  msi crate open FAILED: {}", e),
    }

    // Expected sizes
    println!("\n=== Expected vs Actual Stream Sizes ===");
    println!("Property: 6 rows x 2 string cols = 24 bytes expected (col-major: 2x6x2 = 24)");
    println!("Directory: 2 rows x 3 string cols = 12 bytes expected (col-major: 3x2x2 = 12)");
    println!("InstallExecSeq: 2 rows x (2 string + 1 int16) = 12 bytes expected");
    println!("_Tables: N rows x 1 string col = Nx2 bytes");
    println!("_Columns: M rows x (3 string + 1 int16) = Mx8 bytes");
    println!("_Validation: K rows x (7 string + 3 int) = Kx22 bytes");
}
