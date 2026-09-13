use std::io::Cursor;

fn main() {
    let cursor = Cursor::new(Vec::new());
    let mut pkg = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();

    // Property table
    pkg.create_table("Property", vec![
        msi::Column::build("Property").primary_key().string(72),
        msi::Column::build("Value").string(1024),
    ]).unwrap();
    pkg.insert_rows(msi::Insert::into("Property")
        .row(vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Minimal Test".into())])
        .row(vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())])
        .row(vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Test Corp".into())])
        .row(vec![msi::Value::Str("ProductCode".into()), msi::Value::Str("{12345678-1234-1234-1234-123456789012}".into())])
        .row(vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())])
    ).unwrap();

    // Directory table
    pkg.create_table("Directory", vec![
        msi::Column::build("Directory").primary_key().category(msi::Category::Identifier).string(72),
        msi::Column::build("Directory_Parent").nullable().category(msi::Category::Identifier).string(72),
        msi::Column::build("DefaultDir").primary_key().category(msi::Category::DefaultDir).string(255),
    ]).unwrap();
    pkg.insert_rows(msi::Insert::into("Directory")
        .row(vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str("SourceDir".into())])
        .row(vec![msi::Value::Str("INSTALLDIR".into()), msi::Value::Str("TARGETDIR".into()), msi::Value::Str("VelTest:VelTest".into())])
    ).unwrap();

    // Component table
    pkg.create_table("Component", vec![
        msi::Column::build("Component").primary_key().category(msi::Category::Identifier).string(72),
        msi::Column::build("ComponentId").nullable().category(msi::Category::Guid).string(38),
        msi::Column::build("Directory_").category(msi::Category::Identifier).string(72),
        msi::Column::build("Attributes").int16(),
        msi::Column::build("Condition").nullable().category(msi::Category::Condition).string(255),
        msi::Column::build("KeyPath").nullable().category(msi::Category::Identifier).string(72),
    ]).unwrap();
    pkg.insert_rows(msi::Insert::into("Component")
        .row(vec![
            msi::Value::Str("MainComp".into()),
            msi::Value::Null,
            msi::Value::Str("INSTALLDIR".into()),
            msi::Value::Int(0),
            msi::Value::Null,
            msi::Value::Null,
        ])
    ).unwrap();

    // Feature table
    pkg.create_table("Feature", vec![
        msi::Column::build("Feature").primary_key().category(msi::Category::Identifier).string(38),
        msi::Column::build("Feature_Parent").nullable().category(msi::Category::Identifier).string(38),
        msi::Column::build("Title").nullable().string(64),
        msi::Column::build("Description").nullable().string(255),
        msi::Column::build("Display").nullable().int16(),
        msi::Column::build("Level").int16(),
        msi::Column::build("Directory_").nullable().string(72),
        msi::Column::build("Attributes").nullable().int16(),
    ]).unwrap();
    pkg.insert_rows(msi::Insert::into("Feature")
        .row(vec![
            msi::Value::Str("Complete".into()),
            msi::Value::Null,
            msi::Value::Str("Complete Install".into()),
            msi::Value::Str("Installs all files".into()),
            msi::Value::Null,
            msi::Value::Int(1),
            msi::Value::Null,
            msi::Value::Null,
        ])
    ).unwrap();

    // FeatureComponents table
    pkg.create_table("FeatureComponents", vec![
        msi::Column::build("Feature_").primary_key().category(msi::Category::Identifier).string(38),
        msi::Column::build("Component_").primary_key().category(msi::Category::Identifier).string(72),
    ]).unwrap();
    pkg.insert_rows(msi::Insert::into("FeatureComponents")
        .row(vec![msi::Value::Str("Complete".into()), msi::Value::Str("MainComp".into())])
    ).unwrap();

    // File table
    pkg.create_table("File", vec![
        msi::Column::build("File").primary_key().category(msi::Category::Identifier).string(72),
        msi::Column::build("Component_").category(msi::Category::Identifier).string(72),
        msi::Column::build("FileName").category(msi::Category::Filename).string(255),
        msi::Column::build("FileSize").int32(),
        msi::Column::build("Version").nullable().category(msi::Category::Version).string(72),
        msi::Column::build("Language").nullable().category(msi::Category::Language).string(20),
        msi::Column::build("Attributes").nullable().int16(),
        msi::Column::build("Sequence").int16(),
    ]).unwrap();

    let content = b"Hello from Velocity MSI!\r\n";
    pkg.insert_rows(msi::Insert::into("File")
        .row(vec![
            msi::Value::Str("hello.txt".into()),
            msi::Value::Str("MainComp".into()),
            msi::Value::Str("hello.txt".into()),
            msi::Value::Int(content.len() as i32),
            msi::Value::Null,
            msi::Value::Null,
            msi::Value::Null,
            msi::Value::Int(1),
        ])
    ).unwrap();

    // Media table
    pkg.create_table("Media", vec![
        msi::Column::build("DiskId").primary_key().int16(),
        msi::Column::build("LastSequence").string(20),
        msi::Column::build("Cabinet").nullable().category(msi::Category::Cabinet).string(255),
        msi::Column::build("VolumeLabel").nullable().string(32),
        msi::Column::build("DiskPrompt").nullable().string(64),
    ]).unwrap();

    // Build cabinet using velocity-msi's cabinet builder
    let cab_files = vec![
        velocity_msi::CabinetFile {
            name: "hello.txt".to_string(),
            data: content.to_vec(),
        },
    ];
    let cab_data = velocity_msi::build_cabinet(&cab_files);

    pkg.insert_rows(msi::Insert::into("Media")
        .row(vec![
            msi::Value::Int(1),
            msi::Value::Str("1".into()),
            msi::Value::Str("#vel.cab".into()),
            msi::Value::Null,
            msi::Value::Null,
        ])
    ).unwrap();

    // Add cabinet stream
    {
        let mut writer = pkg.write_stream("vel.cab").unwrap();
        std::io::Write::write_all(&mut writer, &cab_data).unwrap();
    }
    pkg.flush().unwrap();

    // InstallExecuteSequence table
    pkg.create_table("InstallExecuteSequence", vec![
        msi::Column::build("Action").primary_key().string(72),
        msi::Column::build("Condition").nullable().string(255),
        msi::Column::build("Sequence").nullable().int16(),
    ]).unwrap();
    pkg.insert_rows(msi::Insert::into("InstallExecuteSequence")
        .row(vec![msi::Value::Str("CostFinalize".into()), msi::Value::Null, msi::Value::Int(1000)])
        .row(vec![msi::Value::Str("CostInitialize".into()), msi::Value::Null, msi::Value::Int(800)])
        .row(vec![msi::Value::Str("FileCost".into()), msi::Value::Null, msi::Value::Int(900)])
        .row(vec![msi::Value::Str("InstallFiles".into()), msi::Value::Null, msi::Value::Int(4000)])
        .row(vec![msi::Value::Str("InstallFinalize".into()), msi::Value::Null, msi::Value::Int(6600)])
        .row(vec![msi::Value::Str("InstallInitialize".into()), msi::Value::Null, msi::Value::Int(1500)])
        .row(vec![msi::Value::Str("InstallValidate".into()), msi::Value::Null, msi::Value::Int(1400)])
    ).unwrap();

    let msi_data = pkg.into_inner().unwrap().into_inner();
    std::fs::write("ref_minimal_test.msi", &msi_data).unwrap();
    println!("Reference MSI written: {} bytes", msi_data.len());
}
