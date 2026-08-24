//! Compare stream structure of compiler MSI vs test MSI
use std::io::Cursor;

fn main() {
    // Read the compiler MSI
    let compiler_msi = std::fs::read(r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\installer.msi").unwrap();
    println!("=== compiler installer.msi ({} bytes) ===", compiler_msi.len());
    dump_streams(&compiler_msi);

    // Read the test MSI (if it exists from a previous run)
    let test_msi_path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\crates\velocity-msi\examples\test_8col_file.msi";
    if let Ok(data) = std::fs::read(test_msi_path) {
        println!("\n=== test_8col_file.msi ({} bytes) ===", data.len());
        dump_streams(&data);
    } else {
        println!("\ntest_8col_file.msi not found, building one...");
        
        // Build a minimal MSI matching test_8col_file pattern
        let mut b = velocity_msi::MsiBuilder::new();
        b.set_title("Test");
        b.set_author("Test");
        b.set_template("x64", 1033);

        b.create_table("Property", vec![
            velocity_msi::Column::build("Property").string(72).primary_key().build(),
            velocity_msi::Column::build("Value").string(255).nullable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("Test")],
            vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0")],
            vec![velocity_msi::Value::from("Manufacturer"), velocity_msi::Value::from("Test")],
            vec![velocity_msi::Value::from("ProductCode"), velocity_msi::Value::from("{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}")],
            vec![velocity_msi::Value::from("UpgradeCode"), velocity_msi::Value::from("{11111111-2222-3333-4444-555555555555}")],
            vec![velocity_msi::Value::from("ProductLanguage"), velocity_msi::Value::from("1033")],
        ]).unwrap();

        b.create_table("Directory", vec![
            velocity_msi::Column::build("Directory").string(72).primary_key().build(),
            velocity_msi::Column::build("Directory_Parent").string(72).nullable().build(),
            velocity_msi::Column::build("DefaultDir").string(255).nullable().build(),
        ]).unwrap();
        b.insert_rows("Directory", vec![
            vec![velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::Null, velocity_msi::Value::from("SourceDir")],
            vec![velocity_msi::Value::from("INSTALLDIR"), velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::from("TestDir:TESTDIR")],
        ]).unwrap();

        b.create_table("Component", vec![
            velocity_msi::Column::build("Component").string(72).primary_key().build(),
            velocity_msi::Column::build("ComponentId").string(38).nullable().build(),
            velocity_msi::Column::build("Directory_").string(72).nullable().build(),
            velocity_msi::Column::build("Attributes").int16().nullable().build(),
            velocity_msi::Column::build("Condition").string(255).nullable().build(),
            velocity_msi::Column::build("KeyPath").string(72).nullable().build(),
        ]).unwrap();
        b.insert_rows("Component", vec![
            vec![velocity_msi::Value::from("C1"), velocity_msi::Value::from("{BBBBBBBB-CCCC-DDDD-EEEE-FFFFFFFFFFFF}"), velocity_msi::Value::from("INSTALLDIR"), velocity_msi::Value::Int(0), velocity_msi::Value::Null, velocity_msi::Value::from("F1")],
        ]).unwrap();

        b.create_table("File", vec![
            velocity_msi::Column::build("File").string(72).primary_key().build(),
            velocity_msi::Column::build("Component_").string(72).build(),
            velocity_msi::Column::build("FileName").string(255).build(),
            velocity_msi::Column::build("FileSize").int32().build(),
            velocity_msi::Column::build("Version").string(72).nullable().build(),
            velocity_msi::Column::build("Language").int16().nullable().build(),
            velocity_msi::Column::build("Attributes").int16().nullable().build(),
            velocity_msi::Column::build("Sequence").int32().build(),
        ]).unwrap();
        b.insert_rows("File", vec![
            vec![velocity_msi::Value::from("F1"), velocity_msi::Value::from("C1"), velocity_msi::Value::from("test.txt"), velocity_msi::Value::Int(24), velocity_msi::Value::Null, velocity_msi::Value::Int(0), velocity_msi::Value::Int(0), velocity_msi::Value::Int(1)],
        ]).unwrap();

        b.create_table("Feature", vec![
            velocity_msi::Column::build("Feature").string(38).primary_key().build(),
            velocity_msi::Column::build("Feature_Parent").string(38).nullable().build(),
            velocity_msi::Column::build("Title").string(64).nullable().build(),
            velocity_msi::Column::build("Description").string(255).nullable().build(),
            velocity_msi::Column::build("Display").int16().nullable().build(),
            velocity_msi::Column::build("Level").int16().nullable().build(),
            velocity_msi::Column::build("Directory_").string(72).nullable().build(),
            velocity_msi::Column::build("Attributes").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("Feature", vec![
            vec![velocity_msi::Value::from("Complete"), velocity_msi::Value::Null, velocity_msi::Value::from("Complete"), velocity_msi::Value::Null, velocity_msi::Value::Int(0), velocity_msi::Value::Int(1), velocity_msi::Value::Null, velocity_msi::Value::Int(0)],
        ]).unwrap();

        b.create_table("FeatureComponents", vec![
            velocity_msi::Column::build("Feature_").string(38).primary_key().build(),
            velocity_msi::Column::build("Component_").string(72).primary_key().build(),
        ]).unwrap();
        b.insert_rows("FeatureComponents", vec![
            vec![velocity_msi::Value::from("Complete"), velocity_msi::Value::from("C1")],
        ]).unwrap();

        b.create_table("Media", vec![
            velocity_msi::Column::build("DiskId").int16().primary_key().build(),
            velocity_msi::Column::build("LastSequence").int32().build(),
            velocity_msi::Column::build("DiskPrompt").string(255).nullable().build(),
            velocity_msi::Column::build("VolumeLabel").string(32).nullable().build(),
            velocity_msi::Column::build("Cabinet").string(255).nullable().build(),
            velocity_msi::Column::build("Source").string(72).nullable().build(),
        ]).unwrap();
        b.insert_rows("Media", vec![
            vec![velocity_msi::Value::Int(1), velocity_msi::Value::Int(1), velocity_msi::Value::Null, velocity_msi::Value::Null, velocity_msi::Value::from("#velo.cab"), velocity_msi::Value::Null],
        ]).unwrap();

        let cabinet = velocity_msi::build_cabinet(&[velocity_msi::CabinetFile {
            name: "F1".to_string(),
            data: b"Hello from Velocity MSI!".to_vec(),
        }]);
        b.add_stream("velo.cab".to_string(), cabinet);

        let msi = b.build().unwrap();
        std::fs::write("test_minimal.msi", &msi).unwrap();
        println!("\n=== test_minimal.msi ({} bytes) ===", msi.len());
        dump_streams(&msi);
    }
}

fn dump_streams(msi_data: &[u8]) {
    let cursor = Cursor::new(msi_data);
    match cfb::CompoundFile::open(cursor) {
        Ok(comp) => {
            let entries = comp.walk();
            let mut stream_count = 0;
            let mut empty_streams = Vec::new();
            let mut total_data = 0usize;
            for entry in entries {
                if entry.is_stream() {
                    stream_count += 1;
                    let size = entry.len();
                    total_data += size as usize;
                    let name = entry.path().to_string_lossy().to_string();
                    if size == 0 {
                        empty_streams.push(name.clone());
                    }
                    println!("  {:50} {} bytes", name, size);
                }
            }
            println!("  ---");
            println!("  Total: {} streams, {} bytes data, {} empty streams", stream_count, total_data, empty_streams.len());
            if !empty_streams.is_empty() {
                println!("  EMPTY STREAMS:");
                for s in &empty_streams {
                    println!("    - {}", s);
                }
            }
        }
        Err(e) => println!("  ERROR opening: {}", e),
    }
}
