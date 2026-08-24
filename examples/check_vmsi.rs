/// Quick test: build velocity-msi output and check CFB version
/// cargo run --example check_vmsi -p velocity-msi
fn main() {
    let mut builder = velocity_msi::MsiBuilder::new();
    builder.set_title("Test");
    builder.set_author("Test");

    builder.create_table("Property", vec![
        velocity_msi::Column::build("Property").primary_key().string(72).build(),
        velocity_msi::Column::build("Value").nullable().string(255).build(),
    ]).unwrap();

    builder.insert_rows("Property", vec![
        vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("Test")],
    ]).unwrap();

    let data = builder.build().unwrap();
    let path = "C:\\temp\\velocity_test.msi";
    std::fs::write(path, &data).unwrap();
    println!("Wrote {} ({} bytes)", path, data.len());

    // Check CFB version
    let ver = data[26] as u16 + (data[27] as u16) * 256;
    let sector_pow = data[30] as u16 + (data[31] as u16) * 256;
    println!("CFB Version: {} (sector size: {})", ver, 2u32.pow(sector_pow as u32));

    // Test with msiexec
    let _ = std::fs::remove_file("C:\\temp\\velocity_test.log");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", path, "/qn", "/l*v", "C:\\temp\\velocity_test.log"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("msiexec exit code: {}", exit_code);

    if let Ok(log) = std::fs::read_to_string("C:\\temp\\velocity_test.log") {
        for line in log.lines() {
            if line.contains("Error") || line.contains("2219") || line.contains("2203") ||
               line.contains("Product:") || line.contains("successful") {
                println!("  {}", line.trim());
            }
        }
    }
}
