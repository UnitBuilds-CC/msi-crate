/// Verify that the msi crate properly writes changes to disk.
/// Opens our base MSI, adds InstallExecuteSequence, and reads back the file.
use std::fs;
use std::io::Read;
use velocity_msi::{Column, MsiBuilder, Value};

fn build_base_msi() -> Vec<u8> {
    let mut builder = MsiBuilder::new();
    builder.set_title("Verify Test");
    builder.set_author("Velocity");
    builder.set_template("Intel", 1033);

    builder
        .create_table(
            "Property",
            vec![
                Column::build("Property").string(72).primary_key().build(),
                Column::build("Value").string(1024).build(),
            ],
        )
        .unwrap();
    builder
        .insert_rows(
            "Property",
            vec![
                vec![Value::from("ProductName"), Value::from("Verify Test")],
                vec![Value::from("ProductVersion"), Value::from("1.0.0")],
                vec![Value::from("Manufacturer"), Value::from("Velocity Corp")],
                vec![
                    Value::from("ProductCode"),
                    Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}"),
                ],
                vec![
                    Value::from("UpgradeCode"),
                    Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}"),
                ],
                vec![Value::from("ProductLanguage"), Value::from("1033")],
            ],
        )
        .unwrap();

    builder
        .create_table(
            "Directory",
            vec![
                Column::build("Directory").string(72).primary_key().build(),
                Column::build("Directory_Parent")
                    .string(72)
                    .nullable()
                    .build(),
                Column::build("DefaultDir").string(255).primary_key().build(),
            ],
        )
        .unwrap();
    builder
        .insert_rows(
            "Directory",
            vec![
                vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
                vec![
                    Value::from("INSTALLDIR"),
                    Value::from("TARGETDIR"),
                    Value::from("VelTest:VelTest"),
                ],
            ],
        )
        .unwrap();

    builder.build().unwrap()
}

fn main() {
    println!("=== Verify msi crate write ===\n");

    let msi_data = build_base_msi();
    let base_path = "verify_base.msi";
    fs::write(base_path, &msi_data).unwrap();
    println!("Base MSI: {} bytes", msi_data.len());

    // Open with msi crate and add InstallExecuteSequence
    let mod_path = "verify_modified.msi";
    fs::copy(base_path, mod_path).unwrap();

    {
        let mut pkg = msi::open_rw(mod_path).expect("open_rw");

        let columns = vec![
            msi::Column::build("Action").primary_key().string(72),
            msi::Column::build("Condition").nullable().string(255),
            msi::Column::build("Sequence").nullable().int16(),
        ];
        pkg.create_table("InstallExecuteSequence", columns)
            .expect("create_table");

        pkg.insert_rows(
            msi::Insert::into("InstallExecuteSequence")
                .row(vec![
                    msi::Value::Str("CostInitialize".into()),
                    msi::Value::Null,
                    msi::Value::Int(800),
                ])
                .row(vec![
                    msi::Value::Str("CostFinalize".into()),
                    msi::Value::Null,
                    msi::Value::Int(1000),
                ]),
        )
        .expect("insert_rows");

        pkg.flush().expect("flush");
        println!("Flushed. Dropping package...");
    } // drop here

    let modified_size = fs::metadata(mod_path).unwrap().len();
    println!("Modified MSI: {} bytes", modified_size);

    // Read back with cfb to verify streams
    let data = fs::read(mod_path).unwrap();
    let cursor = std::io::Cursor::new(&data);
    match cfb::CompoundFile::open(cursor) {
        Ok(mut comp) => {
            println!("\nStreams in modified file:");
            for entry in comp.walk() {
                if entry.is_stream() {
                    println!("  {} ({} bytes)", entry.name(), entry.len());
                }
            }

            // Check if InstallExecuteSequence stream exists
            // The encoded name for InstallExecuteSequence with TABLE_PREFIX
            let enc_name = velocity_msi::encode_stream_name("InstallExecuteSequence", true);
            println!("\nLooking for encoded stream name: {:?}", enc_name);
            if comp.exists(&enc_name) {
                let mut stream = comp.open_stream(&enc_name).unwrap();
                let mut stream_data = Vec::new();
                stream.read_to_end(&mut stream_data).unwrap();
                println!("InstallExecuteSequence stream: {} bytes", stream_data.len());
                // Dump hex
                print!("  Data: ");
                for (i, &b) in stream_data.iter().enumerate() {
                    if i > 0 && i % 16 == 0 {
                        print!("\n  ");
                    }
                    print!("{:02X} ", b);
                }
                println!();
            } else {
                println!("InstallExecuteSequence stream NOT FOUND!");
            }
        }
        Err(e) => {
            println!("Failed to open modified file with cfb: {}", e);
        }
    }

    // Also try reading back with msi crate
    println!("\nReading back with msi crate...");
    match msi::open_rw(mod_path) {
        Ok(pkg) => {
            println!("msi crate can open the modified file.");
            // Try to read InstallExecuteSequence
            for table in pkg.tables() {
                println!("  Table: {} ({} columns)", table.name(), table.columns().len());
            }
        }
        Err(e) => {
            println!("msi crate CANNOT open modified file: {}", e);
        }
    }

    println!("\nDone.");
}
