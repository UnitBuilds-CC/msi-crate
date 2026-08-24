/// Test: MSI with Media table referencing #vel.cab but NO cabinet stream.
/// Does msiexec give a different error (stream not found) or same 2725?
use velocity_msi::{MsiBuilder, Column, Value};

fn main() {
    let mut builder = MsiBuilder::new();
    builder.set_title("No Stream Test");
    builder.set_author("Velocity");
    builder.set_template("x64", 1033);

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();

    let product_code = "{FFFFFFFF-0000-1111-2222-333333333333}";
    let upgrade_code = "{87654321-4321-4321-4321-CBA987654321}";

    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("No Stream Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity")],
        vec![Value::from("ProductCode"), Value::from(product_code)],
        vec![Value::from("UpgradeCode"), Value::from(upgrade_code)],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("TestDir:.")],
    ]).unwrap();

    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(38).nullable().build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Component", vec![
        vec![Value::from("Comp1"), Value::Null, Value::from("INSTALLDIR"),
             Value::Int(0), Value::Null, Value::from("hello.txt")],
    ]).unwrap();

    builder.create_table("File", vec![
        Column::build("File").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).build(),
        Column::build("FileSize").int32().build(),
        Column::build("Sequence").int16().build(),
    ]).unwrap();
    builder.insert_rows("File", vec![
        vec![Value::from("hello.txt"), Value::from("Comp1"), Value::from("hello.txt"),
             Value::Int(13), Value::Int(1)],
    ]).unwrap();

    builder.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").string(38).nullable().build(),
        Column::build("Title").string(64).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("Feature", vec![
        vec![Value::from("Feat1"), Value::Null, Value::from("Main"),
             Value::from("Main feature"), Value::Int(1), Value::Int(1),
             Value::from("INSTALLDIR"), Value::Null],
    ]).unwrap();

    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("FeatureComponents", vec![
        vec![Value::from("Feat1"), Value::from("Comp1")],
    ]).unwrap();

    builder.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int16().build(),
        Column::build("DiskPrompt").string(64).nullable().build(),
        Column::build("Cabinet").string(255).nullable().build(),
        Column::build("VolumeLabel").string(32).nullable().build(),
        Column::build("Source").string(72).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Media", vec![
        vec![Value::Int(1), Value::Int(1), Value::Null,
             Value::from("#vel.cab"), Value::Null, Value::Null],
    ]).unwrap();

    // NO cabinet stream added!
    
    let msi = builder.build().unwrap();
    std::fs::write("no_stream.msi", &msi).unwrap();
    println!("MSI: {} bytes", msi.len());

    // List streams to verify no cabinet
    let comp = cfb::CompoundFile::open(std::io::Cursor::new(&msi)).unwrap();
    println!("Streams:");
    for entry in comp.walk() {
        if entry.is_stream() {
            println!("  S {}", entry.path().to_string_lossy());
        }
    }

    let output = std::process::Command::new("msiexec")
        .args(&["/i", "no_stream.msi", "/qn", "/l*v", "no_stream_log.txt"])
        .output().unwrap();
    println!("Exit: {}", output.status.code().unwrap_or(-1));
    // Keep files for inspection
}
