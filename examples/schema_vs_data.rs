/// Test: is it the table schema or the data that breaks msiexec?
/// cargo run --example schema_vs_data
use std::io::Cursor;

fn main() {
    println!("=== SCHEMA VS DATA TEST ===\n");
    let template_path = "C:\\Program Files\\Microsoft Office\\root\\Integration\\C2RIntLoc.en-us.16.msi";
    let template_data = std::fs::read(template_path).unwrap();
    let product_code = "A1B2C3D4-E5F6-7890-ABCD-EF1234567890";

    // Test 1: Directory table EMPTY (no rows)
    test_config(&template_data, product_code, "Directory (empty)", |pkg| {
        let cols = vec![
            msi::Column::build("Directory").primary_key().string(72),
            msi::Column::build("Directory_Parent").nullable().string(72),
            msi::Column::build("DefaultDir").localizable().string(255),
        ];
        pkg.create_table("Directory".to_string(), cols).unwrap();
        // No rows!
    });

    // Test 2: Upgrade table WITH rows
    test_config(&template_data, product_code, "Upgrade (with rows)", |pkg| {
        let cols = vec![
            msi::Column::build("UpgradeCode").primary_key().string(38),
            msi::Column::build("VersionMin").nullable().string(20),
            msi::Column::build("VersionMax").nullable().string(20),
            msi::Column::build("Language").nullable().string(20),
            msi::Column::build("Attributes").int32(),
        ];
        pkg.create_table("Upgrade".to_string(), cols).unwrap();
        pkg.insert_rows(msi::Insert::into("Upgrade").rows(vec![
            vec![
                msi::Value::Str("{B2C3D4E5-F6A7-8901-BCDE-F12345678901}".into()),
                msi::Value::Null, msi::Value::Null, msi::Value::Null,
                msi::Value::Int(255),
            ],
        ])).unwrap();
    });

    // Test 3: Directory with 1 row, non-localizable DefaultDir
    test_config(&template_data, product_code, "Directory (1 row, no localizable)", |pkg| {
        let cols = vec![
            msi::Column::build("Directory").primary_key().string(72),
            msi::Column::build("Directory_Parent").nullable().string(72),
            msi::Column::build("DefaultDir").string(255), // NOT localizable
        ];
        pkg.create_table("Directory".to_string(), cols).unwrap();
        pkg.insert_rows(msi::Insert::into("Directory").rows(vec![
            vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str(".".into())],
        ])).unwrap();
    });

    // Test 4: Property with 2 tables that have data
    test_config(&template_data, product_code, "Dir(empty) + Comp(empty)", |pkg| {
        let cols = vec![
            msi::Column::build("Directory").primary_key().string(72),
            msi::Column::build("Directory_Parent").nullable().string(72),
            msi::Column::build("DefaultDir").localizable().string(255),
        ];
        pkg.create_table("Directory".to_string(), cols).unwrap();
        let cols2 = vec![
            msi::Column::build("Component").primary_key().string(72),
            msi::Column::build("ComponentId").nullable().string(38),
            msi::Column::build("Directory_").string(72),
            msi::Column::build("Attributes").nullable().int16(),
            msi::Column::build("Condition").nullable().string(255),
            msi::Column::build("KeyPath").nullable().string(72),
        ];
        pkg.create_table("Component".to_string(), cols2).unwrap();
    });

    // Test 5: Two empty string tables + one with data
    test_config(&template_data, product_code, "Dir(empty) + Upgrade(1 row)", |pkg| {
        let cols = vec![
            msi::Column::build("Directory").primary_key().string(72),
            msi::Column::build("Directory_Parent").nullable().string(72),
            msi::Column::build("DefaultDir").localizable().string(255),
        ];
        pkg.create_table("Directory".to_string(), cols).unwrap();
        
        let cols2 = vec![
            msi::Column::build("UpgradeCode").primary_key().string(38),
            msi::Column::build("VersionMin").nullable().string(20),
            msi::Column::build("VersionMax").nullable().string(20),
            msi::Column::build("Language").nullable().string(20),
            msi::Column::build("Attributes").int32(),
        ];
        pkg.create_table("Upgrade".to_string(), cols2).unwrap();
        pkg.insert_rows(msi::Insert::into("Upgrade").rows(vec![
            vec![
                msi::Value::Str("{B2C3D4E5-F6A7-8901-BCDE-F12345678901}".into()),
                msi::Value::Null, msi::Value::Null, msi::Value::Null,
                msi::Value::Int(255),
            ],
        ])).unwrap();
    });

    println!("\n=== DONE ===");
}

fn test_config(template_data: &[u8], product_code: &str, label: &str, setup: impl FnOnce(&mut msi::Package<Cursor<Vec<u8>>>)) {
    let cursor = Cursor::new(template_data.to_vec());
    let mut pkg = msi::Package::open(cursor).unwrap();
    
    let user_tables: Vec<String> = pkg.tables()
        .map(|t| t.name().to_string())
        .filter(|n| !n.starts_with('_'))
        .collect();
    for tn in &user_tables { pkg.drop_table(tn).unwrap(); }
    
    // Add Property
    let prop_cols = vec![
        msi::Column::build("Property").primary_key().string(72),
        msi::Column::build("Value").nullable().localizable().string(255),
    ];
    pkg.create_table("Property".to_string(), prop_cols).unwrap();
    pkg.insert_rows(msi::Insert::into("Property").rows(vec![
        vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Test".into())],
        vec![msi::Value::Str("ProductCode".into()), msi::Value::Str(format!("{{{}}}", product_code))],
        vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())],
        vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Test".into())],
        vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())],
    ])).unwrap();

    // Run the test-specific setup
    setup(&mut pkg);

    pkg.summary_info_mut().set_title("Test");
    pkg.summary_info_mut().set_author("Test");
    pkg.summary_info_mut().set_uuid(uuid::Uuid::parse_str(product_code).unwrap());
    pkg.summary_info_mut().set_word_count(2);

    pkg.flush().unwrap();
    let cursor = pkg.into_inner().unwrap();
    let msi_data = cursor.into_inner();
    let out_path = "C:\\temp\\schema_data_test.msi";
    std::fs::write(out_path, &msi_data).unwrap();

    let _ = std::process::Command::new("msiexec")
        .args(&["/x", &format!("{{{}}}", product_code), "/qn"]).output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn"]).output().expect("msiexec");
    let code = output.status.code().unwrap_or(-1);
    let status = match code { 0 => "OK", 1620 => "1620", 1603 => "1603", _ => "?" };
    println!("  {} -> exit {} ({})", label, code, status);
}
