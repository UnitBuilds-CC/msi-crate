/// Open our MSI with the msi crate, flush it to a new file,
/// then compare the two. If the msi-flushed version works with
/// ProcessComponents but ours doesn't, the bug is in our OLE serialization.
/// If both fail, the bug is in the table data (but we proved data is correct).
use std::io::Cursor;

fn main() {
    let input_path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\diag_custom.msi";
    let output_path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\msi_flushed.msi";
    
    let data = std::fs::read(input_path).unwrap();
    let cursor = Cursor::new(data.clone());
    
    // Open with msi crate (read-write mode)
    let mut pkg = msi::Package::open(cursor).unwrap();
    eprintln!("Opened MSI successfully");
    
    // Verify we can read tables
    let tables: Vec<String> = pkg.tables().map(|t| t.name().to_string()).collect();
    eprintln!("Tables: {:?}", tables);
    
    // Flush to new file via msi crate's serialization
    let flushed = {
        let inner = pkg.into_inner().unwrap();
        inner.into_inner()
    };
    
    std::fs::write(output_path, &flushed).unwrap();
    eprintln!("Flushed MSI written: {} ({} bytes)", output_path, flushed.len());
    eprintln!("Original size: {} bytes", data.len());
    
    // Compare sizes
    if flushed.len() == data.len() {
        eprintln!("SAME SIZE!");
    } else {
        eprintln!("SIZE DIFFERS: original={}, flushed={}", data.len(), flushed.len());
    }
    
    // Compare bytes
    let mut diffs = 0;
    let min_len = data.len().min(flushed.len());
    for i in 0..min_len {
        if data[i] != flushed[i] {
            if diffs < 20 {
                eprintln!("  Diff at byte {}: original=0x{:02X}, flushed=0x{:02X}", i, data[i], flushed[i]);
            }
            diffs += 1;
        }
    }
    if data.len() != flushed.len() {
        eprintln!("  ... plus {} bytes of length difference", (data.len() as i64 - flushed.len() as i64).abs());
    }
    eprintln!("Total byte differences: {}", diffs);
    
    eprintln!("\nTo test flushed MSI:");
    eprintln!("  Start-Process msiexec -ArgumentList '/i','{}','/qn','/l*vx','{}' -Wait",
        output_path, output_path.replace(".msi", ".log"));
}
