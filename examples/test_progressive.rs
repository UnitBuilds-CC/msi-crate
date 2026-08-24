//! Progressive test: add compiler-specific tables one by one to find which breaks.

use std::process::Command;
use velocity_msi::{MsiBuilder, Column, Value, CabinetFile, build_cabinet};

fn build_base_msi(extra_tables: &[(&str, Vec<Column>, Vec<Vec<Value>>)]) -> Vec<u8> {
    let test_content = b"Hello from Velocity MSI!";
    let test_file_name = "velocity_test.txt";

    let cabinet = build_cabinet(&[
        CabinetFile { name: "F1".to_string(), data: test_content.to_vec() },
    ]);

    let mut b = MsiBuilder::new();
    b.set_title("Test");
    b.set_author("Test");
    b.set_subject("Test");
    b.set_comments("Test");
    b.set_template("x64", 1033);

    // Base tables (known working)
    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).build(),
    ]).unwrap();
    b.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(72).nullable().build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
    b.create_table("File", vec![
        Column::build("File").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).build(),
        Column::build("FileSize").int32().build(),
        Column::build("Version").string(72).nullable().build(),
        Column::build("Language").int16().nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
        Column::build("Sequence").int32().build(),
    ]).unwrap();
    b.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int32().build(),
        Column::build("DiskPrompt").string(255).nullable().build(),
        Column::build("VolumeLabel").string(32).nullable().build(),
        Column::build("Cabinet").string(255).nullable().build(),
        Column::build("Source").string(72).nullable().build(),
    ]).unwrap();
    b.create_table("Feature", vec![
        Column::build("Feature").string(72).primary_key().build(),
        Column::build("Feature_Parent").string(72).nullable().build(),
        Column::build("Title").string(255).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().build(),
    ]).unwrap();
    b.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(72).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    b.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.create_table("InstallUISequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();

    // Populate base data
    b.insert_rows("Property", vec![
        vec![Value::from("ProductCode"), Value::from("{12345678-1234-5678-9ABC-DEF012345678}")],
        vec![Value::from("ProductName"), Value::from("Test Product")],
        vec![Value::from("Manufacturer"), Value::from("Test Mfg")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("TestDir")],
    ]).unwrap();
    b.insert_rows("Component", vec![
        vec![Value::from("MainComp"), Value::Null, Value::from("INSTALLDIR"),
             Value::from(0i32), Value::Null, Value::from("F1")],
    ]).unwrap();
    b.insert_rows("File", vec![
        vec![Value::from("F1"), Value::from("MainComp"), Value::from(test_file_name),
             Value::from(test_content.len() as i32), Value::Null, Value::Null,
             Value::from(0i32), Value::from(1i32)],
    ]).unwrap();
    b.insert_rows("Media", vec![
        vec![Value::from(1i32), Value::from(1i32), Value::Null, Value::Null,
             Value::from("#velo.cab"), Value::Null],
    ]).unwrap();
    b.insert_rows("Feature", vec![
        vec![Value::from("MainFeature"), Value::Null, Value::from("Main"),
             Value::from("Main feature"), Value::from(1i32), Value::from(1i32),
             Value::Null, Value::from(0i32)],
    ]).unwrap();
    b.insert_rows("FeatureComponents", vec![
        vec![Value::from("MainFeature"), Value::from("MainComp")],
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::from(800i32)],
        vec![Value::from("CostFinalize"), Value::Null, Value::from(1000i32)],
        vec![Value::from("InstallValidate"), Value::Null, Value::from(1400i32)],
        vec![Value::from("InstallInitialize"), Value::Null, Value::from(1500i32)],
        vec![Value::from("InstallFiles"), Value::Null, Value::from(4000i32)],
        vec![Value::from("InstallFinalize"), Value::Null, Value::from(6600i32)],
    ]).unwrap();
    b.insert_rows("InstallUISequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::from(800i32)],
        vec![Value::from("CostFinalize"), Value::Null, Value::from(1000i32)],
    ]).unwrap();

    // Add extra tables
    for (name, columns, rows) in extra_tables {
        b.create_table(name, columns.clone()).unwrap();
        if !rows.is_empty() {
            b.insert_rows(name, rows.clone()).unwrap();
        }
    }

    // Embed cabinet
    b.add_stream("velo.cab".to_string(), cabinet);
    b.build().unwrap()
}

fn test_msi(name: &str, msi_data: &[u8]) -> i32 {
    let msi_dir = "C:\\temp\\vel_msi_test";
    std::fs::create_dir_all(msi_dir).ok();
    let path = format!("{}\\{}.msi", msi_dir, name);
    std::fs::write(&path, msi_data).unwrap();
    
    let install_dir = format!("C:\\temp\\vel_msi_install\\{}", name);
    let log_path = format!("{}\\{}.log", msi_dir, name);
    let status = Command::new("msiexec")
        .args(&["/i", &path, "/qn", "/l*v", &log_path, &format!("TARGETDIR={}", install_dir)])
        .status().unwrap();
    let code = status.code().unwrap_or(-1);
    println!("  {} => exit code: {}", name, code);
    
    if code != 0 {
        // Print last 20 lines of log
        if let Ok(log) = std::fs::read_to_string(&log_path) {
            let lines: Vec<&str> = log.lines().collect();
            let start = lines.len().saturating_sub(20);
            for line in &lines[start..] {
                println!("    {}", line);
            }
        }
    }
    
    if code == 0 {
        let _ = Command::new("msiexec").args(&["/x", &path, "/qn"]).status();
    }
    let _ = std::fs::remove_dir_all(&install_dir);
    code
}

fn main() {
    std::fs::create_dir_all("C:\\temp\\vel_msi_test").ok();

    // Phase 0: Base only (known working)
    println!("=== Phase 0: Base tables (known working) ===");
    let base = build_base_msi(&[]);
    let code = test_msi("phase0_base", &base);
    if code != 0 {
        println!("BASE IS BROKEN! Fix this first.");
        return;
    }

    // Phase 1: + Registry
    println!("\n=== Phase 1: + Registry ===");
    let msi = build_base_msi(&[
        ("Registry", vec![
            Column::build("Registry").string(72).primary_key().build(),
            Column::build("Root").int16().nullable().build(),
            Column::build("Key").string(255).nullable().build(),
            Column::build("Name").string(255).nullable().build(),
            Column::build("Value").string(255).nullable().build(),
            Column::build("Component_").string(72).nullable().build(),
        ], vec![
            vec![Value::from("Reg1"), Value::from(2i32), Value::from("Software\\TestApp"),
                 Value::from("Path"), Value::from("C:\\Test"), Value::from("MainComp")],
        ]),
    ]);
    test_msi("phase1_registry", &msi);

    // Phase 2: + CustomAction
    println!("\n=== Phase 2: + CustomAction ===");
    let msi = build_base_msi(&[
        ("CustomAction", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Type").int16().nullable().build(),
            Column::build("Source").string(72).nullable().build(),
            Column::build("Target").string(255).nullable().build(),
        ], vec![
            vec![Value::from("ca1"), Value::from(51i32), Value::from("TARGETDIR"), Value::from("C:\\Test")],
        ]),
    ]);
    test_msi("phase2_ca", &msi);

    // Phase 3: + Environment
    println!("\n=== Phase 3: + Environment ===");
    let msi = build_base_msi(&[
        ("Environment", vec![
            Column::build("Environment").string(72).primary_key().build(),
            Column::build("Name").string(255).nullable().build(),
            Column::build("Value").string(255).nullable().build(),
            Column::build("Component_").string(72).nullable().build(),
        ], vec![
            vec![Value::from("Env1"), Value::from("TEST_VAR"), Value::from("test_value"), Value::from("MainComp")],
        ]),
    ]);
    test_msi("phase3_env", &msi);

    // Phase 4: + Shortcut
    println!("\n=== Phase 4: + Shortcut ===");
    let msi = build_base_msi(&[
        ("Shortcut", vec![
            Column::build("Shortcut").string(72).primary_key().build(),
            Column::build("Directory_").string(72).nullable().build(),
            Column::build("Name").string(128).nullable().build(),
            Column::build("Component_").string(72).nullable().build(),
            Column::build("Target").string(255).nullable().build(),
            Column::build("Arguments").string(255).nullable().build(),
            Column::build("Description").string(255).nullable().build(),
            Column::build("Hotkey").int16().nullable().build(),
            Column::build("Icon_").string(72).nullable().build(),
            Column::build("IconIndex").int16().nullable().build(),
            Column::build("ShowCmd").int16().nullable().build(),
            Column::build("WkDir").string(72).nullable().build(),
        ], vec![
            vec![Value::from("SC1"), Value::from("INSTALLDIR"), Value::from("Test SC"),
                 Value::from("MainComp"), Value::from("[INSTALLDIR]test.txt"),
                 Value::Null, Value::from("A shortcut"), Value::Null,
                 Value::Null, Value::Null, Value::from(1i32), Value::from("INSTALLDIR")],
        ]),
    ]);
    test_msi("phase4_shortcut", &msi);

    // Phase 5: + Icon (with null data)
    println!("\n=== Phase 5: + Icon ===");
    let msi = build_base_msi(&[
        ("Icon", vec![
            Column::build("Name").string(72).primary_key().build(),
            Column::build("Data").binary().nullable().build(),
        ], vec![
            vec![Value::from("AppIcon.ico"), Value::Null],
        ]),
    ]);
    test_msi("phase5_icon", &msi);

    // Phase 6: + Upgrade
    println!("\n=== Phase 6: + Upgrade ===");
    let msi = build_base_msi(&[
        ("Upgrade", vec![
            Column::build("UpgradeCode").string(38).primary_key().build(),
            Column::build("VersionMin").string(20).nullable().build(),
            Column::build("VersionMax").string(20).nullable().build(),
            Column::build("Language").string(20).nullable().build(),
            Column::build("Attributes").int32().nullable().build(),
        ], vec![
            vec![Value::from("{AAAAAAAA-1111-2222-3333-444455556666}"),
                 Value::from("1.0.0"), Value::from("1.0.0"), Value::Null, Value::from(259i32)],
        ]),
    ]);
    test_msi("phase6_upgrade", &msi);

    // Phase 7: + LaunchCondition
    println!("\n=== Phase 7: + LaunchCondition ===");
    let msi = build_base_msi(&[
        ("LaunchCondition", vec![
            Column::build("Condition").string(255).primary_key().build(),
            Column::build("Description").string(255).nullable().build(),
        ], vec![
            vec![Value::from("VersionNT >= 601"), Value::from("Windows 7+ required")],
        ]),
    ]);
    test_msi("phase7_launch", &msi);

    // Phase 8: + ServiceInstall (empty)
    println!("\n=== Phase 8: + ServiceInstall (empty) ===");
    let msi = build_base_msi(&[
        ("ServiceInstall", vec![
            Column::build("ServiceInstall").string(72).primary_key().build(),
            Column::build("Name").string(255).nullable().build(),
            Column::build("DisplayName").string(255).nullable().build(),
            Column::build("ServiceType").int32().nullable().build(),
            Column::build("StartType").int32().nullable().build(),
            Column::build("ErrorControl").int32().nullable().build(),
            Column::build("LoadOrderGroup").string(255).nullable().build(),
            Column::build("Dependencies").string(255).nullable().build(),
            Column::build("StartName").string(255).nullable().build(),
            Column::build("Password").string(255).nullable().build(),
            Column::build("Arguments").string(255).nullable().build(),
            Column::build("Component_").string(72).nullable().build(),
            Column::build("Description").string(255).nullable().build(),
        ], vec![]),
    ]);
    test_msi("phase8_svcinst", &msi);

    // Phase 9: + ServiceControl (empty)
    println!("\n=== Phase 9: + ServiceControl (empty) ===");
    let msi = build_base_msi(&[
        ("ServiceControl", vec![
            Column::build("ServiceControl").string(72).primary_key().build(),
            Column::build("Name").string(255).nullable().build(),
            Column::build("Event").int32().nullable().build(),
            Column::build("Arguments").string(255).nullable().build(),
            Column::build("Wait").int16().nullable().build(),
            Column::build("Component_").string(72).nullable().build(),
        ], vec![]),
    ]);
    test_msi("phase9_svcctrl", &msi);

    // Phase 10: ALL extra tables combined
    println!("\n=== Phase 10: ALL extra tables combined ===");
    let msi = build_base_msi(&[
        ("Registry", vec![
            Column::build("Registry").string(72).primary_key().build(),
            Column::build("Root").int16().nullable().build(),
            Column::build("Key").string(255).nullable().build(),
            Column::build("Name").string(255).nullable().build(),
            Column::build("Value").string(255).nullable().build(),
            Column::build("Component_").string(72).nullable().build(),
        ], vec![
            vec![Value::from("Reg1"), Value::from(2i32), Value::from("Software\\TestApp"),
                 Value::from("Path"), Value::from("C:\\Test"), Value::from("MainComp")],
        ]),
        ("CustomAction", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Type").int16().nullable().build(),
            Column::build("Source").string(72).nullable().build(),
            Column::build("Target").string(255).nullable().build(),
        ], vec![
            vec![Value::from("ca1"), Value::from(51i32), Value::from("TARGETDIR"), Value::from("C:\\Test")],
        ]),
        ("Environment", vec![
            Column::build("Environment").string(72).primary_key().build(),
            Column::build("Name").string(255).nullable().build(),
            Column::build("Value").string(255).nullable().build(),
            Column::build("Component_").string(72).nullable().build(),
        ], vec![
            vec![Value::from("Env1"), Value::from("TEST_VAR"), Value::from("test_value"), Value::from("MainComp")],
        ]),
        ("Upgrade", vec![
            Column::build("UpgradeCode").string(38).primary_key().build(),
            Column::build("VersionMin").string(20).nullable().build(),
            Column::build("VersionMax").string(20).nullable().build(),
            Column::build("Language").string(20).nullable().build(),
            Column::build("Attributes").int32().nullable().build(),
        ], vec![
            vec![Value::from("{AAAAAAAA-1111-2222-3333-444455556666}"),
                 Value::from("1.0.0"), Value::from("1.0.0"), Value::Null, Value::from(259i32)],
        ]),
    ]);
    test_msi("phase10_all", &msi);

    let _ = std::fs::remove_dir_all("C:\\temp\\vel_msi_install");
    println!("\nDone!");
}
