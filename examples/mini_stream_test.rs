/// Test: Does the mini-stream cause msiexec error 2705?
///
/// Builds the same MSI twice:
/// 1. Normal (uses mini-stream for small table streams)
/// 2. No mini-stream (all streams use regular sectors)
///
/// If #1 fails but #2 succeeds, the mini-stream implementation is the bug.
use velocity_msi::{MsiBuilder, Column, Value};
use std::process::Command;

fn build_failing_msi() -> Vec<u8> {
    let mut b = MsiBuilder::new();
    b.set_title("Mini Stream Test");
    b.set_author("Velocity");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(1024).build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("MiniStream Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity Corp")],
        vec![Value::from("ProductCode"), Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![Value::from("UpgradeCode"), Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    b.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("VelTest:VelTest")],
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

    b.build().unwrap()
}

fn test_msi(data: &[u8], label: &str) -> i32 {
    let path = format!("{}.msi", label);
    std::fs::write(&path, data).unwrap();
    println!("\n=== {} ===", label);
    println!("File size: {} bytes", data.len());

    // Verify with msi crate
    let file = std::fs::File::open(&path).unwrap();
    match msi::Package::open(file) {
        Ok(pkg) => {
            println!("msi crate: opened OK, {} tables", pkg.tables().count());
        }
        Err(e) => {
            println!("msi crate: FAILED to open: {}", e);
        }
    }

    // Test with msiexec
    let output = Command::new("msiexec")
        .args(&["/i", &path, "/qn", "/norestart"])
        .output()
        .expect("Failed to run msiexec");
    let code = output.status.code().unwrap_or(-1);
    println!("msiexec exit code: {}", code);

    // Try to uninstall if install succeeded
    if code == 0 {
        let _ = Command::new("msiexec")
            .args(&["/x", &path, "/qn", "/norestart"])
            .output();
    }

    code
}

fn main() {
    println!("=== Mini-stream diagnostic test ===");
    println!("Testing if the OLE mini-stream causes error 2705\n");

    // Test 1: Normal MSI (uses mini-stream)
    let msi_data = build_failing_msi();
    let code_normal = test_msi(&msi_data, "mini_stream_normal");

    // Test 2: Force all streams to be large (no mini-stream)
    // We do this by modifying the OLE file after building:
    // Read our MSI, extract all streams, rebuild with cutoff=0
    println!("\n\n=== Attempting no-mini-stream rebuild ===");

    // Open with msi crate, read all table data
    let temp_path = "mini_stream_normal.msi";
    let file = std::fs::File::open(temp_path).unwrap();
    let mut package = msi::Package::open_rw(file).unwrap();

    // Collect all table info
    let table_names: Vec<String> = package.tables().map(|t| {
        let name = t.name().to_string();
        let cols: Vec<String> = t.columns().iter().map(|c| {
            let type_str = match c.coltype() {
                msi::ColumnType::Int16 => "int16".to_string(),
                msi::ColumnType::Int32 => "int32".to_string(),
                msi::ColumnType::Str(n) => format!("str{}", n),
            };
            let flags = if c.is_nullable() { " nullable" } else { "" };
            let pk = if c.is_primary_key() { " pk" } else { "" };
            format!("{}:{}{}{}", c.name(), type_str, flags, pk)
        }).collect();
        println!("  Table {}: {} cols [{}]", name, cols.len(), cols.join(", "));
        name
    }).collect();

    // Read all rows from each table
    println!("\n--- Table contents ---");
    for tname in &table_names {
        let col_names: Vec<&str> = {
            for table in package.tables() {
                if table.name() == tname.as_str() {
                    break;
                }
            }
            // Need to get column names from the table
            let mut names = Vec::new();
            for table in package.tables() {
                if table.name() == tname.as_str() {
                    for col in table.columns() {
                        names.push(col.name());
                    }
                    break;
                }
            }
            names
        };

        match package.select_rows(msi::Select::table(tname.as_str()).columns(&col_names)) {
            Ok(rows) => {
                let count = rows.count();
                println!("  {}: {} rows", tname, count);
            }
            Err(e) => {
                println!("  {}: ERROR reading: {}", tname, e);
            }
        }
    }

    // Now flush to a new file - this rewrites through the msi crate's cfb path
    println!("\n--- Flushing through msi crate ---");
    package.flush().unwrap();
    drop(package);

    // Read the flushed file
    let flushed_data = std::fs::read(temp_path).unwrap();
    println!("Flushed file size: {} bytes (original: {} bytes)", flushed_data.len(), msi_data.len());

    // Test the flushed version
    let code_flushed = test_msi(&flushed_data, "mini_stream_flushed");

    // Summary
    println!("\n\n=== SUMMARY ===");
    println!("Normal (custom OLE):    exit code {}", code_normal);
    println!("Flushed (msi crate):    exit code {}", code_flushed);

    if code_normal != 0 && code_flushed == 0 {
        println!("\n>>> CONCLUSION: Custom OLE writer has a bug!");
        println!(">>> The msi crate's cfb flush fixes it.");
    } else if code_normal != 0 && code_flushed != 0 {
        println!("\n>>> CONCLUSION: The DATA is the problem, not the OLE writer.");
        println!(">>> Even msi crate flush can't fix it.");
    } else if code_normal == 0 {
        println!("\n>>> CONCLUSION: Normal MSI works! Issue may be fixed.");
    }
}
