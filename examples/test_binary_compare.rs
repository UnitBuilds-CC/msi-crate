//! Binary compare: test MSI vs compiler MSI - dump first 200 bytes of each stream.

use std::fs;

fn main() {
    // Read the compiler's MSI
    let compiler_msi = fs::read("C:\\temp\\sample_msi_test\\SampleApp.msi").unwrap();
    
    // Read our working test MSI
    let test_msi = fs::read("C:\\temp\\vel_msi_test\\one_at_a_time.msi").unwrap();
    
    println!("Compiler MSI: {} bytes", compiler_msi.len());
    println!("Test MSI:     {} bytes", test_msi.len());
    
    // Compare OLE headers (first 76 bytes)
    println!("\n=== OLE Header comparison ===");
    println!("Compiler: {:02X?}", &compiler_msi[0..8]);
    println!("Test:     {:02X?}", &test_msi[0..8]);
    
    // Both should start with D0 CF 11 E0 A1 B1 1A E1
    println!("\nCompiler magic: {:02X} {:02X} {:02X} {:02X}", 
        compiler_msi[0], compiler_msi[1], compiler_msi[2], compiler_msi[3]);
    println!("Test magic:     {:02X} {:02X} {:02X} {:02X}", 
        test_msi[0], test_msi[1], test_msi[2], test_msi[3]);
    
    // Check sector size (byte 30-31)
    let compiler_sector = u16::from_le_bytes([compiler_msi[30], compiler_msi[31]]);
    let test_sector = u16::from_le_bytes([test_msi[30], test_msi[31]]);
    println!("\nCompiler sector size: {} (2^{})", 1u32 << compiler_sector, compiler_sector);
    println!("Test sector size:     {} (2^{})", 1u32 << test_sector, test_sector);
    
    // Check version (byte 28-29)
    let compiler_ver = u16::from_le_bytes([compiler_msi[28], compiler_msi[29]]);
    let test_ver = u16::from_le_bytes([test_msi[28], test_msi[29]]);
    println!("Compiler version: {}", compiler_ver);
    println!("Test version:     {}", test_ver);
    
    // Try to enumerate streams using cfb crate
    println!("\n=== Stream enumeration (compiler MSI) ===");
    match cfb::open("C:\\temp\\sample_msi_test\\SampleApp.msi") {
        Ok(compound) => {
            for entry in compound.walk() {
                println!("  {} ({} bytes)", entry.name(), entry.len());
            }
        }
        Err(e) => println!("  Failed to open: {}", e),
    }
    
    println!("\n=== Stream enumeration (test MSI) ===");
    match cfb::open("C:\\temp\\vel_msi_test\\one_at_a_time.msi") {
        Ok(compound) => {
            for entry in compound.walk() {
                println!("  {} ({} bytes)", entry.name(), entry.len());
            }
        }
        Err(e) => println!("  Failed to open: {}", e),
    }
}
