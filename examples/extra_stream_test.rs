/// Test: does adding an extra stream (cabinet) to velocity-msi cause 1620?
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
    println!("=== EXTRA STREAM TEST ===\n");
    
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
    ]).unwrap();
    
    b.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
    ]).unwrap();
    
    // Add a fake cabinet stream
    let fake_cab: Vec<u8> = (0..400).map(|i| (i % 256) as u8).collect();
    b.add_stream("Velocity.cab".to_string(), fake_cab);
    
    let data = b.build().unwrap();
    let path = "C:\\temp\\extra_stream_test.msi";
    std::fs::write(path, &data).unwrap();
    println!("Created: {} ({} bytes)", path, data.len());
    
    // List streams
    println!("\nStreams:");
    let comp = cfb::CompoundFile::open(std::io::Cursor::new(&data)).unwrap();
    for entry in comp.walk() {
        if entry.is_stream() {
            println!("  {} ({} bytes)", entry.name(), entry.len());
        }
    }
    
    // Test with msiexec
    let output = std::process::Command::new("msiexec")
        .args(&["/i", path, "/qn", "/norestart"])
        .output().unwrap();
    let code = output.status.code().unwrap_or(-1);
    println!("\nmsiexec exit code: {}", code);
    
    if code == 0 {
        println!("SUCCESS!");
        let _ = std::process::Command::new("msiexec")
            .args(&["/x", path, "/qn", "/norestart"]).output();
    } else {
        println!("FAILED");
    }
}
