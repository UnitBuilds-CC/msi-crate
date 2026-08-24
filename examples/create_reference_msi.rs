/// Create a reference MSI using the msi crate, test with msiexec,
/// then binary-compare with our custom OLE output.
use std::io::Cursor;

fn main() {
    println!("=== Reference MSI Creation ===\n");

    // Create a minimal MSI using the msi crate
    let cursor = Cursor::new(Vec::new());
    let mut pkg = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
    
    // Set codepage
    pkg.summary_info_mut().set_codepage(msi::CodePage::Windows1252);
    pkg.set_database_codepage(msi::CodePage::Windows1252);
    
    // Set summary info
    {
        let si = pkg.summary_info_mut();
        si.set_title("Installation Database");
        si.set_subject("Reference Corp");
        si.set_author("Reference Corp");
    }

    // Create Property table
    pkg.create_table("Property", vec![
        msi::Column::build("Property").primary_key().string(72),
        msi::Column::build("Value").nullable().localizable().string(255),
    ]).unwrap();

    // Insert properties
    let properties = vec![
        ("ProductName", "Reference Product"),
        ("ProductVersion", "1.0.0"),
        ("Manufacturer", "Reference Corp"),
        ("ProductCode", "{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}"),
        ("UpgradeCode", "{11111111-2222-3333-4444-555555555555}"),
        ("ProductLanguage", "1033"),
    ];

    for (name, value) in properties {
        pkg.insert_rows(msi::Insert::into("Property")
            .row(vec![msi::Value::Str(name.into()), msi::Value::Str(value.into())])
        ).unwrap();
    }

    // Flush and get data
    pkg.flush().unwrap();
    let cursor = pkg.into_inner().unwrap();
    let ref_data = cursor.into_inner();
    
    println!("Reference MSI (msi crate): {} bytes", ref_data.len());
    std::fs::write("C:\\temp\\reference_msi.msi", &ref_data).unwrap();

    // Test with msiexec
    let status = std::process::Command::new("msiexec")
        .args(&["/i", "C:\\temp\\reference_msi.msi", "/qn", "/norestart"])
        .status();
    let code = status.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
    println!("msiexec exit code: {}", code);

    // Uninstall
    let status2 = std::process::Command::new("msiexec")
        .args(&["/x", "{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}", "/qn", "/norestart"])
        .status();
    if let Ok(s) = status2 {
        println!("uninstall exit code: {}", s.code().unwrap_or(-1));
    }

    if code == 0 {
        println!("\n✓ Reference MSI works! Now comparing with our output...");
        
        // Now build the same MSI with velocity-msi
        use velocity_msi::{MsiBuilder, Column, Value};
        
        let mut builder = MsiBuilder::new();
        builder.set_title("Installation Database");
        builder.set_author("Reference Corp");
        builder.set_subject("Reference Corp");
        builder.set_template("Intel", 1033);
        
        builder.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ]).unwrap();
        builder.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("Reference Product")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("Manufacturer"), Value::from("Reference Corp")],
            vec![Value::from("ProductCode"), Value::from("{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}")],
            vec![Value::from("UpgradeCode"), Value::from("{11111111-2222-3333-4444-555555555555}")],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
        ]).unwrap();
        
        let our_data = builder.build().unwrap();
        println!("\nOur MSI (velocity-msi): {} bytes", our_data.len());
        std::fs::write("C:\\temp\\our_msi.msi", &our_data).unwrap();
        
        // Binary compare headers
        println!("\n=== Header Comparison ===");
        compare_bytes(&ref_data, &our_data, "Header", 0, 76);
        
        // Compare DIFAT
        println!("\n=== DIFAT Comparison ===");
        compare_bytes(&ref_data, &our_data, "DIFAT", 76, 436);
        
        // Compare first directory sector
        let ref_dir_sector = u32::from_le_bytes([ref_data[48], ref_data[49], ref_data[50], ref_data[51]]);
        let our_dir_sector = u32::from_le_bytes([our_data[48], our_data[49], our_data[50], our_data[51]]);
        println!("Reference first dir sector: {}", ref_dir_sector);
        println!("Our first dir sector:       {}", our_dir_sector);
        
        let ref_dir_off = 512 + ref_dir_sector as usize * 512;
        let our_dir_off = 512 + our_dir_sector as usize * 512;
        compare_bytes(&ref_data, &our_data, "Directory sector", ref_dir_off, 512);
        
        // Compare individual directory entries
        for i in 0..4 {
            let ref_entry_off = ref_dir_off + i * 128;
            let our_entry_off = our_dir_off + i * 128;
            if ref_entry_off + 128 <= ref_data.len() && our_entry_off + 128 <= our_data.len() {
                compare_bytes(&ref_data, &our_data, &format!("Dir entry {}", i), ref_entry_off, 128);
            }
        }
    } else {
        println!("\n✗ Reference MSI failed with msiexec. Cannot compare.");
    }
}

fn compare_bytes(a: &[u8], b: &[u8], label: &str, offset: usize, len: usize) {
    let a_end = std::cmp::min(offset + len, a.len());
    let b_end = std::cmp::min(offset + len, b.len());
    let a_slice = &a[offset..a_end];
    let b_slice = &b[offset..b_end];
    
    if a_slice == b_slice {
        println!("{}: IDENTICAL ({} bytes)", label, a_slice.len());
    } else {
        println!("{}: DIFFERENT!", label);
        let mut diffs = 0;
        for i in 0..std::cmp::min(a_slice.len(), b_slice.len()) {
            if a_slice[i] != b_slice[i] {
                if diffs < 20 {
                    println!("  +{}: ref=0x{:02X} our=0x{:02X}", i, a_slice[i], b_slice[i]);
                }
                diffs += 1;
            }
        }
        println!("  Total: {} differences in {} bytes", diffs, std::cmp::min(a_slice.len(), b_slice.len()));
    }
}
