/// Check CLSID and OLE structure of various MSIs.
/// cargo run --example check_clsid -p velocity-msi
use std::io::{Cursor, Read};

fn check_msi(path: &str, label: &str) {
    println!("\n=== {} ===", label);
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => { println!("  Cannot read: {}", e); return; }
    };
    println!("  Size: {} bytes", data.len());

    // Check OLE header
    let sig = u16::from_le_bytes([data[0], data[1]]);
    println!("  OLE signature: 0x{:04X} {}", sig, if sig == 0xD0CF { "(valid)" } else { "(INVALID!)" });

    // Check version
    if data.len() > 29 {
        let ver_major = data[26];
        let ver_minor = data[28];
        println!("  OLE version: {}.{}", ver_major, ver_minor);
    }

    let mut comp = match cfb::CompoundFile::open(Cursor::new(&data)) {
        Ok(c) => c,
        Err(e) => { println!("  Cannot open CFB: {}", e); return; }
    };

    // Check root entry CLSID - read from raw OLE header
    // The root entry CLSID is stored in the directory entry
    // For now, just check if the OLE structure is valid
    println!("  Root CLSID: (check via raw bytes)");

    // List streams
    let names: Vec<String> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.name().to_string())
        .collect();
    println!("  Streams: {}", names.len());
    for name in &names {
        let mut stream = comp.open_stream(name).unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).unwrap();
        let safe: String = name.chars().map(|c| if c.is_ascii() && !c.is_control() { c } else { '.' }).collect();
        println!("    {} ({} bytes)", safe, buf.len());

        // For SummaryInfo, show first bytes
        if name.contains("SummaryInformation") {
            print!("    SI header: ");
            for b in &buf[..32.min(buf.len())] { print!("{:02X} ", b); }
            println!();
            // OS version is at offset 4-5
            if buf.len() > 6 {
                let os_ver = u16::from_le_bytes([buf[4], buf[5]]);
                println!("    SI OS version: {}", os_ver);
            }
        }

        // For _StringPool, show codepage
        if name.contains("StringPool") || (buf.len() >= 4 && name.starts_with('\u{4840}')) {
            // Check if this could be a string pool (first 4 bytes = codepage)
            if buf.len() >= 4 {
                let cp = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
                let cp_low = cp & 0xFFFF;
                if cp_low == 1252 || cp_low == 65001 || cp_low == 1200 {
                    println!("    Possible codepage: {}", cp_low);
                }
            }
        }
    }

    // Try to open with msi crate
    match msi::Package::open(Cursor::new(&data)) {
        Ok(pkg) => {
            println!("  msi crate: OPENED OK");
            for table in pkg.tables() {
                println!("    Table: {} ({} cols)", 
                    table.name(), table.columns().len());
            }
        }
        Err(e) => println!("  msi crate: FAILED: {:?}", e),
    };
}

fn main() {
    println!("=== MSI STRUCTURE CHECKER ===");

    // Template (known working)
    check_msi(
        "C:\\Program Files\\Microsoft Office\\root\\Integration\\C2RIntLoc.en-us.16.msi",
        "Template (Office MSI)"
    );

    // cfb roundtrip (accepted by msiexec, 1603)
    check_msi("C:\\temp\\cfb_roundtrip.msi", "CFB Roundtrip");

    // msi crate create() output (1620)
    check_msi("C:\\temp\\ref_msi.msi", "msi crate create()");

    // velocity-msi output (1620)
    check_msi("C:\\temp\\velo_compare.msi", "velocity-msi");

    // velocity-msi definitive (1620)
    check_msi("C:\\temp\\velocity_definitive.msi", "velocity-msi definitive");

    println!("\n=== DONE ===");
}
