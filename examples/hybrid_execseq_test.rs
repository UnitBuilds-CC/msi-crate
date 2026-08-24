/// Test: Open a working Directory-only MSI with the msi crate,
/// add InstallExecuteSequence using the msi crate's API, save, and test with msiexec.
/// This determines if the bug is in our OLE structure or our table data.
use velocity_msi::{MsiBuilder, Column, Value};

fn main() {
    // Step 1: Create a working MSI with just Property + Directory
    let mut b = MsiBuilder::new();
    b.set_title("Hybrid Test");
    b.set_author("V");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("HybridTest")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("V")],
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
    ]).unwrap();

    let msi_data = b.build().unwrap();
    std::fs::write("hybrid_base.msi", &msi_data).unwrap();
    println!("Step 1: Built base MSI (Property+Directory) - {} bytes", msi_data.len());

    // Step 2: Open with msi crate and add InstallExecuteSequence
    println!("Step 2: Opening with msi crate to add InstallExecuteSequence...");
    let cursor = std::io::Cursor::new(&msi_data);
    let mut pkg = msi::Package::open(cursor).unwrap();

    // Create InstallExecuteSequence table
    pkg.create_table("InstallExecuteSequence", vec![
        ("Action", msi::ColumnType::Str(72)),
        ("Condition", msi::ColumnType::Str(255)),
        ("Sequence", msi::ColumnType::Int16),
    ]).unwrap();

    // Add rows
    pkg.insert_rows("InstallExecuteSequence", vec![
        vec![
            msi::Value::Str("CostInitialize".to_string()),
            msi::Value::Null,
            msi::Value::Int(800),
        ],
        vec![
            msi::Value::Str("CostFinalize".to_string()),
            msi::Value::Null,
            msi::Value::Int(1000),
        ],
    ]).unwrap();

    // Save
    let mut out = std::io::Cursor::new(Vec::new());
    pkg.flush(&mut out).unwrap();
    let hybrid_data = out.into_inner();
    std::fs::write("hybrid_msi_crate.msi", &hybrid_data).unwrap();
    println!("Step 2: Saved hybrid MSI (msi crate ExecSeq) - {} bytes", hybrid_data.len());

    // Step 3: Verify with msi crate
    println!("\nStep 3: Verifying hybrid MSI...");
    let cursor2 = std::io::Cursor::new(&hybrid_data);
    let pkg2 = msi::Package::open(cursor2).unwrap();
    let table_names: Vec<String> = pkg2.tables().map(|t| t.name().to_string()).collect();
    println!("Tables: {:?}", table_names);

    // Read Directory table
    let col_names = vec!["Directory", "Directory_Parent", "DefaultDir"];
    match pkg2.select_rows(msi::Select::table("Directory").columns(&col_names)) {
        Ok(select) => {
            println!("\nDirectory rows:");
            for row in select {
                let mut vals = Vec::new();
                for i in 0..row.len() {
                    vals.push(match &row[i] {
                        msi::Value::Null => "NULL".to_string(),
                        msi::Value::Int(n) => format!("Int({})", n),
                        msi::Value::Str(s) => format!("\"{}\"", s),
                    });
                }
                println!("  {:?}", vals);
            }
        }
        Err(e) => println!("Error reading Directory: {}", e),
    }

    // Read ExecSeq
    let col_names2 = vec!["Action", "Condition", "Sequence"];
    match pkg2.select_rows(msi::Select::table("InstallExecuteSequence").columns(&col_names2)) {
        Ok(select) => {
            println!("\nInstallExecuteSequence rows:");
            for row in select {
                let mut vals = Vec::new();
                for i in 0..row.len() {
                    vals.push(match &row[i] {
                        msi::Value::Null => "NULL".to_string(),
                        msi::Value::Int(n) => format!("Int({})", n),
                        msi::Value::Str(s) => format!("\"{}\"", s),
                    });
                }
                println!("  {:?}", vals);
            }
        }
        Err(e) => println!("Error reading ExecSeq: {}", e),
    }

    println!("\nDone. Test both MSIs:");
    println!("  msiexec /i hybrid_base.msi /qn /lv hybrid_base.log       (should work)");
    println!("  msiexec /i hybrid_msi_crate.msi /qn /lv hybrid_crate.log (test this one)");
}
