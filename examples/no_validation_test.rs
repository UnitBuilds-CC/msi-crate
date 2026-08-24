/// Test: build MSI without _Validation table to see if it's causing 2705.
/// Uses velocity-msi internals but skips _Validation.
use velocity_msi::{MsiBuilder, Column, Value, encode_stream_name};
use velocity_msi::ole;
use std::process::Command;

fn main() {
    // Build normally first
    let mut b = MsiBuilder::new();
    b.set_title("NoValTest");
    b.set_author("V");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("NoValTest")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("V")],
        vec![Value::from("ProductCode"), Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![Value::from("UpgradeCode"), Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    b.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
    ]).unwrap();

    b.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
    ]).unwrap();

    let msi_data = b.build().unwrap();
    std::fs::write("noval_original.msi", &msi_data).unwrap();
    
    // Test 1: Original (with _Validation)
    println!("=== Test 1: Original (with _Validation) ===");
    let _ = std::fs::remove_file("noval_original.log");
    let output = Command::new("msiexec")
        .args(&["/i", "noval_original.msi", "/qn", "/norestart", "/lv", "noval_original.log"])
        .output().expect("msiexec");
    println!("  exit: {}", output.status.code().unwrap_or(-1));
    
    // Test 2: Remove _Validation stream from the OLE file using cfb
    println!("\n=== Test 2: Remove _Validation stream via cfb ===");
    {
        // Read the MSI with cfb
        let cursor = std::io::Cursor::new(&msi_data);
        let mut comp = cfb::CompoundFile::open(cursor).expect("cfb open");
        
        // List all streams
        let entries: Vec<(String, bool)> = comp.walk()
            .map(|e| (e.name().to_string(), e.is_stream()))
            .collect();
        
        let val_enc = encode_stream_name("_Validation", true);
        println!("  _Validation encoded name: {:?}", val_enc);
        
        // Check if we can find it
        for (name, is_stream) in &entries {
            if *is_stream {
                println!("  stream: {:?}", name);
            }
        }
    }
    
    // Test 3: Build MSI with ONLY Property + Directory (no ExecSeq)
    // This should work (exit 0)
    println!("\n=== Test 3: Property + Directory only (no ExecSeq) ===");
    {
        let mut b = MsiBuilder::new();
        b.set_title("NoExecSeq");
        b.set_author("V");
        b.set_template("Intel", 1033);
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("NoExecSeq")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("Manufacturer"), Value::from("V")],
            vec![Value::from("ProductCode"), Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
            vec![Value::from("UpgradeCode"), Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
        ]).unwrap();
        b.create_table("Directory", vec![
            Column::build("Directory").string(72).primary_key().build(),
            Column::build("Directory_Parent").string(72).nullable().build(),
            Column::build("DefaultDir").string(255).primary_key().build(),
        ]).unwrap();
        b.insert_rows("Directory", vec![
            vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        ]).unwrap();
        let data = b.build().unwrap();
        std::fs::write("noval_noexecseq.msi", &data).unwrap();
        let _ = std::fs::remove_file("noval_noexecseq.log");
        let output = Command::new("msiexec")
            .args(&["/i", "noval_noexecseq.msi", "/qn", "/norestart", "/lv", "noval_noexecseq.log"])
            .output().expect("msiexec");
        println!("  exit: {}", output.status.code().unwrap_or(-1));
    }
    
    // Test 4: Build MSI with Property + ExecSeq (no Directory)
    // CostInitialize should work without Directory
    println!("\n=== Test 4: Property + ExecSeq only (no Directory) ===");
    {
        let mut b = MsiBuilder::new();
        b.set_title("NoDir");
        b.set_author("V");
        b.set_template("Intel", 1033);
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("NoDir")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("Manufacturer"), Value::from("V")],
            vec![Value::from("ProductCode"), Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
            vec![Value::from("UpgradeCode"), Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
        ]).unwrap();
        b.create_table("InstallExecuteSequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("InstallExecuteSequence", vec![
            vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        ]).unwrap();
        let data = b.build().unwrap();
        std::fs::write("noval_nodir.msi", &data).unwrap();
        let _ = std::fs::remove_file("noval_nodir.log");
        let output = Command::new("msiexec")
            .args(&["/i", "noval_nodir.msi", "/qn", "/norestart", "/lv", "noval_nodir.log"])
            .output().expect("msiexec");
        println!("  exit: {}", output.status.code().unwrap_or(-1));
    }
    
    // Test 5: Use the msi crate to read our MSI, then write it back out
    // by creating a new MSI with the same data via msi crate
    println!("\n=== Test 5: msi crate creates MSI with same data ===");
    {
        let cursor = std::io::Cursor::new(Vec::new());
        let mut pkg = msi::Package::create(msi::PackageType::Installer, cursor).expect("create");
        
        pkg.create_table("Property", vec![
            msi::Column::build("Property").primary_key().string(72),
            msi::Column::build("Value").nullable().string(255),
        ]).expect("create Property");
        pkg.insert_rows(msi::Insert::into("Property")
            .row(vec![msi::Value::Str("ProductName".into()), msi::Value::Str("NoValTest".into())])
            .row(vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())])
            .row(vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("V".into())])
            .row(vec![msi::Value::Str("ProductCode".into()), msi::Value::Str("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}".into())])
            .row(vec![msi::Value::Str("UpgradeCode".into()), msi::Value::Str("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}".into())])
            .row(vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())])
        ).expect("insert Property");

        pkg.create_table("Directory", vec![
            msi::Column::build("Directory").primary_key().string(72),
            msi::Column::build("Directory_Parent").nullable().string(72),
            msi::Column::build("DefaultDir").primary_key().string(255),
        ]).expect("create Directory");
        pkg.insert_rows(msi::Insert::into("Directory")
            .row(vec![
                msi::Value::Str("TARGETDIR".into()),
                msi::Value::Null,
                msi::Value::Str("SourceDir".into()),
            ])
        ).expect("insert Directory");

        pkg.create_table("InstallExecuteSequence", vec![
            msi::Column::build("Action").primary_key().string(72),
            msi::Column::build("Condition").nullable().string(255),
            msi::Column::build("Sequence").nullable().int16(),
        ]).expect("create ExecSeq");
        pkg.insert_rows(msi::Insert::into("InstallExecuteSequence")
            .row(vec![
                msi::Value::Str("CostInitialize".into()),
                msi::Value::Null,
                msi::Value::Int(800),
            ])
        ).expect("insert ExecSeq");

        let ref_data = pkg.into_inner().expect("into_inner").into_inner();
        std::fs::write("noval_msi_crate.msi", &ref_data).unwrap();
        
        // Read it back with msi crate to verify Directory
        let cursor2 = std::io::Cursor::new(ref_data.clone());
        let mut pkg2 = msi::Package::open(cursor2).expect("open ref");
        for row in pkg2.select_rows(msi::Select::table("Directory")).expect("read Dir") {
            let dir = row[0].as_str().unwrap_or("?");
            let parent = row[1].as_str();
            let defdir = row[2].as_str().unwrap_or("?");
            println!("  msi crate Directory: Dir={}, Parent={:?}, Default={}", dir, parent, defdir);
        }
        
        let _ = std::fs::remove_file("noval_msi_crate.log");
        let output = Command::new("msiexec")
            .args(&["/i", "noval_msi_crate.msi", "/qn", "/norestart", "/lv", "noval_msi_crate.log"])
            .output().expect("msiexec");
        let code = output.status.code().unwrap_or(-1);
        println!("  msi crate MSI exit: {}", code);
        if code != 0 {
            if let Ok(log) = std::fs::read_to_string("noval_msi_crate.log") {
                for line in log.lines() {
                    if line.contains("1620") || line.contains("2705") || line.contains("DEBUG: Error") {
                        println!("  {}", line.trim());
                    }
                }
            }
        }
    }
}
