/// Definitive test: Create the SAME MSI using the msi crate's Package::create()
/// and test with msiexec. If msi crate output works → our serialization is buggy.
/// If msi crate output also fails → the approach itself is broken.
use std::process::Command;

fn main() {
    println!("=== MSI CRATE REFERENCE TEST ===\n");

    let _ = Command::new("taskkill").args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Create MSI using the msi crate directly
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut pkg = msi::Package::create(msi::PackageType::Installer, &mut buf).unwrap();

        // Create Property table
        pkg.create_table("Property", vec![
            msi::Column::build("Property").primary_key().string(72),
            msi::Column::build("Value").nullable().string(255),
        ]).unwrap();

        // Insert property rows
        pkg.insert_rows(
            msi::Insert::into("Property")
                .row(vec![msi::Value::Str("ProductName".into()), msi::Value::Str("MsiRefTest".into())])
                .row(vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())])
                .row(vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("V".into())])
                .row(vec![msi::Value::Str("ProductCode".into()), msi::Value::Str("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}".into())])
                .row(vec![msi::Value::Str("UpgradeCode".into()), msi::Value::Str("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}".into())])
                .row(vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())])
        ).unwrap();

        // Create Directory table
        pkg.create_table("Directory", vec![
            msi::Column::build("Directory").primary_key().string(72),
            msi::Column::build("Directory_Parent").nullable().string(72),
            msi::Column::build("DefaultDir").primary_key().string(255),
        ]).unwrap();

        pkg.insert_rows(
            msi::Insert::into("Directory")
                .row(vec![
                    msi::Value::Str("TARGETDIR".into()),
                    msi::Value::Null,
                    msi::Value::Str("SourceDir".into()),
                ])
        ).unwrap();

        // Create InstallExecuteSequence
        pkg.create_table("InstallExecuteSequence", vec![
            msi::Column::build("Action").primary_key().string(72),
            msi::Column::build("Condition").nullable().string(255),
            msi::Column::build("Sequence").nullable().int16(),
        ]).unwrap();

        pkg.insert_rows(
            msi::Insert::into("InstallExecuteSequence")
                .row(vec![
                    msi::Value::Str("CostInitialize".into()),
                    msi::Value::Null,
                    msi::Value::Int(800),
                ])
                .row(vec![
                    msi::Value::Str("CostFinalize".into()),
                    msi::Value::Null,
                    msi::Value::Int(1000),
                ])
        ).unwrap();

        pkg.flush().unwrap();
    }

    let msi_data = buf.into_inner();
    let path = "msi_ref_test.msi";
    std::fs::write(path, &msi_data).unwrap();
    println!("msi crate MSI: {} bytes", msi_data.len());

    // Test with msiexec
    let log_path = "msi_ref_test.log";
    let output = Command::new("msiexec")
        .args(&["/i", path, "/qn", "/norestart", "/l*v", log_path])
        .output()
        .expect("msiexec failed");
    let code = output.status.code().unwrap_or(-1);
    println!("msi crate exit code: {}", code);

    match code {
        0 => {
            println!("SUCCESS! msi crate MSI installed!");
            let _ = Command::new("msiexec")
                .args(&["/x", path, "/qn", "/norestart"]).output();
        }
        1603 => {
            println!("1603 - Fatal error. Checking log...");
            if let Ok(log) = std::fs::read_to_string(log_path) {
                for line in log.lines() {
                    if line.contains("Error") || line.contains("2705") || line.contains("return value 3")
                        || line.contains("1620")
                    {
                        println!("  {}", line.trim());
                    }
                }
            }
        }
        1620 => println!("1620 - Could not open package"),
        _ => println!("Error code: {}", code),
    }

    // Now build our MSI for comparison
    println!("\n=== Building velocity-msi version ===");
    let mut b = velocity_msi::MsiBuilder::new();
    b.set_title("MsiRefTest");
    b.set_author("Velocity");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        velocity_msi::Column::build("Property").string(72).primary_key().build(),
        velocity_msi::Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("MsiRefTest")],
        vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0.0")],
        vec![velocity_msi::Value::from("Manufacturer"), velocity_msi::Value::from("V")],
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

    let our_data = b.build().unwrap();
    let our_path = "velocity_ref_test.msi";
    std::fs::write(our_path, &our_data).unwrap();
    println!("velocity-msi MSI: {} bytes", our_data.len());

    // Compare both MSIs using the msi crate reader
    println!("\n=== Stream-by-stream comparison ===");

    let mut ref_pkg = msi::Package::open(std::fs::File::open(path).unwrap()).unwrap();
    let mut our_pkg = msi::Package::open(std::fs::File::open(our_path).unwrap()).unwrap();

    let ref_tables: Vec<String> = ref_pkg.tables().map(|t| t.name().to_string()).collect();
    let our_tables: Vec<String> = our_pkg.tables().map(|t| t.name().to_string()).collect();
    println!("Reference tables: {:?}", ref_tables);
    println!("Our tables:       {:?}", our_tables);

    // Compare _Tables content
    println!("\n_Tables comparison:");
    if let Ok(rows) = ref_pkg.select_rows(msi::Select::table("_Tables")) {
        let names: Vec<String> = rows.map(|r| r[0].as_str().unwrap_or("?").to_string()).collect();
        println!("  ref: {:?}", names);
    }
    if let Ok(rows) = our_pkg.select_rows(msi::Select::table("_Tables")) {
        let names: Vec<String> = rows.map(|r| r[0].as_str().unwrap_or("?").to_string()).collect();
        println!("  our: {:?}", names);
    }

    // Compare _Columns for Directory
    println!("\n_Columns for Directory:");
    if let Ok(rows) = ref_pkg.select_rows(msi::Select::table("_Columns")) {
        for row in rows {
            let table = row[0].as_str().unwrap_or("?");
            if table == "Directory" {
                println!("  ref: col {} name={} type=0x{:04X}",
                    row[1].as_int().unwrap_or(0),
                    row[2].as_str().unwrap_or("?"),
                    row[3].as_int().unwrap_or(0) as u16);
            }
        }
    }
    if let Ok(rows) = our_pkg.select_rows(msi::Select::table("_Columns")) {
        for row in rows {
            let table = row[0].as_str().unwrap_or("?");
            if table == "Directory" {
                println!("  our: col {} name={} type=0x{:04X}",
                    row[1].as_int().unwrap_or(0),
                    row[2].as_str().unwrap_or("?"),
                    row[3].as_int().unwrap_or(0) as u16);
            }
        }
    }

    // Compare _Validation
    println!("\n_Validation row count:");
    if let Ok(rows) = ref_pkg.select_rows(msi::Select::table("_Validation")) {
        let count = rows.count();
        println!("  ref: {} rows", count);
    }
    if let Ok(rows) = our_pkg.select_rows(msi::Select::table("_Validation")) {
        let count = rows.count();
        println!("  our: {} rows", count);
    }

    // Compare _Validation entries for _Tables/_Columns
    println!("\n_Validation entries for _Tables:");
    if let Ok(rows) = ref_pkg.select_rows(msi::Select::table("_Validation")) {
        for row in rows {
            let table = row[0].as_str().unwrap_or("?");
            if table == "_Tables" {
                println!("  ref: {} / {}", table, row[1].as_str().unwrap_or("?"));
            }
        }
    }
    if let Ok(rows) = our_pkg.select_rows(msi::Select::table("_Validation")) {
        for row in rows {
            let table = row[0].as_str().unwrap_or("?");
            if table == "_Tables" {
                println!("  our: {} / {}", table, row[1].as_str().unwrap_or("?"));
            }
        }
    }
}
