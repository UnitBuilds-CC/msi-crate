/// Incremental test: find minimum required MSI content.
/// Start with minimal MSI and add things until it works.
use std::io::Cursor;

fn test_msi(label: &str, data: &[u8]) {
    let path = format!("C:\\temp\\test_{}.msi", label);
    std::fs::write(&path, data).unwrap();
    let output = std::process::Command::new("msiexec.exe")
        .args(&["/i", &path, "/quiet", "/norestart"])
        .output().unwrap();
    let code = output.status.code().unwrap_or(-1);
    let status = match code {
        0 => "SUCCESS (installed!)",
        1620 => "FAIL (can't open)",
        1625 => "OK (opens, blocked by policy)",
        _ => "OTHER",
    };
    println!("  {} -> exit {} ({})", label, code, status);
}

fn main() {
    std::fs::create_dir_all("C:\\temp").ok();

    // TEST 1: Minimal MSI with all SummaryInfo properties
    println!("=== TEST 1: Minimal + all SummaryInfo ===");
    {
        let cursor = Cursor::new(Vec::new());
        let mut pkg = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
        pkg.set_database_codepage(msi::CodePage::Windows1252);

        let columns = vec![
            msi::Column::build("Property").primary_key().id_string(72),
            msi::Column::build("Value").nullable().string(0),
        ];
        pkg.create_table("Property", columns).unwrap();
        pkg.insert_rows(msi::Insert::into("Property")
            .row(vec![msi::Value::from("ProductName"), msi::Value::from("Test Product")])
            .row(vec![msi::Value::from("ProductVersion"), msi::Value::from("1.0.0")])
            .row(vec![msi::Value::from("Manufacturer"), msi::Value::from("Test Corp")])
            .row(vec![msi::Value::from("ProductCode"), msi::Value::from("{12345678-1234-1234-1234-123456789012}")])
            .row(vec![msi::Value::from("ProductLanguage"), msi::Value::from("1033")])
            .row(vec![msi::Value::from("UpgradeCode"), msi::Value::from("{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}")])
        ).unwrap();

        let si = pkg.summary_info_mut();
        si.set_arch("Intel");
        si.set_languages(&[msi::Language::from_tag("en-US")]);
        si.set_codepage(msi::CodePage::Windows1252);
        si.set_word_count(2);
        si.set_author("Test".to_string());
        si.set_subject("Test".to_string());
        si.set_comments("Test".to_string());
        si.set_creating_application("Test".to_string());

        let data = pkg.into_inner().unwrap().into_inner();
        test_msi("t1", &data);
    }

    // TEST 2: Add Directory table
    println!("\n=== TEST 2: + Directory table ===");
    {
        let cursor = Cursor::new(Vec::new());
        let mut pkg = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
        pkg.set_database_codepage(msi::CodePage::Windows1252);

        // Property table
        pkg.create_table("Property", vec![
            msi::Column::build("Property").primary_key().id_string(72),
            msi::Column::build("Value").nullable().string(0),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Property")
            .row(vec![msi::Value::from("ProductName"), msi::Value::from("Test Product")])
            .row(vec![msi::Value::from("ProductVersion"), msi::Value::from("1.0.0")])
            .row(vec![msi::Value::from("Manufacturer"), msi::Value::from("Test Corp")])
            .row(vec![msi::Value::from("ProductCode"), msi::Value::from("{12345678-1234-1234-1234-123456789012}")])
            .row(vec![msi::Value::from("ProductLanguage"), msi::Value::from("1033")])
        ).unwrap();

        // Directory table
        pkg.create_table("Directory", vec![
            msi::Column::build("Directory").primary_key().id_string(72),
            msi::Column::build("Directory_Parent").nullable().id_string(72),
            msi::Column::build("DefaultDir").nullable().text_string(255),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Directory")
            .row(vec![msi::Value::from("TARGETDIR"), msi::Value::Null, msi::Value::from("SourceDir")])
            .row(vec![msi::Value::from("ProgramFilesFolder"), msi::Value::from("TARGETDIR"), msi::Value::from(".")])
        ).unwrap();

        let si = pkg.summary_info_mut();
        si.set_arch("Intel");
        si.set_languages(&[msi::Language::from_tag("en-US")]);
        si.set_codepage(msi::CodePage::Windows1252);
        si.set_word_count(2);

        let data = pkg.into_inner().unwrap().into_inner();
        test_msi("t2", &data);
    }

    // TEST 3: Add Component + Feature + FeatureComponents
    println!("\n=== TEST 3: + Component/Feature tables ===");
    {
        let cursor = Cursor::new(Vec::new());
        let mut pkg = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
        pkg.set_database_codepage(msi::CodePage::Windows1252);

        pkg.create_table("Property", vec![
            msi::Column::build("Property").primary_key().id_string(72),
            msi::Column::build("Value").nullable().string(0),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Property")
            .row(vec![msi::Value::from("ProductName"), msi::Value::from("Test Product")])
            .row(vec![msi::Value::from("ProductVersion"), msi::Value::from("1.0.0")])
            .row(vec![msi::Value::from("Manufacturer"), msi::Value::from("Test Corp")])
            .row(vec![msi::Value::from("ProductCode"), msi::Value::from("{12345678-1234-1234-1234-123456789012}")])
            .row(vec![msi::Value::from("ProductLanguage"), msi::Value::from("1033")])
        ).unwrap();

        pkg.create_table("Directory", vec![
            msi::Column::build("Directory").primary_key().id_string(72),
            msi::Column::build("Directory_Parent").nullable().id_string(72),
            msi::Column::build("DefaultDir").nullable().text_string(255),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Directory")
            .row(vec![msi::Value::from("TARGETDIR"), msi::Value::Null, msi::Value::from("SourceDir")])
            .row(vec![msi::Value::from("ProgramFilesFolder"), msi::Value::from("TARGETDIR"), msi::Value::from(".")])
        ).unwrap();

        pkg.create_table("Component", vec![
            msi::Column::build("Component").primary_key().id_string(72),
            msi::Column::build("ComponentId").nullable().id_string(38),
            msi::Column::build("Directory_").id_string(72),
            msi::Column::build("Attributes").int16(),
            msi::Column::build("Condition").nullable().text_string(255),
            msi::Column::build("KeyPath").nullable().id_string(72),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Component")
            .row(vec![msi::Value::from("MainComponent"), msi::Value::Null, msi::Value::from("ProgramFilesFolder"), msi::Value::Int(0), msi::Value::Null, msi::Value::Null])
        ).unwrap();

        pkg.create_table("Feature", vec![
            msi::Column::build("Feature").primary_key().id_string(38),
            msi::Column::build("Feature_Parent").nullable().id_string(38),
            msi::Column::build("Title").nullable().text_string(64),
            msi::Column::build("Description").nullable().text_string(255),
            msi::Column::build("Display").nullable().int16(),
            msi::Column::build("Level").int16(),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Feature")
            .row(vec![msi::Value::from("MainFeature"), msi::Value::Null, msi::Value::from("Complete"), msi::Value::from("Full installation"), msi::Value::Int(1), msi::Value::Int(1)])
        ).unwrap();

        pkg.create_table("FeatureComponents", vec![
            msi::Column::build("Feature_").primary_key().id_string(38),
            msi::Column::build("Component_").primary_key().id_string(72),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("FeatureComponents")
            .row(vec![msi::Value::from("MainFeature"), msi::Value::from("MainComponent")])
        ).unwrap();

        let si = pkg.summary_info_mut();
        si.set_arch("Intel");
        si.set_languages(&[msi::Language::from_tag("en-US")]);
        si.set_codepage(msi::CodePage::Windows1252);
        si.set_word_count(2);

        let data = pkg.into_inner().unwrap().into_inner();
        test_msi("t3", &data);
    }

    // TEST 4: Same as 3 but with Media table
    println!("\n=== TEST 4: + Media table ===");
    {
        let cursor = Cursor::new(Vec::new());
        let mut pkg = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
        pkg.set_database_codepage(msi::CodePage::Windows1252);

        pkg.create_table("Property", vec![
            msi::Column::build("Property").primary_key().id_string(72),
            msi::Column::build("Value").nullable().string(0),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Property")
            .row(vec![msi::Value::from("ProductName"), msi::Value::from("Test Product")])
            .row(vec![msi::Value::from("ProductVersion"), msi::Value::from("1.0.0")])
            .row(vec![msi::Value::from("Manufacturer"), msi::Value::from("Test Corp")])
            .row(vec![msi::Value::from("ProductCode"), msi::Value::from("{12345678-1234-1234-1234-123456789012}")])
            .row(vec![msi::Value::from("ProductLanguage"), msi::Value::from("1033")])
        ).unwrap();

        pkg.create_table("Directory", vec![
            msi::Column::build("Directory").primary_key().id_string(72),
            msi::Column::build("Directory_Parent").nullable().id_string(72),
            msi::Column::build("DefaultDir").nullable().text_string(255),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Directory")
            .row(vec![msi::Value::from("TARGETDIR"), msi::Value::Null, msi::Value::from("SourceDir")])
        ).unwrap();

        pkg.create_table("Component", vec![
            msi::Column::build("Component").primary_key().id_string(72),
            msi::Column::build("ComponentId").nullable().id_string(38),
            msi::Column::build("Directory_").id_string(72),
            msi::Column::build("Attributes").int16(),
            msi::Column::build("Condition").nullable().text_string(255),
            msi::Column::build("KeyPath").nullable().id_string(72),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Component")
            .row(vec![msi::Value::from("MainComponent"), msi::Value::Null, msi::Value::from("TARGETDIR"), msi::Value::Int(0), msi::Value::Null, msi::Value::Null])
        ).unwrap();

        pkg.create_table("Feature", vec![
            msi::Column::build("Feature").primary_key().id_string(38),
            msi::Column::build("Feature_Parent").nullable().id_string(38),
            msi::Column::build("Title").nullable().text_string(64),
            msi::Column::build("Description").nullable().text_string(255),
            msi::Column::build("Display").nullable().int16(),
            msi::Column::build("Level").int16(),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Feature")
            .row(vec![msi::Value::from("MainFeature"), msi::Value::Null, msi::Value::Null, msi::Value::Null, msi::Value::Null, msi::Value::Int(1)])
        ).unwrap();

        pkg.create_table("FeatureComponents", vec![
            msi::Column::build("Feature_").primary_key().id_string(38),
            msi::Column::build("Component_").primary_key().id_string(72),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("FeatureComponents")
            .row(vec![msi::Value::from("MainFeature"), msi::Value::from("MainComponent")])
        ).unwrap();

        pkg.create_table("Media", vec![
            msi::Column::build("DiskId").primary_key().int16(),
            msi::Column::build("LastSequence").int32(),
            msi::Column::build("DiskPrompt").nullable().text_string(64),
            msi::Column::build("Cabinet").nullable().text_string(255),
            msi::Column::build("VolumeLabel").nullable().text_string(32),
            msi::Column::build("Source").nullable().id_string(72),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Media")
            .row(vec![msi::Value::Int(1), msi::Value::Int(0), msi::Value::Null, msi::Value::Null, msi::Value::Null, msi::Value::Null])
        ).unwrap();

        let si = pkg.summary_info_mut();
        si.set_arch("Intel");
        si.set_languages(&[msi::Language::from_tag("en-US")]);
        si.set_codepage(msi::CodePage::Windows1252);
        si.set_word_count(2);

        let data = pkg.into_inner().unwrap().into_inner();
        test_msi("t4", &data);
    }

    // TEST 5: Back to absolute minimum - just Property with WordCount=2 and Win-1252
    println!("\n=== TEST 5: Property only, WordCount=2, Win-1252 SummaryInfo ===");
    {
        let cursor = Cursor::new(Vec::new());
        let mut pkg = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
        pkg.set_database_codepage(msi::CodePage::Windows1252);

        pkg.create_table("Property", vec![
            msi::Column::build("Property").primary_key().id_string(72),
            msi::Column::build("Value").nullable().string(0),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Property")
            .row(vec![msi::Value::from("ProductName"), msi::Value::from("Test Product")])
            .row(vec![msi::Value::from("ProductCode"), msi::Value::from("{12345678-1234-1234-1234-123456789012}")])
            .row(vec![msi::Value::from("ProductLanguage"), msi::Value::from("1033")])
            .row(vec![msi::Value::from("Manufacturer"), msi::Value::from("Test")])
            .row(vec![msi::Value::from("ProductVersion"), msi::Value::from("1.0")])
        ).unwrap();

        // Set ALL SummaryInfo properties to match system MSI
        let si = pkg.summary_info_mut();
        si.set_title("Installation Database");
        si.set_subject("Test");
        si.set_author("Test");
        si.set_comments("Test");
        si.set_arch("Intel");
        si.set_languages(&[msi::Language::from_tag("en-US")]);
        si.set_codepage(msi::CodePage::Windows1252);
        si.set_word_count(2);
        si.set_creating_application("Test");

        let data = pkg.into_inner().unwrap().into_inner();
        test_msi("t5", &data);
    }

    // TEST 6: Same as 5 but also set UUID
    println!("\n=== TEST 6: + UUID ===");
    {
        let cursor = Cursor::new(Vec::new());
        let mut pkg = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
        pkg.set_database_codepage(msi::CodePage::Windows1252);

        pkg.create_table("Property", vec![
            msi::Column::build("Property").primary_key().id_string(72),
            msi::Column::build("Value").nullable().string(0),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Property")
            .row(vec![msi::Value::from("ProductName"), msi::Value::from("Test Product")])
            .row(vec![msi::Value::from("ProductCode"), msi::Value::from("{12345678-1234-1234-1234-123456789012}")])
            .row(vec![msi::Value::from("ProductLanguage"), msi::Value::from("1033")])
            .row(vec![msi::Value::from("Manufacturer"), msi::Value::from("Test")])
            .row(vec![msi::Value::from("ProductVersion"), msi::Value::from("1.0")])
        ).unwrap();

        let si = pkg.summary_info_mut();
        si.set_title("Installation Database");
        si.set_subject("Test");
        si.set_author("Test");
        si.set_comments("Test");
        si.set_arch("Intel");
        si.set_languages(&[msi::Language::from_tag("en-US")]);
        si.set_codepage(msi::CodePage::Windows1252);
        si.set_word_count(2);
        si.set_creating_application("Test");
        si.set_uuid(uuid::Uuid::parse_str("12345678-1234-1234-1234-123456789012").unwrap());

        let data = pkg.into_inner().unwrap().into_inner();
        test_msi("t6", &data);
    }

    println!("\nDone!");
}
