/// Debug: compare compiler MSI vs standalone velocity-msi
/// Check if the compiler MSI can be opened by cfb crate
use std::io::Cursor;

fn main() {
    let compiler_msi_path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\test_compiler.msi";
    let standalone_msi_path = r"C:\temp\real_cabinet_test.msi";

    println!("=== DEBUG COMPILER MSI ===\n");

    // Check if files exist
    for (name, path) in &[("Compiler", compiler_msi_path), ("Standalone", standalone_msi_path)] {
        match std::fs::read(path) {
            Ok(data) => {
                println!("{} MSI: {} bytes", name, data.len());
                // Check OLE header
                if data.len() < 512 {
                    println!("  TOO SMALL for OLE!");
                    continue;
                }
                let magic = &data[0..8];
                println!("  Magic: {:02X?}", magic);
                let ver = u16::from_le_bytes([data[24], data[25]]);
                let rev = u16::from_le_bytes([data[26], data[27]]);
                let sector_pow = u16::from_le_bytes([data[30], data[31]]);
                let mini_pow = u16::from_le_bytes([data[32], data[33]]);
                println!("  Version: {}, Revision: {}", ver, rev);
                println!("  Sector size: {} (2^{})", 2u32.pow(sector_pow as u32), sector_pow);
                println!("  Mini sector size: {} (2^{})", 2u32.pow(mini_pow as u32), mini_pow);

                // Try to open with cfb
                match cfb::CompoundFile::open(Cursor::new(&data)) {
                    Ok(comp) => {
                        let streams: Vec<_> = comp.walk()
                            .filter(|e| e.is_stream())
                            .map(|e| (e.name().to_string(), e.len()))
                            .collect();
                        println!("  Streams: {}", streams.len());
                        for (name, len) in &streams {
                            println!("    {} ({} bytes)", name, len);
                        }
                    }
                    Err(e) => {
                        println!("  CFB OPEN FAILED: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("{} MSI: NOT FOUND ({})", name, e);
            }
        }
        println!();
    }
}
