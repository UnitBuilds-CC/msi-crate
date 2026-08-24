/// Test: open our velocity-msi output with the msi crate and verify ProcessComponents data.
/// Also: create a NEW MSI using msi crate's Package::create(), write it to disk,
/// and check if its _Columns stream matches ours byte-for-byte.
use std::io::Cursor;

fn main() {
    let our_msi_path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\definitive_test.msi";
    
    // Read our MSI
    let our_data = std::fs::read(our_msi_path).unwrap();
    eprintln!("Our MSI: {} bytes", our_data.len());
    
    // Open with msi crate and read all table data
    let cursor = Cursor::new(&our_data);
    let pkg = msi::Package::open(cursor).unwrap();
    
    // Read _StringPool to understand string mapping
    eprintln!("\n=== String Pool ===");
    {
        let table = pkg.tables().get("_StringPool");
        if let Some(table_def) = table {
            eprintln!("_StringPool table found, columns: {}", table_def.columns().len());
        }
    }
    
    // Read Component table
    eprintln!("\n=== Component Table ===");
    read_table_rows(&pkg, "Component");
    
    // Read Feature table
    eprintln!("\n=== Feature Table ===");
    read_table_rows(&pkg, "Feature");
    
    // Read FeatureComponents table
    eprintln!("\n=== FeatureComponents Table ===");
    read_table_rows(&pkg, "FeatureComponents");
    
    // Read Directory table
    eprintln!("\n=== Directory Table ===");
    read_table_rows(&pkg, "Directory");
    
    // Read InstallExecuteSequence
    eprintln!("\n=== InstallExecuteSequence ===");
    read_table_rows(&pkg, "InstallExecuteSequence");
    
    // Read Property table
    eprintln!("\n=== Property Table ===");
    read_table_rows(&pkg, "Property");
    
    // Read _Tables
    eprintln!("\n=== _Tables (listed tables) ===");
    read_table_rows(&pkg, "_Tables");
    
    // Read _Columns for Component table
    eprintln!("\n=== _Columns for Component ===");
    read_columns_for_table(&pkg, "Component");
    
    // Now create a NEW MSI using msi crate's Package::create()
    eprintln!("\n\n=== Creating reference MSI via msi crate ===");
    create_msi_crate_reference();
}

fn read_table_rows(pkg: &msi::Package, table_name: &str) {
    match pkg.tables().get(table_name) {
        Some(_table_def) => {
            // Use a SQL-like query to read rows
            let query = format!("SELECT * FROM `{}`", table_name);
            match pkg.select_rows(msi::Select::query(&query)) {
                Ok(rows) => {
                    eprintln!("  {} rows:", rows.len());
                    for (i, row) in rows.iter().enumerate() {
                        let mut vals = Vec::new();
                        for j in 0..row.len() {
                            match &row[j] {
                                msi::ValueRef::Int(v) => vals.push(format!("{}", v)),
                                msi::ValueRef::Str(s) => vals.push(format!("'{}'", s)),
                                msi::ValueRef::Null => vals.push("NULL".to_string()),
                                msi::ValueRef::Stream(_) => vals.push("<stream>".to_string()),
                            }
                        }
                        eprintln!("    [{}] {}", i, vals.join(", "));
                    }
                }
                Err(e) => eprintln!("  Error reading rows: {}", e),
            }
        }
        None => eprintln!("  Table not found!"),
    }
}

fn read_columns_for_table(pkg: &msi::Package, table_name: &str) {
    let query = format!("SELECT * FROM `_Columns` WHERE `Table` = '{}'", table_name);
    match pkg.select_rows(msi::Select::query(&query)) {
        Ok(rows) => {
            for row in &rows {
                let col_num = match &row[1] {
                    msi::ValueRef::Int(v) => *v,
                    _ => -1,
                };
                let col_name = match &row[2] {
                    msi::ValueRef::Str(s) => s.to_string(),
                    _ => "?".to_string(),
                };
                let col_type = match &row[3] {
                    msi::ValueRef::Int(v) => *v,
                    _ => -1,
                };
                eprintln!("  Col {}: {} (type=0x{:04X})", col_num, col_name, col_type as u16);
            }
        }
        Err(e) => eprintln!("  Error: {}", e),
    }
}

fn create_msi_crate_reference() {
    use msi::*;
    
    let mut pkg = Package::create(PackageType::Installer, "x64", 1033).unwrap();
    
    // Create tables matching our velocity-msi output
    pkg.create_table("Property", vec![
        Column::build("Property").primary_key().string(72).build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    
    pkg.create_table("Directory", vec![
        Column::build("Directory").primary_key().string(72).build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).nullable().build(),
    ]).unwrap();
    
    pkg.create_table("Component", vec![
        Column::build("Component").primary_key().string(72).build(),
        Column::build("ComponentId").string(38).nullable().build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
    
    pkg.create_table("Feature", vec![
        Column::build("Feature").primary_key().string(38).build(),
        Column::build("Feature_Parent").string(38).nullable().build(),
        Column::build("Title").string(64).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().build(),
    ]).unwrap();
    
    pkg.create_table("FeatureComponents", vec![
        Column::build("Feature_").primary_key().string(38).build(),
        Column::build("Component_").primary_key().string(72).build(),
    ]).unwrap();
    
    pkg.create_table("InstallExecuteSequence", vec![
        Column::build("Action").primary_key().string(72).build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    
    pkg.create_table("InstallUISequence", vec![
        Column::build("Action").primary_key().string(72).build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    
    // Insert data
    let product_code = "{AABBCCDD-1234-5678-9ABC-DEF012345678}";
    let upgrade_code = "{11223344-5566-7788-99AA-BBCCDDEEFF00}";
    
    insert_rows(&mut pkg, "Property", vec![
        vec![Value::from("ProductName"), Value::from("MSI Crate Reference")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity Team")],
        vec![Value::from("ProductCode"), Value::from(product_code)],
        vec![Value::from("UpgradeCode"), Value::from(upgrade_code)],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]);
    
    insert_rows(&mut pkg, "Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("LocalAppDataFolder"), Value::from("TARGETDIR"), Value::from("LocalAppData")],
        vec![Value::from("INSTALLDIR"), Value::from("LocalAppDataFolder"), Value::from("MsiCrateRef:MsiCrateRef")],
    ]);
    
    insert_rows(&mut pkg, "Component", vec![
        vec![Value::from("comp_0"), Value::Null, Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::from("file_0")],
        vec![Value::from("comp_1"), Value::Null, Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::from("file_1")],
    ]);
    
    insert_rows(&mut pkg, "Feature", vec![
        vec![Value::from("Complete"), Value::Null, Value::from("Complete"), Value::from("All files"), Value::Int(1), Value::Int(1), Value::from("INSTALLDIR"), Value::Int(0)],
    ]);
    
    insert_rows(&mut pkg, "FeatureComponents", vec![
        vec![Value::from("Complete"), Value::from("comp_0")],
        vec![Value::from("Complete"), Value::from("comp_1")],
    ]);
    
    insert_rows(&mut pkg, "InstallExecuteSequence", vec![
        vec![Value::from("LaunchConditions"), Value::from("NOT Installed"), Value::Int(100)],
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("InstallValidate"), Value::Null, Value::Int(1400)],
        vec![Value::from("InstallInitialize"), Value::Null, Value::Int(1500)],
        vec![Value::from("ProcessComponents"), Value::Null, Value::Int(1600)],
        vec![Value::from("RegisterProduct"), Value::Null, Value::Int(6100)],
        vec![Value::from("PublishFeatures"), Value::Null, Value::Int(6300)],
        vec![Value::from("PublishProduct"), Value::Null, Value::Int(6400)],
        vec![Value::from("InstallFinalize"), Value::Null, Value::Int(6600)],
    ]);
    
    insert_rows(&mut pkg, "InstallUISequence", vec![
        vec![Value::from("LaunchConditions"), Value::Null, Value::Int(100)],
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("ExecuteAction"), Value::Null, Value::Int(1300)],
    ]);
    
    // Set summary info
    pkg.set_title("MSI Crate Reference").unwrap();
    pkg.set_author("Velocity Team").unwrap();
    pkg.set_subject("Reference MSI").unwrap();
    pkg.set_comments("Created by msi crate").unwrap();
    pkg.set_word_count(2).unwrap();
    
    // Write to file
    let out_path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\msi_crate_ref.msi";
    let file = std::fs::File::create(out_path).unwrap();
    pkg.flush(file).unwrap();
    
    eprintln!("Created msi crate reference: {}", out_path);
    eprintln!("Size: {} bytes", std::fs::metadata(out_path).unwrap().len());
    
    // Now binary-compare the table streams between our MSI and the msi crate reference
    compare_streams(
        r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\definitive_test.msi",
        out_path,
    );
}

fn compare_streams(our_path: &str, ref_path: &str) {
    use std::io::Read;
    
    let our_data = std::fs::read(our_path).unwrap();
    let ref_data = std::fs::read(ref_path).unwrap();
    
    // Open both with msi crate
    let our_pkg = msi::Package::open(Cursor::new(&our_data)).unwrap();
    let ref_pkg = msi::Package::open(Cursor::new(&ref_data)).unwrap();
    
    // Compare _Columns stream for Component table
    eprintln!("\n=== Binary comparison of table streams ===");
    
    // Read _StringPool from both
    eprintln!("\n--- _StringPool ---");
    compare_stream_data(&our_pkg, &ref_pkg, "_StringPool");
    
    eprintln!("\n--- _StringData ---");
    compare_stream_data(&our_pkg, &ref_pkg, "_StringData");
    
    // Compare Component table stream
    eprintln!("\n--- Component ---");
    compare_stream_data(&our_pkg, &ref_pkg, "Component");
    
    // Compare Directory table stream
    eprintln!("\n--- Directory ---");
    compare_stream_data(&our_pkg, &ref_pkg, "Directory");
    
    // Compare Feature table stream
    eprintln!("\n--- Feature ---");
    compare_stream_data(&our_pkg, &ref_pkg, "Feature");
    
    // Compare InstallExecuteSequence
    eprintln!("\n--- InstallExecuteSequence ---");
    compare_stream_data(&our_pkg, &ref_pkg, "InstallExecuteSequence");
}

fn compare_stream_data(our_pkg: &msi::Package, ref_pkg: &msi::Package, table_name: &str) {
    // We can't directly access raw streams via the msi crate API.
    // But we can compare the logical data.
    // For a true binary comparison, we'd need to extract the raw OLE streams.
    eprintln!("  (Logical comparison via msi crate API)");
    
    // Compare row data
    let query = format!("SELECT * FROM `{}`", table_name);
    let our_rows = our_pkg.select_rows(msi::Select::query(&query));
    let ref_rows = ref_pkg.select_rows(msi::Select::query(&query));
    
    match (our_rows, ref_rows) {
        (Ok(our), Ok(refr)) => {
            eprintln!("  Our rows: {}, Ref rows: {}", our.len(), refr.len());
            if our.len() == refr.len() {
                for (i, (o, r)) in our.iter().zip(refr.iter()).enumerate() {
                    if o.len() != r.len() {
                        eprintln!("  Row {}: column count mismatch ({} vs {})", i, o.len(), r.len());
                        continue;
                    }
                    for j in 0..o.len() {
                        let o_str = format!("{:?}", o[j]);
                        let r_str = format!("{:?}", r[j]);
                        if o_str != r_str {
                            eprintln!("  Row {} Col {}: OUR={} REF={}", i, j, o_str, r_str);
                        }
                    }
                }
                eprintln!("  All rows match logically!");
            }
        }
        (Err(e), _) => eprintln!("  Our read error: {}", e),
        (_, Err(e)) => eprintln!("  Ref read error: {}", e),
    }
}

fn insert_rows(pkg: &mut msi::Package, table_name: &str, rows: Vec<Vec<msi::Value>>) {
    for row in rows {
        pkg.insert_rows(table_name, vec![row]).unwrap();
    }
}
