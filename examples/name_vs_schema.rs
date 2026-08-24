/// Test: is it the NAME "Directory" or the SCHEMA that breaks?
/// cargo run --example name_vs_schema
use std::io::Cursor;

fn main() {
    println!("=== NAME VS SCHEMA TEST ===\n");
    let template_path = "C:\\Program Files\\Microsoft Office\\root\\Integration\\C2RIntLoc.en-us.16.msi";
    let template_data = std::fs::read(template_path).unwrap();
    let product_code = "A1B2C3D4-E5F6-7890-ABCD-EF1234567890";

    // Test 1: Same schema as Directory but different name
    test_config(&template_data, product_code, "Table 'MyDir' (Directory schema)", |pkg| {
        let cols = vec![
            msi::Column::build("MyDir").primary_key().string(72),
            msi::Column::build("MyDir_Parent").nullable().string(72),
            msi::Column::build("DefaultDir").localizable().string(255),
        ];
        pkg.create_table("MyDir".to_string(), cols).unwrap();
        pkg.insert_rows(msi::Insert::into("MyDir").rows(vec![
            vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str(".".into())],
        ])).unwrap();
    });

    // Test 2: Name "Directory" but simple schema (like Property)
    test_config(&template_data, product_code, "Table 'Directory' (Property-like schema)", |pkg| {
        let cols = vec![
            msi::Column::build("Directory").primary_key().string(72),
            msi::Column::build("Value").nullable().string(255),
        ];
        pkg.create_table("Directory".to_string(), cols).unwrap();
        pkg.insert_rows(msi::Insert::into("Directory").rows(vec![
            vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Str("test".into())],
        ])).unwrap();
    });

    // Test 3: Completely new table name with 3 string columns
    test_config(&template_data, product_code, "Table 'FooBar' (3 string cols)", |pkg| {
        let cols = vec![
            msi::Column::build("Key").primary_key().string(72),
            msi::Column::build("Parent").nullable().string(72),
            msi::Column::build("Data").localizable().string(255),
        ];
        pkg.create_table("FooBar".to_string(), cols).unwrap();
        pkg.insert_rows(msi::Insert::into("FooBar").rows(vec![
            vec![msi::Value::Str("KEY1".into()), msi::Value::Null, msi::Value::Str("data".into())],
        ])).unwrap();
    });

    // Test 4: Table with int primary key + string column
    test_config(&template_data, product_code, "Table 'IntKey' (int PK + string)", |pkg| {
        let cols = vec![
            msi::Column::build("ID").primary_key().int32(),
            msi::Column::build("Name").nullable().string(72),
        ];
        pkg.create_table("IntKey".to_string(), cols).unwrap();
        pkg.insert_rows(msi::Insert::into("IntKey").rows(vec![
            vec![msi::Value::Int(1), msi::Value::Str("test".into())],
        ])).unwrap();
    });

    // Test 5: Property + Media (simple 2-column table)
    test_config(&template_data, product_code, "Table 'Media'", |pkg| {
        let cols = vec![
            msi::Column::build("DiskId").primary_key().int16(),
            msi::Column::build("LastSequence").string(20),
            msi::Column::build("Cabinet").nullable().localizable().string(255),
        ];
        pkg.create_table("Media".to_string(), cols).unwrap();
        pkg.insert_rows(msi::Insert::into("Media").rows(vec![
            vec![msi::Value::Int(1), msi::Value::Str("0".into()), msi::Value::Str("#velo.cab".into())],
        ])).unwrap();
    });

    // Test 6: Just Property + Feature (simple table)
    test_config(&template_data, product_code, "Table 'Feature'", |pkg| {
        let cols = vec![
            msi::Column::build("Feature").primary_key().string(38),
            msi::Column::build("Feature_Parent").nullable().string(38),
            msi::Column::build("Title").nullable().localizable().string(64),
            msi::Column::build("Description").nullable().localizable().string(255),
            msi::Column::build("Display").nullable().int16(),
            msi::Column::build("Level").int16(),
            msi::Column::build("Directory_").nullable().string(72),
            msi::Column::build("Attributes").nullable().int16(),
        ];
        pkg.create_table("Feature".to_string(), cols).unwrap();
        pkg.insert_rows(msi::Insert::into("Feature").rows(vec![
            vec![
                msi::Value::Str("MainFeature".into()), msi::Value::Null,
                msi::Value::Str("Test Feature".into()), msi::Value::Null,
                msi::Value::Int(2), msi::Value::Int(1),
                msi::Value::Str("TARGETDIR".into()), msi::Value::Int(0),
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

    setup(&mut pkg);

    pkg.summary_info_mut().set_title("Test");
    pkg.summary_info_mut().set_author("Test");
    pkg.summary_info_mut().set_uuid(uuid::Uuid::parse_str(product_code).unwrap());
    pkg.summary_info_mut().set_word_count(2);

    pkg.flush().unwrap();
    let cursor = pkg.into_inner().unwrap();
    let msi_data = cursor.into_inner();
    let out_path = "C:\\temp\\name_schema_test.msi";
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
