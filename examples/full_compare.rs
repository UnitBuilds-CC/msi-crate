use std::io::Cursor;

fn main() {
    // Read reference MSI
    let ref_data = std::fs::read("ref_minimal_test.msi").unwrap();
    let ref_cursor = Cursor::new(&ref_data);
    let mut ref_pkg = msi::Package::open(ref_cursor).unwrap();

    // Read velocity MSI
    let vel_data = std::fs::read("minimal_test.msi").unwrap();
    let vel_cursor = Cursor::new(&vel_data);
    let mut vel_pkg = match msi::Package::open(vel_cursor) {
        Ok(p) => p,
        Err(e) => {
            println!("ERROR: Cannot open velocity MSI with msi crate: {}", e);
            println!("This means the OLE structure is broken!");
            return;
        }
    };

    println!("Reference MSI: {} bytes", ref_data.len());
    println!("Velocity MSI: {} bytes\n", vel_data.len());

    // Compare _Tables
    println!("=== _Tables ===");
    let ref_tables: Vec<String> = ref_pkg.select_rows(msi::Select::table("_Tables")).unwrap()
        .collect::<Vec<_>>().iter().map(|r| r[0].as_str().unwrap_or("?").to_string()).collect();
    let vel_tables: Vec<String> = vel_pkg.select_rows(msi::Select::table("_Tables")).unwrap()
        .collect::<Vec<_>>().iter().map(|r| r[0].as_str().unwrap_or("?").to_string()).collect();
    println!("  Reference: {:?}", ref_tables);
    println!("  Velocity:  {:?}", vel_tables);
    if ref_tables != vel_tables {
        println!("  *** DIFFERENT! ***");
    }

    // Compare _Columns for each table
    println!("\n=== _Columns comparison ===");
    for table_name in &ref_tables {
        println!("\n  Table: {}", table_name);
        let ref_cols: Vec<_> = ref_pkg.select_rows(msi::Select::table("_Columns")).unwrap()
            .into_iter()
            .filter(|r| r[0].as_str() == Some(table_name))
            .map(|r| (r[1].as_int().unwrap(), r[2].as_str().unwrap().to_string(), r[3].as_int().unwrap()))
            .collect();
        let vel_cols: Vec<_> = vel_pkg.select_rows(msi::Select::table("_Columns")).unwrap()
            .into_iter()
            .filter(|r| r[0].as_str() == Some(table_name))
            .map(|r| (r[1].as_int().unwrap(), r[2].as_str().unwrap().to_string(), r[3].as_int().unwrap()))
            .collect();

        if ref_cols.len() != vel_cols.len() {
            println!("    *** Column count differs: ref={} vel={} ***", ref_cols.len(), vel_cols.len());
        }

        for (i, ((rn, rname, rtype), (vn, vname, vtype))) in ref_cols.iter().zip(vel_cols.iter()).enumerate() {
            let match_str = if rn == vn && rname == vname && rtype == vtype { "OK" } else { "*** DIFF ***" };
            println!("    Col {}: ref=({},{},0x{:04X}) vel=({},{},0x{:04X}) {}", i+1, rn, rname, rtype, vn, vname, vtype, match_str);
        }
    }

    // Compare _Validation for Directory table specifically
    println!("\n=== _Validation for Directory ===");
    let ref_val: Vec<_> = ref_pkg.select_rows(msi::Select::table("_Validation")).unwrap()
        .into_iter()
        .filter(|r| r[0].as_str() == Some("Directory"))
        .collect();
    let vel_val: Vec<_> = vel_pkg.select_rows(msi::Select::table("_Validation")).unwrap()
        .into_iter()
        .filter(|r| r[0].as_str() == Some("Directory"))
        .collect();

    println!("  Reference rows: {}", ref_val.len());
    for row in &ref_val {
        println!("    col={} nullable={} min={:?} max={:?} key_table={:?} key_col={:?} cat={:?} set={:?}",
            row[1].as_str().unwrap_or("?"),
            row[2].as_str().unwrap_or("?"),
            row[3].as_int(),
            row[4].as_int(),
            row[5].as_str(),
            row[6].as_int(),
            row[7].as_str(),
            row[8].as_str());
    }

    println!("  Velocity rows: {}", vel_val.len());
    for row in &vel_val {
        println!("    col={} nullable={} min={:?} max={:?} key_table={:?} key_col={:?} cat={:?} set={:?}",
            row[1].as_str().unwrap_or("?"),
            row[2].as_str().unwrap_or("?"),
            row[3].as_int(),
            row[4].as_int(),
            row[5].as_str(),
            row[6].as_int(),
            row[7].as_str(),
            row[8].as_str());
    }

    // Compare _Validation row counts per table
    println!("\n=== _Validation row counts ===");
    let ref_val_counts: std::collections::HashMap<String, usize> = ref_pkg.select_rows(msi::Select::table("_Validation")).unwrap()
        .into_iter()
        .fold(std::collections::HashMap::new(), |mut acc, r| {
            *acc.entry(r[0].as_str().unwrap_or("?").to_string()).or_insert(0) += 1;
            acc
        });
    let vel_val_counts: std::collections::HashMap<String, usize> = vel_pkg.select_rows(msi::Select::table("_Validation")).unwrap()
        .into_iter()
        .fold(std::collections::HashMap::new(), |mut acc, r| {
            *acc.entry(r[0].as_str().unwrap_or("?").to_string()).or_insert(0) += 1;
            acc
        });

    let mut all_tables: Vec<String> = ref_val_counts.keys().chain(vel_val_counts.keys()).cloned().collect();
    all_tables.sort();
    all_tables.dedup();

    for table in &all_tables {
        let rc = ref_val_counts.get(table).unwrap_or(&0);
        let vc = vel_val_counts.get(table).unwrap_or(&0);
        let match_str = if rc == vc { "" } else { " *** DIFF ***" };
        println!("  {}: ref={} vel={}{}", table, rc, vc, match_str);
    }

    // Compare Directory table data
    println!("\n=== Directory table data ===");
    let ref_dir: Vec<_> = ref_pkg.select_rows(msi::Select::table("Directory")).unwrap().collect();
    let vel_dir: Vec<_> = vel_pkg.select_rows(msi::Select::table("Directory")).unwrap().collect();
    println!("  Reference:");
    for row in &ref_dir {
        println!("    {:?} / {:?} / {:?}", row[0].as_str(), row[1].as_str(), row[2].as_str());
    }
    println!("  Velocity:");
    for row in &vel_dir {
        println!("    {:?} / {:?} / {:?}", row[0].as_str(), row[1].as_str(), row[2].as_str());
    }

    // Compare InstallExecuteSequence
    println!("\n=== InstallExecuteSequence data ===");
    let ref_seq: Vec<_> = ref_pkg.select_rows(msi::Select::table("InstallExecuteSequence")).unwrap().collect();
    let vel_seq: Vec<_> = vel_pkg.select_rows(msi::Select::table("InstallExecuteSequence")).unwrap().collect();
    println!("  Reference ({} rows):", ref_seq.len());
    for row in &ref_seq {
        println!("    {:?} / {:?} / {:?}", row[0].as_str(), row[1].as_str(), row[2].as_int());
    }
    println!("  Velocity ({} rows):", vel_seq.len());
    for row in &vel_seq {
        println!("    {:?} / {:?} / {:?}", row[0].as_str(), row[1].as_str(), row[2].as_int());
    }
}
