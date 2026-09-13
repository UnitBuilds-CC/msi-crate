use velocity_msi::*;

fn main() {
    let mut builder = MsiBuilder::new();
    builder.set_title("Minimal Test");
    builder.set_author("Test");
    builder.set_template("Intel", 1033);

    // Property table (required)
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(1024).build(),
    ]).unwrap();

    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Minimal Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Test Corp")],
        vec![Value::from("ProductCode"), Value::from("{12345678-1234-1234-1234-123456789012}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    // Just Directory table
    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().category("Identifier").build(),
        Column::build("Directory_Parent").string(72).nullable().category("Identifier").build(),
        Column::build("DefaultDir").string(255).primary_key().category("DefaultDir").build(),
    ]).unwrap();

    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("VelTest:VelTest")],
    ]).unwrap();

    // Component table
    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().category("Identifier").build(),
        Column::build("ComponentId").string(38).nullable().category("GUID").build(),
        Column::build("Directory_").string(72).category("Identifier").build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().category("Condition").build(),
        Column::build("KeyPath").string(72).nullable().category("Identifier").build(),
    ]).unwrap();

    builder.insert_rows("Component", vec![
        vec![Value::from("MainComp"), Value::Null, Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::Null],
    ]).unwrap();

    // Feature table
    builder.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().category("Identifier").build(),
        Column::build("Feature_Parent").string(38).nullable().category("Identifier").build(),
        Column::build("Title").string(64).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
    ]).unwrap();

    builder.insert_rows("Feature", vec![
        vec![Value::from("Complete"), Value::Null, Value::from("Complete Install"), Value::from("Installs all files"), Value::Null, Value::Int(1), Value::Null, Value::Null],
    ]).unwrap();

    // FeatureComponents table
    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().category("Identifier").build(),
        Column::build("Component_").string(72).primary_key().category("Identifier").build(),
    ]).unwrap();

    builder.insert_rows("FeatureComponents", vec![
        vec![Value::from("Complete"), Value::from("MainComp")],
    ]).unwrap();

    // File table
    builder.create_table("File", vec![
        Column::build("File").string(72).primary_key().category("Identifier").build(),
        Column::build("Component_").string(72).category("Identifier").build(),
        Column::build("FileName").string(255).category("Filename").build(),
        Column::build("FileSize").int32().build(),
        Column::build("Version").string(72).nullable().category("Version").build(),
        Column::build("Language").string(20).nullable().category("Language").build(),
        Column::build("Attributes").int16().nullable().build(),
        Column::build("Sequence").int16().build(),
    ]).unwrap();

    let content = b"Hello from Velocity MSI!\r\n";
    builder.insert_rows("File", vec![
        vec![
            Value::from("hello.txt"),
            Value::from("MainComp"),
            Value::from("hello.txt"),
            Value::Int(content.len() as i32),
            Value::Null, Value::Null, Value::Null,
            Value::Int(1),
        ],
    ]).unwrap();

    // Media table
    builder.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").string(20).build(),
        Column::build("Cabinet").string(255).nullable().category("Cabinet").build(),
        Column::build("VolumeLabel").string(32).nullable().build(),
        Column::build("DiskPrompt").string(64).nullable().build(),
    ]).unwrap();

    // Build cabinet
    let cab_files = vec![
        CabinetFile {
            name: "hello.txt".to_string(),
            data: content.to_vec(),
        },
    ];
    let cab_data = build_cabinet(&cab_files);

    builder.insert_rows("Media", vec![
        vec![Value::Int(1), Value::from("1"), Value::from("#vel.cab"), Value::Null, Value::Null],
    ]).unwrap();

    builder.add_stream("vel.cab".to_string(), cab_data);

    // InstallExecuteSequence table
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

    // InstallUISequence table - commented out for testing
    // builder.create_table("InstallUISequence", vec![
    //     Column::build("Action").string(72).primary_key().build(),
    //     Column::build("Condition").string(255).nullable().build(),
    //     Column::build("Sequence").int16().nullable().build(),
    // ]).unwrap();

    // builder.insert_rows("InstallUISequence", vec![
    //     vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
    //     vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
    //     vec![Value::from("ExecuteAction"), Value::Null, Value::Int(1300)],
    //     vec![Value::from("FileCost"), Value::Null, Value::Int(900)],
    // ]).unwrap();

    let msi_data = builder.build().unwrap();
    std::fs::write("minimal_test.msi", &msi_data).unwrap();
    println!("Minimal MSI written: {} bytes", msi_data.len());
}
