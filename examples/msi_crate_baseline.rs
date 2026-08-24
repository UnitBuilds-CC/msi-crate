/// Test: build a complete MSI with Component/File/Feature tables using the msi crate.
/// This establishes the reference baseline for ProcessComponents behavior.
/// cargo run --example msi_crate_baseline -p velocity-msi
use std::io::Cursor;

fn main() {
    let out_path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\msi_crate_baseline.msi";
    
    // Create MSI using the msi crate's Package::create()
    let mut buf = Cursor::new(Vec::new());
    {
        let mut pkg = msi::Package::create(msi::PackageType::Installer, &mut buf).unwrap();

        // Property table
        pkg.create_table("Property", vec![
            msi::Column::build("Property").primary_key().string(72),
            msi::Column::build("Value").nullable().string(255),
        ]).unwrap();
        pkg.insert_rows(
            msi::Insert::into("Property")
                .row(vec![msi::Value::Str("ProductCode".into()), msi::Value::Str("{BBBBBBBB-CCCC-DDDD-EEEE-FFFFFFFFFFFF}".into())])
                .row(vec![msi::Value::Str("ProductName".into()), msi::Value::Str("MSI Crate Baseline".into())])
                .row(vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())])
                .row(vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Test".into())])
                .row(vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())])
        ).unwrap();

        // Directory table
        pkg.create_table("Directory", vec![
            msi::Column::build("Directory").primary_key().string(72),
            msi::Column::build("Directory_Parent").nullable().string(72),
            msi::Column::build("DefaultDir").nullable().string(255),
        ]).unwrap();
        pkg.insert_rows(
            msi::Insert::into("Directory")
                .row(vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str("SourceDir".into())])
                .row(vec![msi::Value::Str("LocalAppDataFolder".into()), msi::Value::Str("TARGETDIR".into()), msi::Value::Str("LocalAppData".into())])
                .row(vec![msi::Value::Str("INSTALLDIR".into()), msi::Value::Str("LocalAppDataFolder".into()), msi::Value::Str("MSICrateTest".into())])
        ).unwrap();

        // Component table - Directory_ NOT nullable, Attributes NOT nullable
        pkg.create_table("Component", vec![
            msi::Column::build("Component").primary_key().string(72),
            msi::Column::build("ComponentId").nullable().string(38),
            msi::Column::build("Directory_").string(72),
            msi::Column::build("Attributes").int16(),
            msi::Column::build("Condition").nullable().string(255),
            msi::Column::build("KeyPath").nullable().string(72),
        ]).unwrap();
        pkg.insert_rows(
            msi::Insert::into("Component")
                .row(vec![
                    msi::Value::Str("comp_0".into()),
                    msi::Value::Null,
                    msi::Value::Str("INSTALLDIR".into()),
                    msi::Value::Int(0),
                    msi::Value::Null,
                    msi::Value::Str("file_0".into()),
                ])
        ).unwrap();

        // File table
        pkg.create_table("File", vec![
            msi::Column::build("File").primary_key().string(72),
            msi::Column::build("Component_").string(72),
            msi::Column::build("FileName").localizable().string(255),
            msi::Column::build("FileSize").int32(),
            msi::Column::build("Version").nullable().string(72),
            msi::Column::build("Language").nullable().string(20),
            msi::Column::build("Attributes").nullable().int16(),
            msi::Column::build("Sequence").int32(),
        ]).unwrap();
        pkg.insert_rows(
            msi::Insert::into("File")
                .row(vec![
                    msi::Value::Str("file_0".into()),
                    msi::Value::Str("comp_0".into()),
                    msi::Value::Str("test.txt".into()),
                    msi::Value::Int(13),
                    msi::Value::Null,
                    msi::Value::Null,
                    msi::Value::Int(0),
                    msi::Value::Int(1),
                ])
        ).unwrap();

        // Media table
        pkg.create_table("Media", vec![
            msi::Column::build("DiskId").primary_key().int16(),
            msi::Column::build("LastSequence").int32(),
            msi::Column::build("DiskPrompt").nullable().string(255),
            msi::Column::build("VolumeLabel").nullable().string(32),
            msi::Column::build("Cabinet").nullable().string(255),
            msi::Column::build("Source").nullable().string(72),
        ]).unwrap();
        pkg.insert_rows(
            msi::Insert::into("Media")
                .row(vec![
                    msi::Value::Int(1),
                    msi::Value::Int(1),
                    msi::Value::Null,
                    msi::Value::Null,
                    msi::Value::Str("#cab.cab".into()),
                    msi::Value::Null,
                ])
        ).unwrap();

        // Feature table
        pkg.create_table("Feature", vec![
            msi::Column::build("Feature").primary_key().string(38),
            msi::Column::build("Feature_Parent").nullable().string(38),
            msi::Column::build("Title").nullable().string(64),
            msi::Column::build("Description").nullable().string(255),
            msi::Column::build("Display").nullable().int16(),
            msi::Column::build("Level").int16(),
            msi::Column::build("Directory_").nullable().string(72),
            msi::Column::build("Attributes").int16(),
        ]).unwrap();
        pkg.insert_rows(
            msi::Insert::into("Feature")
                .row(vec![
                    msi::Value::Str("Complete".into()),
                    msi::Value::Null,
                    msi::Value::Str("Setup".into()),
                    msi::Value::Str("Complete install".into()),
                    msi::Value::Int(1),
                    msi::Value::Int(1),
                    msi::Value::Str("INSTALLDIR".into()),
                    msi::Value::Int(0),
                ])
        ).unwrap();

        // FeatureComponents table
        pkg.create_table("FeatureComponents", vec![
            msi::Column::build("Feature_").primary_key().string(38),
            msi::Column::build("Component_").primary_key().string(72),
        ]).unwrap();
        pkg.insert_rows(
            msi::Insert::into("FeatureComponents")
                .row(vec![
                    msi::Value::Str("Complete".into()),
                    msi::Value::Str("comp_0".into()),
                ])
        ).unwrap();

        // InstallExecuteSequence
        pkg.create_table("InstallExecuteSequence", vec![
            msi::Column::build("Action").primary_key().string(72),
            msi::Column::build("Condition").nullable().string(255),
            msi::Column::build("Sequence").nullable().int16(),
        ]).unwrap();
        pkg.insert_rows(
            msi::Insert::into("InstallExecuteSequence")
                .row(vec![msi::Value::Str("CostInitialize".into()), msi::Value::Null, msi::Value::Int(800)])
                .row(vec![msi::Value::Str("FileCost".into()), msi::Value::Null, msi::Value::Int(850)])
                .row(vec![msi::Value::Str("CostFinalize".into()), msi::Value::Null, msi::Value::Int(1000)])
                .row(vec![msi::Value::Str("InstallValidate".into()), msi::Value::Null, msi::Value::Int(1400)])
                .row(vec![msi::Value::Str("InstallInitialize".into()), msi::Value::Null, msi::Value::Int(1500)])
                .row(vec![msi::Value::Str("ProcessComponents".into()), msi::Value::Null, msi::Value::Int(1600)])
                .row(vec![msi::Value::Str("RemoveFiles".into()), msi::Value::Str("REMOVE=\"ALL\"".into()), msi::Value::Int(1800)])
                .row(vec![msi::Value::Str("InstallFiles".into()), msi::Value::Str("NOT Installed".into()), msi::Value::Int(2000)])
                .row(vec![msi::Value::Str("RegisterProduct".into()), msi::Value::Null, msi::Value::Int(6100)])
                .row(vec![msi::Value::Str("PublishFeatures".into()), msi::Value::Str("NOT Installed".into()), msi::Value::Int(6300)])
                .row(vec![msi::Value::Str("PublishProduct".into()), msi::Value::Str("NOT Installed".into()), msi::Value::Int(6400)])
                .row(vec![msi::Value::Str("InstallFinalize".into()), msi::Value::Null, msi::Value::Int(6600)])
        ).unwrap();

        // InstallUISequence
        pkg.create_table("InstallUISequence", vec![
            msi::Column::build("Action").primary_key().string(72),
            msi::Column::build("Condition").nullable().string(255),
            msi::Column::build("Sequence").nullable().int16(),
        ]).unwrap();
        pkg.insert_rows(
            msi::Insert::into("InstallUISequence")
                .row(vec![msi::Value::Str("CostInitialize".into()), msi::Value::Null, msi::Value::Int(800)])
                .row(vec![msi::Value::Str("CostFinalize".into()), msi::Value::Null, msi::Value::Int(1000)])
                .row(vec![msi::Value::Str("ExecuteAction".into()), msi::Value::Null, msi::Value::Int(1300)])
        ).unwrap();

        // Set summary info
        let si = pkg.summary_info_mut();
        si.set_title("MSI Crate Baseline Test");
        si.set_subject("MSI Crate Baseline v1.0.0");
        si.set_author("Test");
        si.set_comments("Test");
        si.set_creating_application("msi crate");
        si.set_uuid(uuid::Uuid::parse_str("AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE").unwrap());
        si.set_word_count(2);
        si.set_arch("Intel");

        pkg.flush().unwrap();
    }

    let msi_data = buf.into_inner();
    std::fs::write(out_path, &msi_data).unwrap();
    println!("MSI crate baseline created: {} ({} bytes)", out_path, msi_data.len());
    println!("\nNow read it back to verify data:");
    
    // Read back and verify
    let cursor = Cursor::new(msi_data);
    let mut pkg = msi::Package::open(cursor).unwrap();
    
    println!("\n=== Component table ===");
    for row in pkg.select_rows(msi::Select::table("Component")).unwrap() {
        println!("  comp={} id={} dir={} attrs={} cond={} keypath={}",
            row[0].as_str().unwrap_or("?"),
            row[1].as_str().map(|s| s.to_string()).unwrap_or("Null".into()),
            row[2].as_str().unwrap_or("?"),
            row[3].as_int().unwrap_or(-9999),
            row[4].as_str().map(|s| s.to_string()).unwrap_or("Null".into()),
            row[5].as_str().map(|s| s.to_string()).unwrap_or("Null".into()),
        );
    }
    
    println!("\n=== Feature table ===");
    for row in pkg.select_rows(msi::Select::table("Feature")).unwrap() {
        println!("  feature={} level={} attrs={}",
            row[0].as_str().unwrap_or("?"),
            row[5].as_int().unwrap_or(-9999),
            row[7].as_int().unwrap_or(-9999),
        );
    }
    
    println!("\n=== _Columns for Component ===");
    for row in pkg.select_rows(msi::Select::table("_Columns")).unwrap() {
        let table = row[0].as_str().unwrap_or("?");
        if table == "Component" {
            println!("  col {} name={} type=0x{:04X}",
                row[1].as_int().unwrap_or(0),
                row[2].as_str().unwrap_or("?"),
                row[3].as_int().unwrap_or(0) as u16);
        }
    }
    
    println!("\nTest with:");
    println!("  msiexec /i \"{}\" /qn /l*v \"{}\"", out_path, out_path.replace(".msi", ".log"));
}
