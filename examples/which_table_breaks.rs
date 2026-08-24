/// Test which tables can be added alongside Property without breaking msiexec
/// cargo run --example which_table_breaks
use std::io::Cursor;

fn main() {
    println!("=== WHICH TABLE BREAKS IT ===\n");
    let template_path = "C:\\Program Files\\Microsoft Office\\root\\Integration\\C2RIntLoc.en-us.16.msi";
    let template_data = std::fs::read(template_path).unwrap();
    let product_code = "A1B2C3D4-E5F6-7890-ABCD-EF1234567890";

    // Test: Property + various second tables
    let tests: Vec<(&str, Vec<msi::Column>, Vec<Vec<msi::Value>>)> = vec![
        ("Directory", 
         vec![
             msi::Column::build("Directory").primary_key().string(72),
             msi::Column::build("Directory_Parent").nullable().string(72),
             msi::Column::build("DefaultDir").localizable().string(255),
         ],
         vec![vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str(".".into())]]
        ),
        ("Component",
         vec![
             msi::Column::build("Component").primary_key().string(72),
             msi::Column::build("ComponentId").nullable().string(38),
             msi::Column::build("Directory_").string(72),
             msi::Column::build("Attributes").nullable().int16(),
             msi::Column::build("Condition").nullable().string(255),
             msi::Column::build("KeyPath").nullable().string(72),
         ],
         vec![vec![
             msi::Value::Str("Comp1".into()), msi::Value::Str("{12345678-1234-1234-1234-123456789012}".into()),
             msi::Value::Str("TARGETDIR".into()), msi::Value::Int(0), msi::Value::Null, msi::Value::Null,
         ]]
        ),
        ("CustomAction",
         vec![
             msi::Column::build("Action").primary_key().string(72),
             msi::Column::build("Type").nullable().int16(),
             msi::Column::build("Source").nullable().string(72),
             msi::Column::build("Target").nullable().localizable().string(255),
             msi::Column::build("ExtendedType").nullable().int32(),
         ],
         vec![] // Empty table
        ),
        ("Upgrade",
         vec![
             msi::Column::build("UpgradeCode").primary_key().string(38),
             msi::Column::build("VersionMin").nullable().string(20),
             msi::Column::build("VersionMax").nullable().string(20),
             msi::Column::build("Language").nullable().string(20),
             msi::Column::build("Attributes").int32(),
         ],
         vec![] // Empty table
        ),
    ];

    for (table_name, columns, rows) in &tests {
        let cursor = Cursor::new(template_data.clone());
        let mut pkg = msi::Package::open(cursor).unwrap();
        
        // Delete ALL user tables
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

        // Add the test table
        pkg.create_table(table_name.to_string(), columns.clone()).unwrap();
        if !rows.is_empty() {
            pkg.insert_rows(msi::Insert::into(*table_name).rows(rows.clone())).unwrap();
        }

        pkg.summary_info_mut().set_title("Test");
        pkg.summary_info_mut().set_author("Test");
        pkg.summary_info_mut().set_uuid(uuid::Uuid::parse_str(product_code).unwrap());
        pkg.summary_info_mut().set_word_count(2);

        pkg.flush().unwrap();
        let cursor = pkg.into_inner().unwrap();
        let msi_data = cursor.into_inner();
        let out_path = "C:\\temp\\which_table.msi";
        std::fs::write(out_path, &msi_data).unwrap();

        // Test
        let _ = std::process::Command::new("msiexec")
            .args(&["/x", &format!("{{{}}}", product_code), "/qn"]).output();
        std::thread::sleep(std::time::Duration::from_secs(1));

        let output = std::process::Command::new("msiexec")
            .args(&["/i", out_path, "/qn"]).output().expect("msiexec");
        let code = output.status.code().unwrap_or(-1);
        let status = match code { 0 => "OK", 1620 => "1620", 1603 => "1603", _ => "?" };
        println!("  Property + {} -> exit {} ({})", table_name, code, status);
    }

    // Also test: Property-only with NO other tables (baseline)
    {
        let cursor = Cursor::new(template_data.clone());
        let mut pkg = msi::Package::open(cursor).unwrap();
        let user_tables: Vec<String> = pkg.tables()
            .map(|t| t.name().to_string())
            .filter(|n| !n.starts_with('_'))
            .collect();
        for tn in &user_tables { pkg.drop_table(tn).unwrap(); }
        
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

        pkg.summary_info_mut().set_title("Test");
        pkg.summary_info_mut().set_author("Test");
        pkg.summary_info_mut().set_uuid(uuid::Uuid::parse_str(product_code).unwrap());
        pkg.summary_info_mut().set_word_count(2);

        pkg.flush().unwrap();
        let cursor = pkg.into_inner().unwrap();
        let msi_data = cursor.into_inner();
        let out_path = "C:\\temp\\which_table.msi";
        std::fs::write(out_path, &msi_data).unwrap();

        let _ = std::process::Command::new("msiexec")
            .args(&["/x", &format!("{{{}}}", product_code), "/qn"]).output();
        std::thread::sleep(std::time::Duration::from_secs(1));
        let output = std::process::Command::new("msiexec")
            .args(&["/i", out_path, "/qn"]).output().expect("msiexec");
        let code = output.status.code().unwrap_or(-1);
        println!("  Property only -> exit {} ({})", code, match code { 0 => "OK", _ => "?" });
    }

    println!("\n=== DONE ===");
}
