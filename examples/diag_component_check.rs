/// Diagnostic: read Component table from our MSI using the msi crate
use std::io::Cursor;
use msi::{Package, Select};

fn main() {
    let path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\definitive_test.msi";
    let data = std::fs::read(path).unwrap();
    eprintln!("File size: {} bytes", data.len());
    
    let cursor = Cursor::new(data);
    let mut pkg: Package<Cursor<Vec<u8>>> = Package::open(cursor).unwrap();
    
    // List all tables
    eprintln!("\n=== Tables ===");
    for table in pkg.tables() {
        eprintln!("  {} ({} cols)", table.name(), table.columns().len());
        for (i, col) in table.columns().iter().enumerate() {
            eprintln!("    [{}] {} nullable={} pk={}", 
                i, col.name(), col.is_nullable(), col.is_primary_key());
        }
    }
    
    // Read tables using index-based access
    let table_names = ["Component", "File", "Directory", "Feature", 
                       "FeatureComponents", "Media", "Property", 
                       "InstallExecuteSequence", "_Tables", "_Columns"];
    
    // Pre-compute column counts
    let col_counts: std::collections::HashMap<String, usize> = pkg.tables()
        .map(|t| (t.name().to_string(), t.columns().len()))
        .collect();

    for table_name in &table_names {
        eprintln!("\n=== {} rows ===", table_name);
        let ncols = col_counts.get(*table_name).copied().unwrap_or(10);
        match pkg.select_rows(Select::table(*table_name)) {
            Ok(rows) => {
                for row in rows {
                    let mut vals = Vec::new();
                    for i in 0..ncols {
                        match &row[i] {
                            msi::Value::Null => vals.push("NULL".to_string()),
                            msi::Value::Int(v) => vals.push(format!("Int({})", v)),
                            msi::Value::Str(s) => vals.push(format!("Str({})", s)),
                        }
                    }
                    eprintln!("  {}", vals.join(", "));
                }
            }
            Err(e) => eprintln!("  ERROR: {}", e),
        }
    }
    
    // Check SummaryInfo
    eprintln!("\n=== SummaryInfo ===");
    let si = pkg.summary_info();
    eprintln!("  Title: {:?}", si.title());
    eprintln!("  Subject: {:?}", si.subject());
    eprintln!("  Author: {:?}", si.author());
    eprintln!("  UUID: {:?}", si.uuid());
    eprintln!("  WordCount: {:?}", si.word_count());
    eprintln!("  Codepage: {:?}", pkg.database_codepage());
}
