/// Progressive test: add tables one by one to find which breaks msiexec.
use velocity_msi::{MsiBuilder, Column, Value, CabinetFile, build_cabinet};

fn build_and_test(label: &str, build_fn: impl FnOnce(&mut MsiBuilder)) {
    let mut builder = MsiBuilder::new();
    builder.set_title("Progressive Test");
    builder.set_author("Velocity");
    builder.set_template("Intel", 1033);

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(1024).build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Progressive Test")],
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

    build_fn(&mut builder);

    let fname = format!("prog_{}.msi", label.to_lowercase().replace(' ', "_"));
    let msi = builder.build().unwrap();
    std::fs::write(&fname, &msi).unwrap();

    let _ = std::fs::remove_dir_all("C:\\VelTest");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", &fname, "/qn", "/l*v", &format!("prog_{}.log", label.to_lowercase().replace(' ', "_"))])
        .output().unwrap();
    let exit = output.status.code().unwrap_or(-1);
    let installed = std::path::Path::new("C:\\VelTest").exists();
    println!("{:40} exit={} installed={}", label, exit, installed);

    if exit != 0 {
        let log_name = format!("prog_{}.log", label.to_lowercase().replace(' ', "_"));
        if let Ok(log) = std::fs::read_to_string(&log_name) {
            for line in log.lines() {
                if (line.contains("Error ") && !line.contains("Error 0") && !line.contains("2205") && !line.contains("2228"))
                    || line.contains("return value 3")
                {
                    println!("  {}", line.trim());
                }
            }
        }
    }
}

fn add_component(m: &mut MsiBuilder) {
    m.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(38).nullable().build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
    m.insert_rows("Component", vec![
        vec![Value::from("MainComp"), Value::Null, Value::from("INSTALLDIR"),
             Value::Int(0), Value::Null, Value::from("hello.txt")],
    ]).unwrap();
}

fn add_file(m: &mut MsiBuilder) {
    let content = b"Hello from Velocity MSI!\r\n";
    m.create_table("File", vec![
        Column::build("File").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).build(),
        Column::build("FileSize").int32().build(),
        Column::build("Version").string(72).nullable().build(),
        Column::build("Language").string(20).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
        Column::build("Sequence").int16().build(),
    ]).unwrap();
    m.insert_rows("File", vec![
        vec![Value::from("hello.txt"), Value::from("MainComp"), Value::from("hello.txt"),
             Value::Int(content.len() as i32),
             Value::Null, Value::Null, Value::Null, Value::Int(1)],
    ]).unwrap();
}

fn add_feature(m: &mut MsiBuilder) {
    m.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").string(38).nullable().build(),
        Column::build("Title").string(64).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
    ]).unwrap();
    m.insert_rows("Feature", vec![
        vec![Value::from("Complete"), Value::Null, Value::from("Complete"),
             Value::from("All files"), Value::Int(1), Value::Int(1),
             Value::Null, Value::Null],
    ]).unwrap();

    m.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    m.insert_rows("FeatureComponents", vec![
        vec![Value::from("Complete"), Value::from("MainComp")],
    ]).unwrap();
}

fn add_media_cab(m: &mut MsiBuilder) {
    let content = b"Hello from Velocity MSI!\r\n";
    let cab = build_cabinet(&[CabinetFile {
        name: "hello.txt".to_string(),
        data: content.to_vec(),
    }]);
    m.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int16().build(),
        Column::build("DiskPrompt").string(64).nullable().build(),
        Column::build("Cabinet").string(255).nullable().build(),
        Column::build("VolumeLabel").string(32).nullable().build(),
        Column::build("Source").string(72).nullable().build(),
    ]).unwrap();
    m.insert_rows("Media", vec![
        vec![Value::Int(1), Value::Int(1), Value::Null,
             Value::from("#vel.cab"), Value::Null, Value::Null],
    ]).unwrap();
    m.add_stream("vel.cab".to_string(), cab);
}

fn add_exec_seq(m: &mut MsiBuilder) {
    m.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    m.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("FileCost"), Value::Null, Value::Int(900)],
        vec![Value::from("InstallFiles"), Value::Null, Value::Int(4000)],
        vec![Value::from("InstallFinalize"), Value::Null, Value::Int(6600)],
        vec![Value::from("InstallInitialize"), Value::Null, Value::Int(1500)],
        vec![Value::from("InstallValidate"), Value::Null, Value::Int(1400)],
    ]).unwrap();
}

fn main() {
    println!("=== Progressive MSI table test ===\n");

    build_and_test("Prop_Dir", |_m| {});
    build_and_test("Prop_Dir_Comp", |m| { add_component(m); });
    build_and_test("Prop_Dir_Comp_File", |m| { add_component(m); add_file(m); });
    build_and_test("With_Feature", |m| { add_component(m); add_file(m); add_feature(m); });
    build_and_test("With_Media", |m| { add_component(m); add_file(m); add_feature(m); add_media_cab(m); });
    build_and_test("With_ExecSeq", |m| { add_component(m); add_file(m); add_feature(m); add_media_cab(m); add_exec_seq(m); });

    println!("\nDone.");
}
