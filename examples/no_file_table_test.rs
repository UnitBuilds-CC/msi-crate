/// Test: does File table cause 1603?
use velocity_msi::{Column, MsiBuilder, Value};

fn make_uuid() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let a = (t & 0xFFFFFFFF) as u32;
    let b = ((t >> 32) & 0xFFFF) as u16;
    let c = (((t >> 48) & 0x0FFF) as u16) | 0x4000;
    let d = ((t >> 64) as u16 & 0x3FFF) | 0x8000;
    let e = (t >> 80) as u64;
    format!("{{{:08X}-{:04X}-{:04X}-{:04X}-{:012X}}}", a, b, c, d, e & 0xFFFFFFFFFFFF)
}

fn main() {
    println!("=== NO FILE TABLE TEST ===\n");
    
    let mut b = MsiBuilder::new();
    b.set_title("Test");
    b.set_author("V");
    b.set_subject("Test");
    b.set_template("Intel", 1033);
    
    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    
    let pc = make_uuid();
    let uc = make_uuid();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("V")],
        vec![Value::from("ProductCode"), Value::from(pc.as_str())],
        vec![Value::from("UpgradeCode"), Value::from(uc.as_str())],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();
    
    b.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("ProgramFilesFolder"), Value::from("TARGETDIR"), Value::from("PFiles")],
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFilesFolder"), Value::from("Test")],
    ]).unwrap();
    
    // Component with KeyPath = file_0 (but no File table!)
    b.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(38).nullable().build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
    b.insert_rows("Component", vec![
        vec![Value::from("comp_0"), Value::Null, Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::from("file_0")],
    ]).unwrap();
    
    b.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").string(38).nullable().build(),
        Column::build("Title").string(64).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().build(),
    ]).unwrap();
    b.insert_rows("Feature", vec![
        vec![Value::from("Complete"), Value::Null, Value::from("Setup"), Value::Null, Value::Int(1), Value::Int(1), Value::from("INSTALLDIR"), Value::Int(0)],
    ]).unwrap();
    
    b.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    b.insert_rows("FeatureComponents", vec![
        vec![Value::from("Complete"), Value::from("comp_0")],
    ]).unwrap();
    
    // Media with cabinet reference
    b.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int16().build(),
        Column::build("DiskPrompt").string(64).nullable().build(),
        Column::build("Cabinet").string(255).nullable().build(),
        Column::build("VolumeLabel").string(32).nullable().build(),
        Column::build("Source").string(72).nullable().build(),
    ]).unwrap();
    b.insert_rows("Media", vec![
        vec![Value::Int(1), Value::Int(1), Value::Null, Value::from("#Test.cab"), Value::Null, Value::Null],
    ]).unwrap();
    
    b.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("InstallFinalize"), Value::Null, Value::Int(6600)],
    ]).unwrap();
    
    // NO File table!
    
    let fake_cab: Vec<u8> = (0..400).map(|i| (i % 256) as u8).collect();
    b.add_stream("Test.cab".to_string(), fake_cab);
    
    let data = b.build().unwrap();
    let path = "C:\\temp\\no_file_table.msi";
    std::fs::write(path, &data).unwrap();
    println!("Created: {} bytes", data.len());
    
    let output = std::process::Command::new("msiexec")
        .args(&["/i", path, "/qn", "/norestart"])
        .output().unwrap();
    let code = output.status.code().unwrap_or(-1);
    println!("msiexec exit code: {}", code);
    
    if code == 0 {
        println!("SUCCESS - no File table works!");
        let _ = std::process::Command::new("msiexec")
            .args(&["/x", path, "/qn", "/norestart"]).output();
    } else {
        println!("FAILED: {} - Component KeyPath=file_0 without File table", code);
    }
}
