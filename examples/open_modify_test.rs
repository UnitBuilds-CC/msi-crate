/// Test: Open our MSI with msi crate, add a dummy table, flush.
/// This uses the msi crate's WRITE path (cfb crate) on our DATA.
/// If the result works → our OLE writer has a bug
/// If the result fails → our DATA has a bug
use std::process::Command;

fn main() {
    println!("=== Test: msi crate open + modify + flush ===\n");

    // Build our MSI with the failing config
    let mut b = velocity_msi::MsiBuilder::new();
    b.set_title("Open Modify Test");
    b.set_author("Velocity");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        velocity_msi::Column::build("Property").string(72).primary_key().build(),
        velocity_msi::Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("Open Modify Test")],
        vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0.0")],
        vec![velocity_msi::Value::from("Manufacturer"), velocity_msi::Value::from("Velocity Corp")],
        vec![velocity_msi::Value::from("ProductCode"), velocity_msi::Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![velocity_msi::Value::from("UpgradeCode"), velocity_msi::Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![velocity_msi::Value::from("ProductLanguage"), velocity_msi::Value::from("1033")],
    ]).unwrap();

    b.create_table("Directory", vec![
        velocity_msi::Column::build("Directory").string(72).primary_key().build(),
        velocity_msi::Column::build("Directory_Parent").string(72).nullable().build(),
        velocity_msi::Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::Null, velocity_msi::Value::from("SourceDir")],
        vec![velocity_msi::Value::from("INSTALLDIR"), velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::from("VelTest:VelTest")],
    ]).unwrap();

    b.create_table("InstallExecuteSequence", vec![
        velocity_msi::Column::build("Action").string(72).primary_key().build(),
        velocity_msi::Column::build("Condition").string(255).nullable().build(),
        velocity_msi::Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![velocity_msi::Value::from("CostInitialize"), velocity_msi::Value::Null, velocity_msi::Value::Int(800)],
        vec![velocity_msi::Value::from("CostFinalize"), velocity_msi::Value::Null, velocity_msi::Value::Int(1000)],
    ]).unwrap();

    let msi_data = b.build().unwrap();
    let our_path = "open_modify_test.msi";
    std::fs::write(our_path, &msi_data).unwrap();
    println!("Our MSI: {} bytes", msi_data.len());

    // Test 1: Our original MSI
    println!("\n--- Test 1: Original velocity-msi MSI ---");
    let code_our = test_msiexec(our_path);
    println!("Exit code: {}", code_our);

    // Test 2: Open with msi crate, add dummy table, flush
    println!("\n--- Test 2: Open + modify + flush via msi crate ---");
    {
        let file = std::fs::OpenOptions::new()
            .read(true).write(true)
            .open(our_path).unwrap();
        let mut package = msi::Package::open(file).unwrap();
        println!("Opened OK. Adding dummy table...");

        // Add a dummy table to force the msi crate to rewrite system tables
        package.create_table("DummyTable", vec![
            msi::Column::build("Dummy").primary_key().string(72),
        ]).unwrap();
        package.insert_rows(msi::Insert::into("DummyTable")
            .row(vec![msi::Value::Str("TestValue".into())])
        ).unwrap();

        // Drop the dummy table (removes it from _Tables, _Columns, _Validation)
        package.drop_table("DummyTable").unwrap();
        println!("Dummy table added and dropped (forces system table rewrite).");

        package.flush().unwrap();
        println!("Flushed.");
    }

    // Read the modified file
    let modified_data = std::fs::read(our_path).unwrap();
    println!("Modified MSI: {} bytes (was {} bytes)", modified_data.len(), msi_data.len());

    let code_modified = test_msiexec(our_path);
    println!("Exit code: {}", code_modified);

    // Summary
    println!("\n\n=== RESULTS ===");
    println!("Original:  exit code {}", code_our);
    println!("Modified:  exit code {}", code_modified);

    if code_our != 0 && code_modified == 0 {
        println!("\n>>> OLE WRITER BUG CONFIRMED!");
        println!(">>> msi crate's cfb flush fixes the OLE structure.");
    } else if code_our != 0 && code_modified != 0 {
        println!("\n>>> Both fail: DATA issue confirmed.");
        println!(">>> Even after msi crate rewrite, the MSI fails.");
    } else if code_our == 0 {
        println!("\n>>> Original works! Issue may be fixed.");
    }
}

fn test_msiexec(path: &str) -> i32 {
    let output = Command::new("msiexec")
        .args(&["/i", path, "/qn", "/norestart"])
        .output()
        .expect("msiexec failed");
    let code = output.status.code().unwrap_or(-1);
    if code == 0 {
        let _ = Command::new("msiexec")
            .args(&["/x", path, "/qn", "/norestart"])
            .output();
    }
    code
}
