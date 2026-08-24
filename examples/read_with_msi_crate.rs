/// Read our MSI with the msi crate and dump all table data.
use std::io::Cursor;

fn dump_table(pkg: &mut msi::Package<Cursor<Vec<u8>>>, tname: &str) {
    eprintln!("\n=== {} ===", tname);
    let query = msi::Select::table(tname);
    match pkg.select_rows(query) {
        Ok(rows) => {
            let ncols = rows.columns().len();
            let mut col_names = Vec::new();
            for i in 0..ncols {
                col_names.push(rows.columns()[i].name().to_owned());
            }
            eprintln!("  Columns: {:?}", col_names);
            let mut count = 0;
            for row in rows {
                eprint!("  [{}] ", count);
                for i in 0..ncols {
                    if i > 0 { eprint!(", "); }
                    let val = &row[i];
                    match val {
                        msi::Value::Null => eprint!("NULL"),
                        msi::Value::Int(v) => eprint!("{}", v),
                        msi::Value::Str(s) => eprint!("'{}'", s),
                    }
                }
                eprintln!();
                count += 1;
            }
            eprintln!("  ({} rows total)", count);
        }
        Err(e) => eprintln!("  ERROR: {}", e),
    }
}

fn main() {
    let msi_path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\diag_custom.msi";
    let data = std::fs::read(msi_path).unwrap();
    let cursor = Cursor::new(data);
    
    let mut pkg = msi::Package::open(cursor).unwrap();
    
    eprintln!("=== Tables in package ===");
    for table in pkg.tables() {
        eprintln!("  {} ({} columns)", table.name(), table.columns().len());
    }
    
    dump_table(&mut pkg, "Component");
    dump_table(&mut pkg, "File");
    dump_table(&mut pkg, "Feature");
    dump_table(&mut pkg, "FeatureComponents");
    dump_table(&mut pkg, "Directory");
    dump_table(&mut pkg, "Media");
    dump_table(&mut pkg, "Property");
    dump_table(&mut pkg, "InstallExecuteSequence");
    dump_table(&mut pkg, "_Columns");
    dump_table(&mut pkg, "_Tables");
}
