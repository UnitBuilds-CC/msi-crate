/// Read Feature table from compiler MSI using the msi crate
use std::io::Cursor;

fn main() {
    let msi_path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\installer.msi";
    let data = std::fs::read(msi_path).expect("read MSI");
    println!("MSI size: {} bytes", data.len());

    let mut pkg = msi::Package::open(Cursor::new(&data)).expect("open MSI");

    // List all tables
    println!("\n=== Tables ===");
    for table in pkg.tables() {
        println!("  {} ({} cols)", table.name(), table.columns().len());
    }

    // Read Feature table rows
    println!("\n=== Feature table rows ===");
    for row in pkg.select_rows(msi::Select::table("Feature")).expect("read Feature") {
        println!("  Feature={:?}, Parent={:?}, Title={:?}, Level={:?}, Dir={:?}, Attrs={:?}",
            row[0].as_str(), row[1].as_str(), row[2].as_str(),
            row[5].as_int(), row[6].as_str(), row[7].as_int());
    }

    // Read _Columns entries for Feature
    println!("\n=== _Columns for Feature (table=Feature) ===");
    for row in pkg.select_rows(msi::Select::table("_Columns")).expect("read _Columns") {
        let table = row[0].as_str().unwrap_or("?");
        if table == "Feature" {
            let number = row[1].as_int().unwrap_or(-1);
            let name = row[2].as_str().unwrap_or("?");
            let type_val = row[3].as_int().unwrap_or(-1) as u32;
            let base = type_val & 0xFFF;
            let nullable = (type_val & 0x1000) != 0;
            let pk = (type_val & 0x2000) != 0;
            let localizable = (type_val & 0x4000) != 0;
            println!("  Col#{}: name={}, type=0x{:03X}, nullable={}, pk={}, localizable={}",
                number, name, base, nullable, pk, localizable);
        }
    }

    // Check product registration in registry
    println!("\n=== HKCU Installer Products ===");
    let output = std::process::Command::new("powershell")
        .args(&["-Command", "Get-ChildItem 'HKCU:\\Software\\Microsoft\\Installer\\Products\\' -ErrorAction SilentlyContinue | ForEach-Object { $pn = (Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue).ProductName; Write-Host \"$($_.PSPath) => $pn\" }"])
        .output();
    if let Ok(out) = output {
        let s = String::from_utf8_lossy(&out.stdout);
        if s.is_empty() {
            println!("  No products found in HKCU registry!");
        } else {
            println!("{}", s);
        }
    }
}
