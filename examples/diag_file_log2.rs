/// Get detailed msiexec log for File table MSI
/// cargo run --example diag_file_log2 -p velocity-msi
use velocity_msi::{Column, MsiBuilder, Value};

fn make_uuid() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!(
        "{{{:08X}-{:04X}-4{:03X}-{:04X}-{:08X}{:04X}}}",
        (t & 0xFFFFFFFF) as u32,
        ((t >> 32) & 0xFFFF) as u16,
        ((t >> 48) & 0x0FFF) as u16,
        (((t >> 44) & 0x0FFF) as u16) | 0x8000,
        ((t >> 16) & 0xFFFFFFFF) as u32,
        (t & 0xFFFF) as u16,
    )
}

fn main() {
    let pc = make_uuid();
    let uc = make_uuid();
    let mut b = MsiBuilder::new();
    b.set_title("Test");
    b.set_author("V");
    b.set_template("Intel", 1033);
    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).localizable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("V")],
        vec![Value::from("ProductCode"), Value::from(pc.as_str())],
        vec![Value::from("UpgradeCode"), Value::from(uc.as_str())],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();
    b.create_table("File", vec![
        Column::build("File_").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).localizable().build(),
        Column::build("FileSize").int32().build(),
        Column::build("Attributes").nullable().int16().build(),
        Column::build("Sequence").int16().build(),
    ]).unwrap();
    b.insert_rows("File", vec![
        vec![Value::from("MainFile"), Value::from("MainComp"),
             Value::from("testfile.txt"), Value::Int(23), Value::Int(0), Value::Int(1)],
    ]).unwrap();
    let data = b.build().unwrap();

    let _ = std::fs::create_dir_all("C:\\temp");
    let path = "C:\\temp\\test_file_detail.msi";
    let log_path = "C:\\temp\\test_file_detail.log";
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(log_path);
    std::fs::write(path, &data).unwrap();

    let output = std::process::Command::new("msiexec")
        .args(&["/i", path, "/qn", "/l*v", log_path])
        .output().unwrap();
    let ec = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", ec);

    // Print FULL log (last 100 lines)
    if let Ok(log) = std::fs::read_to_string(log_path) {
        let lines: Vec<&str> = log.lines().collect();
        let start = lines.len().saturating_sub(100);
        println!("\n=== Last 100 lines of log ===");
        for line in &lines[start..] {
            println!("{}", line);
        }

        // Also search for specific error patterns
        println!("\n=== Lines with 'File' table reference ===");
        for line in log.lines() {
            if line.contains("File") && (line.contains("table") || line.contains("stream") || line.contains("error") || line.contains("Error")) {
                println!("{}", line);
            }
        }

        println!("\n=== Lines with 'return value 3' ===");
        for line in log.lines() {
            if line.contains("return value 3") {
                println!("{}", line);
            }
        }
    }
}
