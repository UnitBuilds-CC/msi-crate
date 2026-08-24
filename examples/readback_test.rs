/// Test: Can the msi crate correctly read our MSI with InstallExecuteSequence?
use velocity_msi::{MsiBuilder, Column, Value};

fn main() {
    println!("=== Read-back verification ===\n");

    let mut b = MsiBuilder::new();
    b.set_title("Read-back Test");
    b.set_author("Velocity");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(1024).build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Readback Test")],
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

    let msi_data = b.build().unwrap();
    let path = "readback_test.msi";
    std::fs::write(path, &msi_data).unwrap();
    println!("Built MSI: {} bytes\n", msi_data.len());

    // Read back with msi crate from file
    println!("--- Reading with msi crate ---");
    let file = std::fs::File::open(path).unwrap();
    let mut package = msi::Package::open(file).unwrap();
    println!("Opened successfully!\n");

    // Collect table names first to avoid borrow issues
    let table_names: Vec<String> = package.tables().map(|t| t.name().to_string()).collect();

    for tname in &table_names {
        println!("Table: {}", tname);
        // Get column info
        for table in package.tables() {
            if table.name() == tname {
                for col in table.columns() {
                    let type_str = match col.coltype() {
                        msi::ColumnType::Int16 => "i16",
                        msi::ColumnType::Int32 => "i32",
                        msi::ColumnType::Str(_) => "str",
                    };
                    print!("  {} ({}) ", col.name(), type_str);
                }
                println!();
                break;
            }
        }
        // Read rows via Select
        let col_names: Vec<&str> = {
            let mut names = Vec::new();
            for table in package.tables() {
                if table.name() == tname {
                    for col in table.columns() {
                        names.push(col.name());
                    }
                    break;
                }
            }
            names
        };
        match package.select_rows(msi::Select::table(tname.as_str()).columns(&col_names)) {
            Ok(select) => {
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
            Err(e) => println!("  Error reading rows: {}", e),
        }
        println!();
    }
}
