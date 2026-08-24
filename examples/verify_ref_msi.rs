/// Verify: can the msi crate read back its own output?
use std::io::Cursor;
use msi::{Select, Package};

fn main() {
    let path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\ref_msi_crate.msi";
    let data = std::fs::read(path).unwrap();
    eprintln!("File size: {} bytes", data.len());
    
    let cursor = Cursor::new(data);
    let mut pkg: Package<Cursor<Vec<u8>>> = match Package::open(cursor) {
        Ok(pkg) => pkg,
        Err(e) => {
            eprintln!("FAILED to open: {}", e);
            return;
        }
    };
    
    eprintln!("Opened successfully!");
    
    // List all tables
    eprintln!("\nTables:");
    for table in pkg.tables() {
        eprintln!("  {} ({} cols)", table.name(), table.columns().len());
    }
    
    // Check Property table
    eprintln!("\nProperty rows:");
    match pkg.select_rows(Select::table("Property")) {
        Ok(rows) => {
            for row in rows {
                eprintln!("  {:?} = {:?}", &row[0], &row[1]);
            }
        }
        Err(e) => eprintln!("  ERROR: {}", e),
    }
    
    // Check SummaryInfo
    eprintln!("\nSummaryInfo:");
    let si = pkg.summary_info();
    eprintln!("  Title: {:?}", si.title());
    eprintln!("  Subject: {:?}", si.subject());
    eprintln!("  Author: {:?}", si.author());
    eprintln!("  UUID: {:?}", si.uuid());
    eprintln!("  WordCount: {:?}", si.word_count());
    eprintln!("  Codepage: {:?}", pkg.database_codepage());
}
