/// Incrementally add tables to find which one breaks msiexec
/// cargo run --example incremental_msi_test
use std::io::{Cursor, Write};

fn main() {
    println!("=== INCREMENTAL MSI TABLE TEST ===\n");

    let template_path = "C:\\Program Files\\Microsoft Office\\root\\Integration\\C2RIntLoc.en-us.16.msi";
    if !std::path::Path::new(template_path).exists() {
        println!("Template MSI not found");
        return;
    }

    let test_dir = "C:\\temp\\velo_test_src";
    let _ = std::fs::create_dir_all(test_dir);
    std::fs::write(format!("{}\\hello.txt", test_dir), "Hello from Velocity!\n").unwrap();
    let cab_data = create_cabinet_data(test_dir, &["hello.txt"]);
    println!("Cabinet: {} bytes\n", cab_data.len());

    let template_data = std::fs::read(template_path).unwrap();
    let product_code = "A1B2C3D4-E5F6-7890-ABCD-EF1234567890";

    // Define table names in order of addition
    let table_names = [
        "Property", "Directory", "Component", "File",
        "Feature", "FeatureComponents", "Media",
        "InstallExecuteSequence", "InstallUISequence",
    ];

    // Build incrementally
    for end_idx in 1..=table_names.len() {
        let cursor = Cursor::new(template_data.clone());
        let mut pkg = msi::Package::open(cursor).unwrap();

        // Delete all user tables
        let user_tables: Vec<String> = pkg.tables()
            .map(|t| t.name().to_string())
            .filter(|n| !n.starts_with('_'))
            .collect();
        for tn in &user_tables {
            pkg.drop_table(tn).unwrap();
        }

        // Add tables up to end_idx
        let tables_to_add = &table_names[..end_idx];
        for &tname in tables_to_add {
            add_table(&mut pkg, tname, test_dir, product_code);
        }

        // Summary info
        pkg.summary_info_mut().set_title("Velocity Test");
        pkg.summary_info_mut().set_author("Velocity");
        pkg.summary_info_mut().set_uuid(uuid::Uuid::parse_str(product_code).unwrap());
        pkg.summary_info_mut().set_word_count(2);

        // Save
        pkg.flush().unwrap();
        let cursor = pkg.into_inner().unwrap();
        let msi_data = cursor.into_inner();

        // Write and embed cabinet
        let out_path = "C:\\temp\\incr_test.msi";
        std::fs::write(out_path, &msi_data).unwrap();
        {
            let file = std::fs::OpenOptions::new()
                .read(true).write(true).open(out_path).unwrap();
            let mut comp = cfb::CompoundFile::open(file).unwrap();
            let _ = comp.remove_stream("velo_data.cab");
            let mut s = comp.create_stream("velo_data.cab").unwrap();
            s.write_all(&cab_data).unwrap();
            drop(s);
            comp.flush().unwrap();
        }

        // Clean previous install
        let _ = std::process::Command::new("msiexec")
            .args(&["/x", &format!("{{{}}}", product_code), "/qn"]).output();
        std::thread::sleep(std::time::Duration::from_secs(1));
        let _ = std::fs::remove_dir_all("C:\\Program Files\\VelocityTest");

        // Test
        let output = std::process::Command::new("msiexec")
            .args(&["/i", out_path, "/qn", "/l*v", "C:\\temp\\incr_test.log"])
            .output().expect("msiexec failed");
        let exit_code = output.status.code().unwrap_or(-1);

        let label = tables_to_add.join(" + ");
        let status = match exit_code {
            0 => "SUCCESS",
            1603 => "1603 (fatal error)",
            1605 => "1605 (data readable)",
            1613 => "1613 (invalid pkg)",
            1620 => "1620 (cannot open)",
            _ => "unknown",
        };
        println!("[{}] -> exit {} ({})", label, exit_code, status);

        if exit_code == 0 {
            if std::path::Path::new("C:\\Program Files\\VelocityTest\\hello.txt").exists() {
                println!("  FILE INSTALLED!");
            }
        }

        if exit_code != 0 && exit_code != 1603 && exit_code != 1605 {
            // Read log
            if let Ok(log) = std::fs::read_to_string("C:\\temp\\incr_test.log") {
                for line in log.lines() {
                    if line.contains("Error") || line.contains("return value 3") {
                        println!("  LOG: {}", line);
                    }
                }
            }
            println!("\n  STOPPING - this table broke it.");
            break;
        }
    }

    println!("\n=== DONE ===");
}

fn add_table(pkg: &mut msi::Package<Cursor<Vec<u8>>>, name: &str, test_dir: &str, product_code: &str) {
    match name {
        "Property" => {
            let cols = vec![
                msi::Column::build("Property").primary_key().string(72),
                msi::Column::build("Value").nullable().localizable().string(255),
            ];
            pkg.create_table("Property".to_string(), cols).unwrap();
            pkg.insert_rows(msi::Insert::into("Property").rows(vec![
                vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Velocity Test".into())],
                vec![msi::Value::Str("ProductCode".into()), msi::Value::Str(format!("{{{}}}", product_code))],
                vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())],
                vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Velocity".into())],
                vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())],
            ])).unwrap();
        }
        "Directory" => {
            let cols = vec![
                msi::Column::build("Directory").primary_key().string(72),
                msi::Column::build("Directory_Parent").nullable().string(72),
                msi::Column::build("DefaultDir").localizable().string(255),
            ];
            pkg.create_table("Directory".to_string(), cols).unwrap();
            pkg.insert_rows(msi::Insert::into("Directory").rows(vec![
                vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str(".".into())],
                vec![msi::Value::Str("ProgramFilesFolder".into()), msi::Value::Str("TARGETDIR".into()), msi::Value::Str("PFiles".into())],
                vec![msi::Value::Str("INSTALLDIR".into()), msi::Value::Str("ProgramFilesFolder".into()), msi::Value::Str("VelocityTest".into())],
            ])).unwrap();
        }
        "Component" => {
            let cols = vec![
                msi::Column::build("Component").primary_key().string(72),
                msi::Column::build("ComponentId").nullable().string(38),
                msi::Column::build("Directory_").string(72),
                msi::Column::build("Attributes").nullable().int16(),
                msi::Column::build("Condition").nullable().string(255),
                msi::Column::build("KeyPath").nullable().string(72),
            ];
            pkg.create_table("Component".to_string(), cols).unwrap();
            pkg.insert_rows(msi::Insert::into("Component").rows(vec![
                vec![
                    msi::Value::Str("MainComponent".into()),
                    msi::Value::Str("{C3D4E5F6-A7B8-9012-CDEF-123456789012}".into()),
                    msi::Value::Str("INSTALLDIR".into()),
                    msi::Value::Int(0), msi::Value::Null,
                    msi::Value::Str("hello.txt".into()),
                ],
            ])).unwrap();
        }
        "File" => {
            let cols = vec![
                msi::Column::build("File").primary_key().string(72),
                msi::Column::build("Component_").string(72),
                msi::Column::build("FileName").localizable().string(255),
                msi::Column::build("FileSize").int32(),
                msi::Column::build("Attributes").nullable().int16(),
                msi::Column::build("Sequence").int16(),
            ];
            pkg.create_table("File".to_string(), cols).unwrap();
            let sz = std::fs::metadata(format!("{}\\hello.txt", test_dir)).unwrap().len() as i32;
            pkg.insert_rows(msi::Insert::into("File").rows(vec![
                vec![
                    msi::Value::Str("hello.txt".into()),
                    msi::Value::Str("MainComponent".into()),
                    msi::Value::Str("hello.txt".into()),
                    msi::Value::Int(sz), msi::Value::Int(0), msi::Value::Int(1),
                ],
            ])).unwrap();
        }
        "Feature" => {
            let cols = vec![
                msi::Column::build("Feature").primary_key().string(38),
                msi::Column::build("Feature_Parent").nullable().string(38),
                msi::Column::build("Title").nullable().localizable().string(64),
                msi::Column::build("Description").nullable().localizable().string(255),
                msi::Column::build("Display").nullable().int16(),
                msi::Column::build("Level").int16(),
                msi::Column::build("Directory_").nullable().string(72),
                msi::Column::build("Attributes").nullable().int16(),
            ];
            pkg.create_table("Feature".to_string(), cols).unwrap();
            pkg.insert_rows(msi::Insert::into("Feature").rows(vec![
                vec![
                    msi::Value::Str("Complete".into()), msi::Value::Null,
                    msi::Value::Str("Complete".into()),
                    msi::Value::Str("All features".into()),
                    msi::Value::Null, msi::Value::Int(1),
                    msi::Value::Str("INSTALLDIR".into()), msi::Value::Null,
                ],
            ])).unwrap();
        }
        "FeatureComponents" => {
            let cols = vec![
                msi::Column::build("Feature_").primary_key().string(38),
                msi::Column::build("Component_").primary_key().string(72),
            ];
            pkg.create_table("FeatureComponents".to_string(), cols).unwrap();
            pkg.insert_rows(msi::Insert::into("FeatureComponents").rows(vec![
                vec![msi::Value::Str("Complete".into()), msi::Value::Str("MainComponent".into())],
            ])).unwrap();
        }
        "Media" => {
            let cols = vec![
                msi::Column::build("DiskId").primary_key().int16(),
                msi::Column::build("LastSequence").int16(),
                msi::Column::build("Cabinet").nullable().string(255),
                msi::Column::build("VolumeLabel").nullable().string(32),
                msi::Column::build("Source").nullable().string(72),
            ];
            pkg.create_table("Media".to_string(), cols).unwrap();
            pkg.insert_rows(msi::Insert::into("Media").rows(vec![
                vec![
                    msi::Value::Int(1), msi::Value::Int(1),
                    msi::Value::Str("#velo_data.cab".into()),
                    msi::Value::Null, msi::Value::Null,
                ],
            ])).unwrap();
        }
        "InstallExecuteSequence" => {
            let cols = vec![
                msi::Column::build("Action").primary_key().string(72),
                msi::Column::build("Condition").nullable().string(255),
                msi::Column::build("Sequence").nullable().int16(),
            ];
            pkg.create_table("InstallExecuteSequence".to_string(), cols).unwrap();
            pkg.insert_rows(msi::Insert::into("InstallExecuteSequence").rows(vec![
                vec![msi::Value::Str("LaunchConditions".into()), msi::Value::Null, msi::Value::Int(100)],
                vec![msi::Value::Str("ValidateProductID".into()), msi::Value::Null, msi::Value::Int(700)],
                vec![msi::Value::Str("CostFinalize".into()), msi::Value::Null, msi::Value::Int(1000)],
                vec![msi::Value::Str("InstallValidate".into()), msi::Value::Null, msi::Value::Int(1400)],
                vec![msi::Value::Str("InstallInitialize".into()), msi::Value::Null, msi::Value::Int(1500)],
                vec![msi::Value::Str("ProcessComponents".into()), msi::Value::Null, msi::Value::Int(1600)],
                vec![msi::Value::Str("RegisterProduct".into()), msi::Value::Null, msi::Value::Int(5700)],
                vec![msi::Value::Str("PublishFeatures".into()), msi::Value::Null, msi::Value::Int(6300)],
                vec![msi::Value::Str("PublishProduct".into()), msi::Value::Null, msi::Value::Int(6400)],
                vec![msi::Value::Str("InstallFinalize".into()), msi::Value::Null, msi::Value::Int(6600)],
            ])).unwrap();
        }
        "InstallUISequence" => {
            let cols = vec![
                msi::Column::build("Action").primary_key().string(72),
                msi::Column::build("Condition").nullable().string(255),
                msi::Column::build("Sequence").nullable().int16(),
            ];
            pkg.create_table("InstallUISequence".to_string(), cols).unwrap();
            pkg.insert_rows(msi::Insert::into("InstallUISequence").rows(vec![
                vec![msi::Value::Str("LaunchConditions".into()), msi::Value::Null, msi::Value::Int(100)],
                vec![msi::Value::Str("ValidateProductID".into()), msi::Value::Null, msi::Value::Int(700)],
                vec![msi::Value::Str("CostFinalize".into()), msi::Value::Null, msi::Value::Int(1000)],
            ])).unwrap();
        }
        _ => panic!("Unknown table: {}", name),
    }
}

fn create_cabinet_data(source_dir: &str, files: &[&str]) -> Vec<u8> {
    let ddf_path = "C:\\temp\\velo_incr.ddf";
    let mut ddf = String::new();
    ddf.push_str(".Set CabinetName1=velo_incr.cab\n");
    ddf.push_str(".Set DiskDirectory1=C:\\temp\n");
    ddf.push_str(".Set CompressionType=MSZIP\n");
    ddf.push_str(".Set Compress=ON\n");
    for file in files {
        ddf.push_str(&format!("\"{}\\{}\"\n", source_dir, file));
    }
    std::fs::write(ddf_path, &ddf).unwrap();
    let output = std::process::Command::new("makecab")
        .args(&["/f", ddf_path]).output().expect("makecab failed");
    if !output.status.success() { panic!("makecab failed"); }
    let _ = std::fs::remove_file(ddf_path);
    let _ = std::fs::remove_file("C:\\temp\\setup.inf");
    let _ = std::fs::remove_file("C:\\temp\\setup.rpt");
    std::fs::read("C:\\temp\\velo_incr.cab").unwrap()
}
