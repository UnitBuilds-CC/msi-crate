/// Progressive test: start from definitive_test base and add compiler tables
/// one group at a time until msiexec returns 1620.
use velocity_msi::{Column, MsiBuilder, Value};

fn make_uuid() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let a = (t & 0xFFFFFFFF) as u32;
    let b = ((t >> 32) & 0xFFFF) as u16;
    let c = (((t >> 48) & 0x0FFF) as u16) | 0x4000;
    let d = ((t >> 64) as u16 & 0x3FFF) | 0x8000;
    let e = (t >> 80) as u64;
    format!("{{{:08X}-{:04X}-{:04X}-{:04X}-{:012X}}}", a, b, c, d, e & 0xFFFFFFFFFFFF)
}

fn test_msi(label: &str, builder: &mut MsiBuilder) {
    let out = format!("C:\\temp\\prog_{}.msi", label);
    let log = format!("C:\\temp\\prog_{}.log", label);
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&log);
    
    let data = builder.build().unwrap();
    std::fs::write(&out, &data).unwrap();
    
    let output = std::process::Command::new("msiexec")
        .args(&["/i", &out, "/qn", "/norestart", "/l*v", &log])
        .output()
        .unwrap();
    let code = output.status.code().unwrap_or(-1);
    println!("  {} => exit code {} ({} bytes)", label, code, data.len());
    
    if code != 0 {
        // Read log for error
        if let Ok(log_text) = std::fs::read_to_string(&log) {
            for line in log_text.lines() {
                if line.contains("return value 3") || line.contains("Error 1620") || line.contains("CustomAction") {
                    println!("    LOG: {}", line.trim());
                }
            }
        }
    }
    
    // Cleanup
    let _ = std::process::Command::new("msiexec")
        .args(&["/x", &out, "/qn", "/norestart"])
        .output();
}

fn base_builder() -> MsiBuilder {
    let mut b = MsiBuilder::new();
    b.set_title("Progressive Test");
    b.set_author("V");
    b.set_subject("Test");
    b.set_template("Intel", 1033);
    
    // Property table
    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    
    let pc = make_uuid();
    let uc = make_uuid();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Prog Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("V")],
        vec![Value::from("ProductCode"), Value::from(pc.as_str())],
        vec![Value::from("UpgradeCode"), Value::from(uc.as_str())],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();
    
    // Directory table
    b.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("ProgramFilesFolder"), Value::from("TARGETDIR"), Value::from("PFiles")],
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFilesFolder"), Value::from("ProgTest")],
    ]).unwrap();
    
    // Component table
    b.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(38).nullable().build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
    b.insert_rows("Component", vec![
        vec![Value::from("MainComp"), Value::Null, Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::Null],
    ]).unwrap();
    
    // Feature table
    b.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").string(38).nullable().build(),
        Column::build("Title").string(64).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().build(),
    ]).unwrap();
    b.insert_rows("Feature", vec![
        vec![Value::from("Complete"), Value::Null, Value::from("Setup"), Value::Null, Value::Int(1), Value::Int(1), Value::from("INSTALLDIR"), Value::Int(0)],
    ]).unwrap();
    
    // FeatureComponents
    b.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    b.insert_rows("FeatureComponents", vec![
        vec![Value::from("Complete"), Value::from("MainComp")],
    ]).unwrap();
    
    // Media
    b.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int16().build(),
        Column::build("DiskPrompt").string(64).nullable().build(),
        Column::build("Cabinet").string(255).nullable().build(),
        Column::build("VolumeLabel").string(32).nullable().build(),
        Column::build("Source").string(72).nullable().build(),
    ]).unwrap();
    b.insert_rows("Media", vec![
        vec![Value::Int(1), Value::Int(0), Value::Null, Value::Null, Value::Null, Value::Null],
    ]).unwrap();
    
    b
}

fn main() {
    println!("=== PROGRESSIVE TABLE TEST ===\n");
    
    // Phase 1: Base (Property + Directory + Component + Feature + FC + Media)
    println!("Phase 1: Base tables only");
    {
        let mut b = base_builder();
        b.create_table("InstallExecuteSequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("InstallExecuteSequence", vec![
            vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
            vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
            vec![Value::from("InstallValidate"), Value::Null, Value::Int(1400)],
            vec![Value::from("InstallInitialize"), Value::Null, Value::Int(1500)],
            vec![Value::from("InstallFinalize"), Value::Null, Value::Int(6600)],
        ]).unwrap();
        test_msi("base", &mut b);
    }
    
    // Phase 2: Add InstallUISequence (minimal)
    println!("\nPhase 2: + minimal InstallUISequence");
    {
        let mut b = base_builder();
        b.create_table("InstallExecuteSequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("InstallExecuteSequence", vec![
            vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
            vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
            vec![Value::from("InstallValidate"), Value::Null, Value::Int(1400)],
            vec![Value::from("InstallInitialize"), Value::Null, Value::Int(1500)],
            vec![Value::from("InstallFinalize"), Value::Null, Value::Int(6600)],
        ]).unwrap();
        
        b.create_table("InstallUISequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("InstallUISequence", vec![
            vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
            vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
            vec![Value::from("ExecuteAction"), Value::Null, Value::Int(1300)],
        ]).unwrap();
        test_msi("ui_min", &mut b);
    }
    
    // Phase 3: Add compiler-style InstallUISequence (with dialogs)
    println!("\nPhase 3: + compiler-style InstallUISequence (with dialogs)");
    {
        let mut b = base_builder();
        b.create_table("InstallExecuteSequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("InstallExecuteSequence", vec![
            vec![Value::from("CostInitialize"), Value::Null, Value::Int(120)],
            vec![Value::from("CostFinalize"), Value::Null, Value::Int(140)],
            vec![Value::from("InstallValidate"), Value::Null, Value::Int(150)],
            vec![Value::from("InstallInitialize"), Value::Null, Value::Int(160)],
            vec![Value::from("InstallFinalize"), Value::Null, Value::Int(400)],
        ]).unwrap();
        
        b.create_table("InstallUISequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("InstallUISequence", vec![
            vec![Value::from("ShowLog"), Value::Null, Value::Int(-1)],
            vec![Value::from("ProgressDlg"), Value::Null, Value::Int(10)],
            vec![Value::from("CancelDlg"), Value::Null, Value::Int(15)],
            vec![Value::from("ErrorDlg"), Value::Null, Value::Int(50)],
            vec![Value::from("FatalError"), Value::Null, Value::Int(9999)],
            vec![Value::from("UserExit"), Value::Null, Value::Int(9999)],
            vec![Value::from("WelcomeDlg"), Value::from("NOT Installed"), Value::Int(1230)],
            vec![Value::from("LicenseAgreementDlg"), Value::from("NOT Installed"), Value::Int(1235)],
            vec![Value::from("InstallDirDlg"), Value::from("NOT Installed"), Value::Int(1240)],
            vec![Value::from("VerifyReadyDlg"), Value::from("NOT Installed"), Value::Int(1250)],
            vec![Value::from("ActionText"), Value::Null, Value::Int(20)],
            vec![Value::from("ExecuteAction"), Value::Null, Value::Int(1300)],
        ]).unwrap();
        test_msi("ui_full", &mut b);
    }
    
    // Phase 4: Add CustomAction table (empty)
    println!("\nPhase 4: + empty CustomAction table");
    {
        let mut b = base_builder();
        b.create_table("InstallExecuteSequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("InstallExecuteSequence", vec![
            vec![Value::from("CostInitialize"), Value::Null, Value::Int(120)],
            vec![Value::from("CostFinalize"), Value::Null, Value::Int(140)],
            vec![Value::from("InstallFinalize"), Value::Null, Value::Int(400)],
        ]).unwrap();
        
        b.create_table("CustomAction", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Type").int16().nullable().build(),
            Column::build("Source").string(72).nullable().build(),
            Column::build("Target").string(255).nullable().build(),
        ]).unwrap();
        // Don't insert any rows
        test_msi("empty_ca", &mut b);
    }
    
    // Phase 5: Add LaunchCondition table (empty)
    println!("\nPhase 5: + empty LaunchCondition table");
    {
        let mut b = base_builder();
        b.create_table("InstallExecuteSequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("InstallExecuteSequence", vec![
            vec![Value::from("CostInitialize"), Value::Null, Value::Int(120)],
            vec![Value::from("CostFinalize"), Value::Null, Value::Int(140)],
            vec![Value::from("InstallFinalize"), Value::Null, Value::Int(400)],
        ]).unwrap();
        
        b.create_table("LaunchCondition", vec![
            Column::build("Condition").string(255).primary_key().build(),
            Column::build("Description").string(255).nullable().build(),
        ]).unwrap();
        test_msi("empty_lc", &mut b);
    }
    
    // Phase 6: Add Upgrade table (empty)
    println!("\nPhase 6: + empty Upgrade table");
    {
        let mut b = base_builder();
        b.create_table("InstallExecuteSequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("InstallExecuteSequence", vec![
            vec![Value::from("CostInitialize"), Value::Null, Value::Int(120)],
            vec![Value::from("CostFinalize"), Value::Null, Value::Int(140)],
            vec![Value::from("InstallFinalize"), Value::Null, Value::Int(400)],
        ]).unwrap();
        
        b.create_table("Upgrade", vec![
            Column::build("UpgradeCode").string(38).primary_key().build(),
            Column::build("VersionMin").string(20).nullable().build(),
            Column::build("VersionMax").string(20).nullable().build(),
            Column::build("Language").string(20).nullable().build(),
            Column::build("Attributes").int32().nullable().build(),
        ]).unwrap();
        test_msi("empty_upg", &mut b);
    }
    
    // Phase 7: Add compiler ExecSeq (full standard actions)
    println!("\nPhase 7: + full compiler ExecSeq actions");
    {
        let mut b = base_builder();
        b.create_table("InstallExecuteSequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("InstallExecuteSequence", vec![
            vec![Value::from("AppSearch"), Value::Null, Value::Int(100)],
            vec![Value::from("LaunchConditions"), Value::from("NOT Installed"), Value::Int(105)],
            vec![Value::from("ValidateProductID"), Value::Null, Value::Int(110)],
            vec![Value::from("CostInitialize"), Value::Null, Value::Int(120)],
            vec![Value::from("FileCost"), Value::Null, Value::Int(130)],
            vec![Value::from("CostFinalize"), Value::Null, Value::Int(140)],
            vec![Value::from("InstallValidate"), Value::Null, Value::Int(150)],
            vec![Value::from("InstallInitialize"), Value::Null, Value::Int(160)],
            vec![Value::from("ProcessComponents"), Value::Null, Value::Int(170)],
            vec![Value::from("InstallFiles"), Value::Null, Value::Int(200)],
            vec![Value::from("RegisterProduct"), Value::Null, Value::Int(300)],
            vec![Value::from("PublishProduct"), Value::Null, Value::Int(310)],
            vec![Value::from("InstallFinalize"), Value::Null, Value::Int(400)],
        ]).unwrap();
        
        b.create_table("InstallUISequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("InstallUISequence", vec![
            vec![Value::from("ExecuteAction"), Value::Null, Value::Int(1300)],
        ]).unwrap();
        test_msi("full_exec", &mut b);
    }
    
    println!("\n=== DONE ===");
}
