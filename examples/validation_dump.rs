use std::io::Cursor;

fn main() {
    let ref_data = std::fs::read("ref_minimal_test.msi").unwrap();
    let vel_data = std::fs::read("minimal_test.msi").unwrap();

    let mut ref_pkg = msi::Package::open(Cursor::new(&ref_data)).unwrap();
    let mut vel_pkg = msi::Package::open(Cursor::new(&vel_data)).unwrap();

    println!("Reference MSI: {} bytes", ref_data.len());
    println!("Velocity MSI:  {} bytes\n", vel_data.len());

    // Dump ALL _Validation entries for ALL tables
    println!("=== ALL _Validation entries ===\n");

    let ref_val: Vec<_> = ref_pkg.select_rows(msi::Select::table("_Validation")).unwrap().collect();
    let vel_val: Vec<_> = vel_pkg.select_rows(msi::Select::table("_Validation")).unwrap().collect();

    println!("Reference _Validation: {} rows", ref_val.len());
    println!("Velocity _Validation:  {} rows\n", vel_val.len());

    // Group by table name
    let mut ref_by_table: std::collections::BTreeMap<String, Vec<_>> = std::collections::BTreeMap::new();
    for row in &ref_val {
        let table = row[0].as_str().unwrap_or("?").to_string();
        ref_by_table.entry(table).or_default().push(row);
    }

    let mut vel_by_table: std::collections::BTreeMap<String, Vec<_>> = std::collections::BTreeMap::new();
    for row in &vel_val {
        let table = row[0].as_str().unwrap_or("?").to_string();
        vel_by_table.entry(table).or_default().push(row);
    }

    let mut all_tables: Vec<String> = ref_by_table.keys().chain(vel_by_table.keys()).cloned().collect();
    all_tables.sort();
    all_tables.dedup();

    for table in &all_tables {
        let ref_rows = ref_by_table.get(table).map(|v| v.as_slice()).unwrap_or(&[]);
        let vel_rows = vel_by_table.get(table).map(|v| v.as_slice()).unwrap_or(&[]);

        println!("--- {} ---", table);
        println!("  Reference: {} rows, Velocity: {} rows", ref_rows.len(), vel_rows.len());

        // Compare each row
        let max_rows = ref_rows.len().max(vel_rows.len());
        for i in 0..max_rows {
            let r = ref_rows.get(i);
            let v = vel_rows.get(i);

            match (r, v) {
                (Some(rr), Some(vr)) => {
                    let col_r = rr[1].as_str().unwrap_or("?");
                    let col_v = vr[1].as_str().unwrap_or("?");
                    let null_r = rr[2].as_str().unwrap_or("?");
                    let null_v = vr[2].as_str().unwrap_or("?");
                    let min_r = rr[3].as_int();
                    let min_v = vr[3].as_int();
                    let max_r = rr[4].as_int();
                    let max_v = vr[4].as_int();
                    let kt_r = rr[5].as_str();
                    let kt_v = vr[5].as_str();
                    let kc_r = rr[6].as_int();
                    let kc_v = vr[6].as_int();
                    let cat_r = rr[7].as_str();
                    let cat_v = vr[7].as_str();
                    let set_r = rr[8].as_str();
                    let set_v = vr[8].as_str();
                    let desc_r = rr[9].as_str();
                    let desc_v = vr[9].as_str();

                    let same = col_r == col_v && null_r == null_v && min_r == min_v && max_r == max_v
                        && kt_r == kt_v && kc_r == kc_v && cat_r == cat_v && set_r == set_v && desc_r == desc_v;

                    if !same {
                        println!("  *** ROW {} DIFF ***", i);
                        println!("    REF: col={} null={} min={:?} max={:?} kt={:?} kc={:?} cat={:?} set={:?} desc={:?}",
                            col_r, null_r, min_r, max_r, kt_r, kc_r, cat_r, set_r, desc_r);
                        println!("    VEL: col={} null={} min={:?} max={:?} kt={:?} kc={:?} cat={:?} set={:?} desc={:?}",
                            col_v, null_v, min_v, max_v, kt_v, kc_v, cat_v, set_v, desc_v);
                    }
                }
                (Some(rr), None) => {
                    println!("  *** REF ONLY row {}: col={} ***", i, rr[1].as_str().unwrap_or("?"));
                }
                (None, Some(vr)) => {
                    println!("  *** VEL ONLY row {}: col={} ***", i, vr[1].as_str().unwrap_or("?"));
                }
                _ => {}
            }
        }
    }

    // Also compare _Columns binary data for Directory
    println!("\n=== _Columns binary comparison for Directory ===");
    let ref_cols: Vec<_> = ref_pkg.select_rows(msi::Select::table("_Columns")).unwrap()
        .into_iter().filter(|r| r[0].as_str() == Some("Directory")).collect();
    let vel_cols: Vec<_> = vel_pkg.select_rows(msi::Select::table("_Columns")).unwrap()
        .into_iter().filter(|r| r[0].as_str() == Some("Directory")).collect();

    for (i, (r, v)) in ref_cols.iter().zip(vel_cols.iter()).enumerate() {
        let rn = r[1].as_int().unwrap();
        let vn = v[1].as_int().unwrap();
        let rname = r[2].as_str().unwrap_or("?");
        let vname = v[2].as_str().unwrap_or("?");
        let rtype = r[3].as_int().unwrap();
        let vtype = v[3].as_int().unwrap();
        let same = rn == vn && rname == vname && rtype == vtype;
        println!("  Col {}: ref=({},{},0x{:04X}) vel=({},{},0x{:04X}) {}",
            i+1, rn, rname, rtype, vn, vname, vtype, if same { "OK" } else { "*** DIFF ***" });
    }

    // Compare Property table data
    println!("\n=== Property table data ===");
    let ref_props: Vec<_> = ref_pkg.select_rows(msi::Select::table("Property")).unwrap().collect();
    let vel_props: Vec<_> = vel_pkg.select_rows(msi::Select::table("Property")).unwrap().collect();
    println!("Reference: {} rows", ref_props.len());
    for row in &ref_props {
        println!("  {} = {:?}", row[0].as_str().unwrap_or("?"), row[1].as_str());
    }
    println!("Velocity: {} rows", vel_props.len());
    for row in &vel_props {
        println!("  {} = {:?}", row[0].as_str().unwrap_or("?"), row[1].as_str());
    }

    // Compare SummaryInformation
    println!("\n=== SummaryInformation ===");
    // Read summary info from both
    let ref_si: Vec<_> = ref_pkg.select_rows(msi::Select::table("_Tables")).unwrap().collect();
    let vel_si: Vec<_> = vel_pkg.select_rows(msi::Select::table("_Tables")).unwrap().collect();
    println!("Reference _Tables: {} entries", ref_si.len());
    println!("Velocity _Tables:  {} entries", vel_si.len());
}
