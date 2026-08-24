/// Test: is the issue with drop+recreate cycle?
/// cargo run --example drop_recreate_test
use std::io::Cursor;

fn main() {
    println!("=== DROP/RECREATE TEST ===\n");
    let template_path = "C:\\Program Files\\Microsoft Office\\root\\Integration\\C2RIntLoc.en-us.16.msi";
    let template_data = std::fs::read(template_path).unwrap();
    let product_code = "A1B2C3D4-E5F6-7890-ABCD-EF1234567890";

    // Template tables: AdminExecuteSequence, AdminUISequence, AdvtExecuteSequence,
    // Directory, File, InstallExecuteSequence, InstallUISequence, Media,
    // ModuleComponents, ModuleDependency, ModuleExclusion, Property
    // NOTE: Upgrade is NOT in the template!

    // Test 1: No changes, just flush (baseline)
    test_config(&template_data, product_code, "No changes (just flush)", |_pkg| {});

    // Test 2: Create NEW table "Upgrade" (not in template)
    test_config(&template_data, product_code, "Create new table 'Upgrade'", |pkg| {
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

    // Test 3: Create NEW table "MyCustomTable" (not in template)
    test_config(&template_data, product_code, "Create new table 'MyCustomTable'", |pkg| {
        let cols = vec![
            msi::Column::build("Key").primary_key().string(72),
            msi::Column::build("Value").nullable().localizable().string(255),
        ];
        pkg.create_table("MyCustomTable".to_string(), cols).unwrap();
        pkg.insert_rows(msi::Insert::into("MyCustomTable").rows(vec![
            vec![msi::Value::Str("test".into()), msi::Value::Str("data".into())],
        ])).unwrap();
    });

    // Test 4: Drop Property (existing), recreate it
    test_config(&template_data, product_code, "Drop+recreate Property", |pkg| {
        pkg.drop_table("Property").unwrap();
        let prop_cols = vec![
            msi::Column::build("Property").primary_key().string(72),
            msi::Column::build("Value").nullable().localizable().string(255),
        ];
        pkg.create_table("Property".to_string(), prop_cols).unwrap();
        pkg.insert_rows(msi::Insert::into("Property").rows(vec![
            vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Test".into())],
            vec![msi::Value::Str("ProductCode".into()), msi::Value::Str(format!("{{{}}}", product_code))],
        ])).unwrap();
    });

    // Test 5: Drop Directory (existing), recreate it
    test_config(&template_data, product_code, "Drop+recreate Directory", |pkg| {
        pkg.drop_table("Directory").unwrap();
        let cols = vec![
            msi::Column::build("Directory").primary_key().string(72),
            msi::Column::build("Directory_Parent").nullable().string(72),
            msi::Column::build("DefaultDir").localizable().string(255),
        ];
        pkg.create_table("Directory".to_string(), cols).unwrap();
        pkg.insert_rows(msi::Insert::into("Directory").rows(vec![
            vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str(".".into())],
        ])).unwrap();
    });

    // Test 6: Create new table + Upgrade (both new)
    test_config(&template_data, product_code, "Create 2 new tables (Upgrade + MyTable)", |pkg| {
        let cols1 = vec![
            msi::Column::build("UpgradeCode").primary_key().string(38),
            msi::Column::build("VersionMin").nullable().string(20),
            msi::Column::build("VersionMax").nullable().string(20),
            msi::Column::build("Language").nullable().string(20),
            msi::Column::build("Attributes").int32(),
        ];
        pkg.create_table("Upgrade".to_string(), cols1).unwrap();
        pkg.insert_rows(msi::Insert::into("Upgrade").rows(vec![
            vec![
                msi::Value::Str("{B2C3D4E5-F6A7-8901-BCDE-F12345678901}".into()),
                msi::Value::Null, msi::Value::Null, msi::Value::Null,
                msi::Value::Int(255),
            ],
        ])).unwrap();

        let cols2 = vec![
            msi::Column::build("Key").primary_key().string(72),
            msi::Column::build("Value").nullable().localizable().string(255),
        ];
        pkg.create_table("MyTable".to_string(), cols2).unwrap();
        pkg.insert_rows(msi::Insert::into("MyTable").rows(vec![
            vec![msi::Value::Str("k1".into()), msi::Value::Str("v1".into())],
        ])).unwrap();
    });

    // Test 7: Drop File (existing), DON'T recreate
    test_config(&template_data, product_code, "Drop File only (no recreate)", |pkg| {
        pkg.drop_table("File").unwrap();
    });

    // Test 8: Create new Directory-like table (different name, same schema)
    test_config(&template_data, product_code, "New table 'Dir2' (Directory schema)", |pkg| {
        let cols = vec![
            msi::Column::build("DirKey").primary_key().string(72),
            msi::Column::build("DirParent").nullable().string(72),
            msi::Column::build("DefaultName").localizable().string(255),
        ];
        pkg.create_table("Dir2".to_string(), cols).unwrap();
        pkg.insert_rows(msi::Insert::into("Dir2").rows(vec![
            vec![msi::Value::Str("TARGET".into()), msi::Value::Null, msi::Value::Str(".".into())],
        ])).unwrap();
    });

    println!("\n=== DONE ===");
}

fn test_config(
    template_data: &[u8],
    product_code: &str,
    label: &str,
    setup: impl FnOnce(&mut msi::Package<Cursor<Vec<u8>>>),
) {
    let cursor = Cursor::new(template_data.to_vec());
    let mut pkg = msi::Package::open(cursor).unwrap();
    
    setup(&mut pkg);

    pkg.summary_info_mut().set_title("Test");
    pkg.summary_info_mut().set_author("Test");
    pkg.summary_info_mut().set_uuid(uuid::Uuid::parse_str(product_code).unwrap());
    pkg.summary_info_mut().set_word_count(2);

    pkg.flush().unwrap();
    let cursor = pkg.into_inner().unwrap();
    let msi_data = cursor.into_inner();
    let out_path = "C:\\temp\\drop_recreate_test.msi";
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
