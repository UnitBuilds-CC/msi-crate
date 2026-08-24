/// Binary comparison: msi crate reference vs velocity-msi output.
use std::io::Cursor;
use msi::{Package, PackageType, Column, Insert, Value, CodePage};
use velocity_msi::{MsiBuilder, Column as VColumn, Value as VValue, CabinetFile, build_cabinet};

fn main() {
    // ═══ REFERENCE MSI via msi crate ═══
    let ref_msi = {
        let cursor = Cursor::new(Vec::new());
        let mut pkg = Package::create(PackageType::Installer, cursor).unwrap();
        pkg.set_database_codepage(CodePage::Windows1252);
        pkg.summary_info_mut().set_title("Velocity Ref Test");

        pkg.create_table("Property", vec![
            Column::build("Property").primary_key().string(72),
            Column::build("Value").string(1024),
        ]).unwrap();
        pkg.insert_rows(Insert::into("Property")
            .row(vec![Value::Str("ProductName".into()), Value::Str("Velocity Test".into())])
            .row(vec![Value::Str("ProductVersion".into()), Value::Str("1.0.0".into())])
            .row(vec![Value::Str("Manufacturer".into()), Value::Str("Velocity Corp".into())])
            .row(vec![Value::Str("ProductCode".into()), Value::Str("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}".into())])
            .row(vec![Value::Str("UpgradeCode".into()), Value::Str("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}".into())])
            .row(vec![Value::Str("ProductLanguage".into()), Value::Str("1033".into())])
        ).unwrap();

        pkg.create_table("Directory", vec![
            Column::build("Directory").primary_key().string(72),
            Column::build("Directory_Parent").nullable().string(72),
            Column::build("DefaultDir").primary_key().string(255),
        ]).unwrap();
        pkg.insert_rows(Insert::into("Directory")
            .row(vec![Value::Str("TARGETDIR".into()), Value::Null, Value::Str("SourceDir".into())])
            .row(vec![Value::Str("INSTALLDIR".into()), Value::Str("TARGETDIR".into()), Value::Str("VelTest:VelTest".into())])
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
                      Value::Int(0), Value::Null, Value::Str("hello.txt".into())])
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
        pkg.insert_rows(Insert::into("File")
            .row(vec![Value::Str("hello.txt".into()), Value::Str("MainComp".into()),
                      Value::Str("hello.txt".into()), Value::Int(26),
                      Value::Null, Value::Null, Value::Null, Value::Int(1)])
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
            .row(vec![Value::Str("Complete".into()), Value::Null,
                      Value::Str("Complete Install".into()),
                      Value::Str("Installs all files".into()),
                      Value::Int(1), Value::Int(1), Value::Null, Value::Null])
        ).unwrap();

        pkg.create_table("FeatureComponents", vec![
            Column::build("Feature_").primary_key().string(38),
            Column::build("Component_").primary_key().string(72),
        ]).unwrap();
        pkg.insert_rows(Insert::into("FeatureComponents")
            .row(vec![Value::Str("Complete".into()), Value::Str("MainComp".into())])
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

        pkg.create_table("InstallExecuteSequence", vec![
            Column::build("Action").primary_key().string(72),
            Column::build("Condition").nullable().string(255),
            Column::build("Sequence").nullable().int16(),
        ]).unwrap();
        pkg.insert_rows(Insert::into("InstallExecuteSequence")
            .row(vec![Value::Str("CostFinalize".into()), Value::Null, Value::Int(1000)])
            .row(vec![Value::Str("CostInitialize".into()), Value::Null, Value::Int(800)])
            .row(vec![Value::Str("FileCost".into()), Value::Null, Value::Int(900)])
            .row(vec![Value::Str("InstallFiles".into()), Value::Null, Value::Int(4000)])
            .row(vec![Value::Str("InstallFinalize".into()), Value::Null, Value::Int(6600)])
            .row(vec![Value::Str("InstallInitialize".into()), Value::Null, Value::Int(1500)])
            .row(vec![Value::Str("InstallValidate".into()), Value::Null, Value::Int(1400)])
        ).unwrap();

        pkg.create_table("InstallUISequence", vec![
            Column::build("Action").primary_key().string(72),
            Column::build("Condition").nullable().string(255),
            Column::build("Sequence").nullable().int16(),
        ]).unwrap();
        pkg.insert_rows(Insert::into("InstallUISequence")
            .row(vec![Value::Str("CostFinalize".into()), Value::Null, Value::Int(1000)])
            .row(vec![Value::Str("CostInitialize".into()), Value::Null, Value::Int(800)])
            .row(vec![Value::Str("ExecuteAction".into()), Value::Null, Value::Int(1300)])
            .row(vec![Value::Str("FileCost".into()), Value::Null, Value::Int(900)])
        ).unwrap();

        let content = b"Hello from Velocity MSI!\r\n";
        let cab = build_cabinet(&[CabinetFile {
            name: "hello.txt".to_string(),
            data: content.to_vec(),
        }]);
        {
            let mut writer = pkg.write_stream("vel.cab").unwrap();
            std::io::Write::write_all(&mut writer, &cab).unwrap();
        }
        pkg.flush().unwrap();
        pkg.into_inner().unwrap().into_inner()
    };
    std::fs::write("ref_install.msi", &ref_msi).unwrap();
    println!("Reference MSI: {} bytes", ref_msi.len());

    // ═══ VELOCITY MSI ═══
    let velocity_msi = {
        let content = b"Hello from Velocity MSI!\r\n";
        let mut builder = MsiBuilder::new();
        builder.set_title("Velocity Test");
        builder.set_author("Velocity");
        builder.set_template("Intel", 1033);

        builder.create_table("Property", vec![
            VColumn::build("Property").string(72).primary_key().build(),
            VColumn::build("Value").string(1024).build(),
        ]).unwrap();
        builder.insert_rows("Property", vec![
            vec![VValue::from("ProductName"), VValue::from("Velocity Test")],
            vec![VValue::from("ProductVersion"), VValue::from("1.0.0")],
            vec![VValue::from("Manufacturer"), VValue::from("Velocity Corp")],
            vec![VValue::from("ProductCode"), VValue::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
            vec![VValue::from("UpgradeCode"), VValue::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
            vec![VValue::from("ProductLanguage"), VValue::from("1033")],
        ]).unwrap();

        builder.create_table("Directory", vec![
            VColumn::build("Directory").string(72).primary_key().build(),
            VColumn::build("Directory_Parent").string(72).nullable().build(),
            VColumn::build("DefaultDir").string(255).primary_key().build(),
        ]).unwrap();
        builder.insert_rows("Directory", vec![
            vec![VValue::from("TARGETDIR"), VValue::Null, VValue::from("SourceDir")],
            vec![VValue::from("INSTALLDIR"), VValue::from("TARGETDIR"), VValue::from("VelTest:VelTest")],
        ]).unwrap();

        builder.create_table("Component", vec![
            VColumn::build("Component").string(72).primary_key().build(),
            VColumn::build("ComponentId").string(38).nullable().build(),
            VColumn::build("Directory_").string(72).build(),
            VColumn::build("Attributes").int16().build(),
            VColumn::build("Condition").string(255).nullable().build(),
            VColumn::build("KeyPath").string(72).nullable().build(),
        ]).unwrap();
        builder.insert_rows("Component", vec![
            vec![VValue::from("MainComp"), VValue::Null, VValue::from("INSTALLDIR"),
                 VValue::Int(0), VValue::Null, VValue::from("hello.txt")],
        ]).unwrap();

        builder.create_table("File", vec![
            VColumn::build("File").string(72).primary_key().build(),
            VColumn::build("Component_").string(72).build(),
            VColumn::build("FileName").string(255).build(),
            VColumn::build("FileSize").int32().build(),
            VColumn::build("Version").string(72).nullable().build(),
            VColumn::build("Language").string(20).nullable().build(),
            VColumn::build("Attributes").int16().nullable().build(),
            VColumn::build("Sequence").int16().build(),
        ]).unwrap();
        builder.insert_rows("File", vec![
            vec![VValue::from("hello.txt"), VValue::from("MainComp"), VValue::from("hello.txt"),
                 VValue::Int(26), VValue::Null, VValue::Null, VValue::Null, VValue::Int(1)],
        ]).unwrap();

        builder.create_table("Feature", vec![
            VColumn::build("Feature").string(38).primary_key().build(),
            VColumn::build("Feature_Parent").string(38).nullable().build(),
            VColumn::build("Title").string(64).nullable().build(),
            VColumn::build("Description").string(255).nullable().build(),
            VColumn::build("Display").int16().nullable().build(),
            VColumn::build("Level").int16().build(),
            VColumn::build("Directory_").string(72).nullable().build(),
            VColumn::build("Attributes").int16().nullable().build(),
        ]).unwrap();
        builder.insert_rows("Feature", vec![
            vec![VValue::from("Complete"), VValue::Null, VValue::from("Complete Install"),
                 VValue::from("Installs all files"), VValue::Int(1), VValue::Int(1),
                 VValue::Null, VValue::Null],
        ]).unwrap();

        builder.create_table("FeatureComponents", vec![
            VColumn::build("Feature_").string(38).primary_key().build(),
            VColumn::build("Component_").string(72).primary_key().build(),
        ]).unwrap();
        builder.insert_rows("FeatureComponents", vec![
            vec![VValue::from("Complete"), VValue::from("MainComp")],
        ]).unwrap();

        builder.create_table("Media", vec![
            VColumn::build("DiskId").int16().primary_key().build(),
            VColumn::build("LastSequence").int16().build(),
            VColumn::build("DiskPrompt").string(64).nullable().build(),
            VColumn::build("Cabinet").string(255).nullable().build(),
            VColumn::build("VolumeLabel").string(32).nullable().build(),
            VColumn::build("Source").string(72).nullable().build(),
        ]).unwrap();
        let cab = build_cabinet(&[CabinetFile {
            name: "hello.txt".to_string(),
            data: content.to_vec(),
        }]);
        builder.insert_rows("Media", vec![
            vec![VValue::Int(1), VValue::Int(1), VValue::Null,
                 VValue::from("#vel.cab"), VValue::Null, VValue::Null],
        ]).unwrap();
        builder.add_stream("vel.cab".to_string(), cab);

        builder.create_table("InstallExecuteSequence", vec![
            VColumn::build("Action").string(72).primary_key().build(),
            VColumn::build("Condition").string(255).nullable().build(),
            VColumn::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        builder.insert_rows("InstallExecuteSequence", vec![
            vec![VValue::from("CostFinalize"), VValue::Null, VValue::Int(1000)],
            vec![VValue::from("CostInitialize"), VValue::Null, VValue::Int(800)],
            vec![VValue::from("FileCost"), VValue::Null, VValue::Int(900)],
            vec![VValue::from("InstallFiles"), VValue::Null, VValue::Int(4000)],
            vec![VValue::from("InstallFinalize"), VValue::Null, VValue::Int(6600)],
            vec![VValue::from("InstallInitialize"), VValue::Null, VValue::Int(1500)],
            vec![VValue::from("InstallValidate"), VValue::Null, VValue::Int(1400)],
        ]).unwrap();

        builder.create_table("InstallUISequence", vec![
            VColumn::build("Action").string(72).primary_key().build(),
            VColumn::build("Condition").string(255).nullable().build(),
            VColumn::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        builder.insert_rows("InstallUISequence", vec![
            vec![VValue::from("CostFinalize"), VValue::Null, VValue::Int(1000)],
            vec![VValue::from("CostInitialize"), VValue::Null, VValue::Int(800)],
            vec![VValue::from("ExecuteAction"), VValue::Null, VValue::Int(1300)],
            vec![VValue::from("FileCost"), VValue::Null, VValue::Int(900)],
        ]).unwrap();

        builder.build().unwrap()
    };
    std::fs::write("velocity_install_test.msi", &velocity_msi).unwrap();
    println!("Velocity MSI: {} bytes", velocity_msi.len());

    // ═══ COMPARE STREAMS ═══
    println!("\n=== Stream Comparison ===");
    let mut ref_cfb = cfb::CompoundFile::open(std::io::Cursor::new(&ref_msi)).unwrap();
    let mut vel_cfb = cfb::CompoundFile::open(std::io::Cursor::new(&velocity_msi)).unwrap();

    use std::io::Read;
    fn read_all(cfb: &mut cfb::CompoundFile<std::io::Cursor<&Vec<u8>>>, path: &str) -> Vec<u8> {
        let mut stream = cfb.open_stream(path).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        data
    }

    let ref_streams: std::collections::HashSet<String> = ref_cfb.walk()
        .filter(|e: &cfb::Entry| e.is_stream())
        .map(|e: cfb::Entry| e.path().to_string_lossy().to_string())
        .collect();
    let vel_streams: std::collections::HashSet<String> = vel_cfb.walk()
        .filter(|e: &cfb::Entry| e.is_stream())
        .map(|e: cfb::Entry| e.path().to_string_lossy().to_string())
        .collect();

    for s in ref_streams.difference(&vel_streams) {
        let data = read_all(&mut ref_cfb, s);
        println!("ONLY IN REF: {} ({} bytes)", s, data.len());
    }
    for s in vel_streams.difference(&ref_streams) {
        let data = read_all(&mut vel_cfb, s);
        println!("ONLY IN VEL: {} ({} bytes)", s, data.len());
    }
    let mut common: Vec<_> = ref_streams.intersection(&vel_streams).collect();
    common.sort();
    for s in &common {
        let ref_data = read_all(&mut ref_cfb, s);
        let vel_data = read_all(&mut vel_cfb, s);
        if ref_data == vel_data {
            println!("MATCH: {} ({} bytes)", s, ref_data.len());
        } else {
            println!("DIFF: {} (ref={} bytes, vel={} bytes)", s, ref_data.len(), vel_data.len());
            let min_len = ref_data.len().min(vel_data.len());
            for i in 0..min_len {
                if ref_data[i] != vel_data[i] {
                    println!("  first diff at byte {}: ref=0x{:02x}, vel=0x{:02x}", i, ref_data[i], vel_data[i]);
                    let start = i.saturating_sub(8);
                    let end = (i + 32).min(min_len);
                    println!("  ref[{}..{}]: {:02x?}", start, end, &ref_data[start..end]);
                    println!("  vel[{}..{}]: {:02x?}", start, end, &vel_data[start..end]);
                    break;
                }
            }
            if ref_data.len() != vel_data.len() {
                println!("  LENGTH DIFFERS!");
            }
        }
    }

    // ═══ TEST REFERENCE MSI ═══
    println!("\n=== Testing reference MSI ===");
    let _ = std::fs::remove_dir_all("C:\\VelTest");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", "ref_install.msi", "/qn", "/l*v", "ref_install_log.txt"])
        .output().unwrap();
    println!("ref msiexec exit: {}", output.status.code().unwrap_or(-1));
    if std::path::Path::new("C:\\VelTest\\hello.txt").exists() {
        println!("REF: File INSTALLED!");
    } else {
        println!("REF: File NOT installed");
        if let Ok(log) = std::fs::read_to_string("ref_install_log.txt") {
            for line in log.lines() {
                if (line.contains("Error ") && !line.contains("Error 0")) || line.contains("return value 3") {
                    println!("  {}", line.trim());
                }
            }
        }
    }
}
