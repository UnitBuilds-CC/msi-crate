// Standalone benchmark for testing large file support
// Run with: cargo run --release --example benchmark_large_files

use velocity_msi::ole::{build_ole_file, OleStream};
use std::time::Instant;

fn main() {
    println!("=== Velocity MSI Large File Benchmark ===\n");

    // Test sizes: 100MB, 500MB, 1GB
    let sizes = vec![
        ("100MB", 100 * 1024 * 1024),
        ("500MB", 500 * 1024 * 1024),
        ("1GB", 1024 * 1024 * 1024),
    ];

    for (name, size) in sizes {
        println!("Testing {} ({} bytes)...", name, size);
        
        let data = vec![0xAB; size];
        let streams = vec![OleStream {
            name: "LargeFile".to_string(),
            data: data.clone(),
        }];

        let start = Instant::now();
        let ole_data = build_ole_file(&streams);
        let elapsed = start.elapsed();

        let file_size_mb = ole_data.len() as f64 / (1024.0 * 1024.0);
        let build_ms = elapsed.as_millis();
        let build_secs = elapsed.as_secs_f64();
        let throughput_mb = file_size_mb / build_secs;

        println!("  Output size:   {:.2} MB", file_size_mb);
        println!("  Build time:    {} ms ({:.2} seconds)", build_ms, build_secs);
        println!("  Throughput:    {:.1} MB/s", throughput_mb);
        
        // Verify header
        let num_fat = u32::from_le_bytes([ole_data[44], ole_data[45], ole_data[46], ole_data[47]]);
        let num_difat = u32::from_le_bytes([ole_data[68], ole_data[69], ole_data[70], ole_data[71]]);
        let first_difat = u32::from_le_bytes([ole_data[64], ole_data[65], ole_data[66], ole_data[67]]);
        
        println!("  FAT sectors:   {}", num_fat);
        println!("  DIFAT sectors: {}", num_difat);
        
        if num_fat > 109 && num_difat > 0 && first_difat != 0xFFFFFFFF {
            println!("  ✓ DIFAT support working correctly!");
        } else if num_fat <= 109 {
            println!("  ✓ No DIFAT needed (fits in header)");
        } else {
            println!("  ✗ DIFAT support NOT working!");
        }
        println!();
    }

    println!("=== Benchmark Complete ===");
}
