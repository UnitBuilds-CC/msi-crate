/// Test: keep template's Directory table instead of recreating
/// cargo run --example keep_dir_test
use std::io::Cursor;

fn main() {
    println!("=== KEEP DIRECTORY TABLE TEST ===\n");

    let template_path = "C:\\Program Files\\Microsoft Office\\root\\Integration\\C2RIntLoc.en-us.16.msi";
    let template_data = std::fs::read(template_path).unwrap();
    let product_code = "A1B2C3D4-E5F6-7890-ABCD-EF1234567890";

    // Test 1: Keep Directory, drop everything else, add Property
    println!("--- Test 1: Keep Directory, add Property ---");
    {
        let cursor = Cursor::new(template_data.clone());
        let mut pkg = msi::Package::open(cursor).unwrap();
        let user_tables: Vec<String> = pkg.tables()
            .map(|t| t.name().to_string())
            .filter(|n| !n.starts_with('_') && n != "Directory")
            .collect();
        for tn in &user_tables { pkg.drop_table(tn).unwrap(); }
        
        // Add Property table
        let cols = vec![
            msi::Column::build("Property").primary_key().string(72),
            msi::Column::build("Value").nullable().localizable().string(255),
        ];
        pkg.create_table("Property".to_string(), cols).unwrap();
        pkg.insert_rows(msi::Insert::into("Property").rows(vec![
            vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Velocity Test".into())],
            vec![msi::Value::Str("ProductCode".into()), msi::Value::Str(format!("{{{}}}", product_code))],
            vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())],
            vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Velocity".into())],
            vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())],
        ])).unwrap();

        pkg.summary_info_mut().set_title("Velocity Test");
        pkg.summary_info_mut().set_author("Velocity");
        pkg.summary_info_mut().set_uuid(uuid::Uuid::parse_str(product_code).unwrap());
        pkg.summary_info_mut().set_word_count(2);

        pkg.flush().unwrap();
        let cursor = pkg.into_inner().unwrap();
        let msi_data = cursor.into_inner();
        let out_path = "C:\\temp\\keep_dir_test.msi";
        std::fs::write(out_path, &msi_data).unwrap();
        test_msiexec(out_path, product_code);
    }

    // Test 2: Drop Directory, recreate it with just TARGETDIR
    println!("\n--- Test 2: Recreate Directory with just TARGETDIR ---");
    {
        let cursor = Cursor::new(template_data.clone());
        let mut pkg = msi::Package::open(cursor).unwrap();
        let user_tables: Vec<String> = pkg.tables()
            .map(|t| t.name().to_string())
            .filter(|n| !n.starts_with('_'))
            .collect();
        for tn in &user_tables { pkg.drop_table(tn).unwrap(); }
        
        // Add Property
        let cols = vec![
            msi::Column::build("Property").primary_key().string(72),
            msi::Column::build("Value").nullable().localizable().string(255),
        ];
        pkg.create_table("Property".to_string(), cols).unwrap();
        pkg.insert_rows(msi::Insert::into("Property").rows(vec![
            vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Velocity Test".into())],
            vec![msi::Value::Str("ProductCode".into()), msi::Value::Str(format!("{{{}}}", product_code))],
            vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())],
            vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Velocity".into())],
            vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())],
        ])).unwrap();

        // Recreate Directory with just TARGETDIR
        let cols = vec![
            msi::Column::build("Directory").primary_key().string(72),
            msi::Column::build("Directory_Parent").nullable().string(72),
            msi::Column::build("DefaultDir").localizable().string(255),
        ];
        pkg.create_table("Directory".to_string(), cols).unwrap();
        pkg.insert_rows(msi::Insert::into("Directory").rows(vec![
            vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str(".".into())],
        ])).unwrap();

        pkg.summary_info_mut().set_title("Velocity Test");
        pkg.summary_info_mut().set_author("Velocity");
        pkg.summary_info_mut().set_uuid(uuid::Uuid::parse_str(product_code).unwrap());
        pkg.summary_info_mut().set_word_count(2);

        pkg.flush().unwrap();
        let cursor = pkg.into_inner().unwrap();
        let msi_data = cursor.into_inner();
        let out_path = "C:\\temp\\keep_dir_test2.msi";
        std::fs::write(out_path, &msi_data).unwrap();
        test_msiexec(out_path, product_code);
    }

    // Test 3: Drop Directory, recreate with non-localizable DefaultDir
    println!("\n--- Test 3: Directory with non-localizable DefaultDir ---");
    {
        let cursor = Cursor::new(template_data.clone());
        let mut pkg = msi::Package::open(cursor).unwrap();
        let user_tables: Vec<String> = pkg.tables()
            .map(|t| t.name().to_string())
            .filter(|n| !n.starts_with('_'))
            .collect();
        for tn in &user_tables { pkg.drop_table(tn).unwrap(); }
        
        // Property
        let cols = vec![
            msi::Column::build("Property").primary_key().string(72),
            msi::Column::build("Value").nullable().localizable().string(255),
        ];
        pkg.create_table("Property".to_string(), cols).unwrap();
        pkg.insert_rows(msi::Insert::into("Property").rows(vec![
            vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Velocity Test".into())],
            vec![msi::Value::Str("ProductCode".into()), msi::Value::Str(format!("{{{}}}", product_code))],
            vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())],
            vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Velocity".into())],
            vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())],
        ])).unwrap();

        // Directory with non-localizable DefaultDir
        let cols = vec![
            msi::Column::build("Directory").primary_key().string(72),
            msi::Column::build("Directory_Parent").nullable().string(72),
            msi::Column::build("DefaultDir").string(255),  // NOT localizable
        ];
        pkg.create_table("Directory".to_string(), cols).unwrap();
        pkg.insert_rows(msi::Insert::into("Directory").rows(vec![
            vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str(".".into())],
        ])).unwrap();

        pkg.summary_info_mut().set_title("Velocity Test");
        pkg.summary_info_mut().set_author("Velocity");
        pkg.summary_info_mut().set_uuid(uuid::Uuid::parse_str(product_code).unwrap());
        pkg.summary_info_mut().set_word_count(2);

        pkg.flush().unwrap();
        let cursor = pkg.into_inner().unwrap();
        let msi_data = cursor.into_inner();
        let out_path = "C:\\temp\\keep_dir_test3.msi";
        std::fs::write(out_path, &msi_data).unwrap();
        test_msiexec(out_path, product_code);
    }

    // Test 4: Use the template's Directory table schema (read it)
    println!("\n--- Test 4: Read template Directory schema ---");
    {
        let cursor = Cursor::new(template_data.clone());
        let pkg = msi::Package::open(cursor).unwrap();
        if let Some(table) = pkg.tables().find(|t| t.name() == "Directory") {
            println!("  Directory columns:");
            for col in table.columns() {
                println!("    {} - {:?} (nullable={}, localizable={}, pk={})",
                    col.name(), col.coltype(), col.is_nullable(), col.is_localizable(), col.is_primary_key());
            }
            // Can't easily iterate rows in msi crate 0.7
            println!("  (rows not easily iterable in msi crate 0.7)");
        }
    }

    println!("\n=== DONE ===");
}

fn test_msiexec(path: &str, product_code: &str) {
    let _ = std::process::Command::new("msiexec")
        .args(&["/x", &format!("{{{}}}", product_code), "/qn"]).output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    let output = std::process::Command::new("msiexec")
        .args(&["/i", path, "/qn", "/l*v", "C:\\temp\\keep_dir_test.log"])
        .output().expect("msiexec failed");
    let code = output.status.code().unwrap_or(-1);
    println!("  Exit code: {} ({})", code, match code {
        0 => "SUCCESS",
        1603 => "fatal error",
        1605 => "data readable",
        1613 => "invalid pkg",
        1620 => "cannot open",
        _ => "unknown",
    });
}
