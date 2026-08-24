//! Progressive test: add compiler's exact tables one-by-one to find which breaks msiexec
use velocity_msi::{MsiBuilder, Column, Value, CabinetFile, build_cabinet};
use std::process::Command;

fn build_base_msi() -> MsiBuilder {
    let mut b = MsiBuilder::new();
    b.set_title("Sample App");
    b.set_author("Velocity Team");
    b.set_template("x64", 1033);

    // Property table - matching compiler exactly
    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    let props = vec![
        ("ProductCode", "{D5E0EEC4-CF5C-5D68-BAC7-AA26667F6C80}"),
        ("UpgradeCode", "{7B44FAB1-58DD-5368-9B0C-338B5E7519DD}"),
        ("ProductName", "Sample App"),
        ("Manufacturer", "Velocity Team"),
        ("ProductVersion", "1.0.0"),
        ("ProductLanguage", "1033"),
        ("Description", "A sample application"),
    ];
    for (name, value) in props {
        b.insert_rows("Property", vec![
            vec![Value::from(name), Value::from(value)],
        ]).unwrap();
    }

    // Directory table - use writable target to avoid admin privilege issues
    b.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("SampleApp:SAMPLEAPP")],
        vec![Value::from("ProgramMenuFolder"), Value::from("TARGETDIR"), Value::from("Programs")],
        vec![Value::from("ApplicationProgramsFolder"), Value::from("ProgramMenuFolder"), Value::from("SampleApp:SAMPLEAPP")],
        vec![Value::from("DesktopFolder"), Value::from("TARGETDIR"), Value::from("Desktop")],
    ]).unwrap();

    // Component table (1 file component)
    b.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(38).nullable().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
    b.insert_rows("Component", vec![
        vec![Value::from("comp_0"), Value::from("{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}"), Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::from("file_0")],
    ]).unwrap();

    // File table (1 file)
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
    b.insert_rows("File", vec![
        vec![Value::from("file_0"), Value::from("comp_0"), Value::from("sample-app.exe"), Value::Int(1024), Value::Null, Value::Int(0), Value::Int(0), Value::Int(1)],
    ]).unwrap();

    // Feature table
    b.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").string(38).nullable().build(),
        Column::build("Title").string(64).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().nullable().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
    ]).unwrap();
    b.insert_rows("Feature", vec![
        vec![Value::from("Complete"), Value::Null, Value::from("Sample App Setup"), Value::from("Complete installation"), Value::Int(1), Value::Int(1), Value::from("INSTALLDIR"), Value::Int(0)],
    ]).unwrap();

    // FeatureComponents
    b.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    b.insert_rows("FeatureComponents", vec![
        vec![Value::from("Complete"), Value::from("comp_0")],
    ]).unwrap();

    // Media table
    b.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int32().build(),
        Column::build("DiskPrompt").string(255).nullable().build(),
        Column::build("VolumeLabel").string(32).nullable().build(),
        Column::build("Cabinet").string(255).nullable().build(),
        Column::build("Source").string(72).nullable().build(),
    ]).unwrap();
    b.insert_rows("Media", vec![
        vec![Value::Int(1), Value::Int(1), Value::Null, Value::Null, Value::from("#Velocity.cab"), Value::Null],
    ]).unwrap();

    // Embed cabinet
    let cabinet = build_cabinet(&[CabinetFile {
        name: "file_0".to_string(),
        data: vec![0u8; 1024],
    }]);
    b.add_stream("Velocity.cab".to_string(), cabinet);

    b
}

fn test_msi(b: &mut MsiBuilder, label: &str, phase: &mut i32) -> bool {
    let filename = format!("progressive_phase{}.msi", phase);
    match b.build() {
        Ok(msi) => {
            std::fs::write(&filename, &msi).unwrap();
            let log = format!("progressive_phase{}.log", phase);
            let status = Command::new("msiexec.exe")
                .args(&["/i", &filename, "/qn", "/l*v", &log])
                .status()
                .unwrap();
            let code = status.code().unwrap_or(-1);
            let ok = code == 0;
            println!("Phase {}: {} -> exit code {} {}", phase, label, code, if ok { "OK" } else { "FAIL" });
            if !ok {
                // Show the error from log
                if let Ok(log_content) = std::fs::read_to_string(&log) {
                    for line in log_content.lines() {
                        if line.contains("installed the product") || line.contains("1620") || line.contains("error") {
                            println!("  LOG: {}", line.trim());
                        }
                    }
                }
            }
            // Uninstall if successful
            if ok {
                let _ = Command::new("msiexec.exe")
                    .args(&["/x", "{D5E0EEC4-CF5C-5D68-BAC7-AA26667F6C80}", "/qn"])
                    .status();
            }
            *phase += 1;
            ok
        }
        Err(e) => {
            println!("Phase {}: {} -> BUILD ERROR: {}", phase, label, e);
            *phase += 1;
            false
        }
    }
}

fn main() {
    println!("=== Progressive compiler table test ===\n");
    let mut phase = 0;

    // Phase 0: Base MSI (should work)
    {
        let mut b = build_base_msi();
        if !test_msi(&mut b, "Base (Property/Dir/Comp/File/Feature/Media)", &mut phase) {
            println!("BASE FAILED - cannot proceed");
            return;
        }
    }

    // Phase 1: Add Registry table (3 rows)
    {
        let mut b = build_base_msi();
        b.create_table("Registry", vec![
            Column::build("Registry").string(72).primary_key().build(),
            Column::build("Root").int16().nullable().build(),
            Column::build("Key").string(255).nullable().build(),
            Column::build("Name").string(255).nullable().build(),
            Column::build("Value").string(255).nullable().build(),
            Column::build("Component_").string(72).nullable().build(),
        ]).unwrap();
        // Add 3 registry entries with components
        for i in 0..3 {
            let reg_id = format!("reg_{}", i);
            let comp_id = format!("comp_reg_{}", i);
            b.insert_rows("Component", vec![
                vec![Value::from(comp_id.as_str()), Value::from("{11111111-2222-3333-4444-555555555555}"), Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::Null],
            ]).unwrap();
            b.insert_rows("FeatureComponents", vec![
                vec![Value::from("Complete"), Value::from(comp_id.as_str())],
            ]).unwrap();
            b.insert_rows("Registry", vec![
                vec![Value::from(reg_id.as_str()), Value::Int(2), Value::from("Software\\SampleApp"), Value::from("TestValue"), Value::from("TestData"), Value::from(comp_id.as_str())],
            ]).unwrap();
        }
        test_msi(&mut b, "+ Registry (3 rows)", &mut phase);
    }

    // Phase 2: Add Shortcut table (2 rows)
    {
        let mut b = build_base_msi();
        b.create_table("Shortcut", vec![
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
        ]).unwrap();
        // Desktop shortcut component
        b.insert_rows("Component", vec![
            vec![Value::from("comp_shortcut_desktop"), Value::from("{AAAAAAAA-1111-2222-3333-444444444444}"), Value::from("DesktopFolder"), Value::Int(0), Value::Null, Value::Null],
        ]).unwrap();
        b.insert_rows("FeatureComponents", vec![
            vec![Value::from("Complete"), Value::from("comp_shortcut_desktop")],
        ]).unwrap();
        b.insert_rows("Shortcut", vec![
            vec![Value::from("DesktopShortcut"), Value::from("DesktopFolder"), Value::from("Sample App"), Value::from("comp_shortcut_desktop"), Value::from("[INSTALLDIR]sample-app.exe"), Value::Null, Value::from("A sample application"), Value::Int(0), Value::Null, Value::Int(0), Value::Int(1), Value::from("INSTALLDIR")],
        ]).unwrap();
        test_msi(&mut b, "+ Shortcut (1 row)", &mut phase);
    }

    // Phase 3: Add Environment table (2 rows)
    {
        let mut b = build_base_msi();
        b.create_table("Environment", vec![
            Column::build("Environment").string(72).primary_key().build(),
            Column::build("Name").string(255).nullable().build(),
            Column::build("Value").string(255).nullable().build(),
            Column::build("Component_").string(72).nullable().build(),
        ]).unwrap();
        for i in 0..2 {
            let env_id = format!("env_{}", i);
            let comp_id = format!("comp_env_{}", i);
            b.insert_rows("Component", vec![
                vec![Value::from(comp_id.as_str()), Value::from("{BBBBBBBB-1111-2222-3333-444444444444}"), Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::Null],
            ]).unwrap();
            b.insert_rows("FeatureComponents", vec![
                vec![Value::from("Complete"), Value::from(comp_id.as_str())],
            ]).unwrap();
            b.insert_rows("Environment", vec![
                vec![Value::from(env_id.as_str()), Value::from("SAMPLE_HOME"), Value::from("[INSTALLDIR]"), Value::from(comp_id.as_str())],
            ]).unwrap();
        }
        test_msi(&mut b, "+ Environment (2 rows)", &mut phase);
    }

    // Phase 4: Add InstallExecuteSequence + InstallUISequence
    {
        let mut b = build_base_msi();
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
        let exec_actions: Vec<(&str, Option<&str>, i32)> = vec![
            ("AppSearch", None, 100),
            ("LaunchConditions", Some("NOT Installed"), 105),
            ("CostInitialize", None, 120),
            ("CostFinalize", None, 140),
            ("InstallValidate", None, 150),
            ("InstallInitialize", None, 160),
            ("ProcessComponents", None, 170),
            ("InstallFiles", None, 200),
            ("InstallFinalize", None, 400),
        ];
        for (action, cond, seq) in exec_actions {
            let c = match cond { Some(v) => Value::from(v), None => Value::Null };
            b.insert_rows("InstallExecuteSequence", vec![
                vec![Value::from(action), c, Value::Int(seq)],
            ]).unwrap();
        }
        let ui_actions: Vec<(&str, Option<&str>, i32)> = vec![
            ("ShowLog", None, -1),
            ("FatalError", None, 9999),
            ("UserExit", None, 9999),
            ("WelcomeDlg", Some("NOT Installed"), 1230),
            ("VerifyReadyDlg", Some("NOT Installed"), 1250),
            ("ExecuteAction", None, 1300),
        ];
        for (action, cond, seq) in ui_actions {
            let c = match cond { Some(v) => Value::from(v), None => Value::Null };
            b.insert_rows("InstallUISequence", vec![
                vec![Value::from(action), c, Value::Int(seq)],
            ]).unwrap();
        }
        test_msi(&mut b, "+ InstallExecuteSequence + InstallUISequence", &mut phase);
    }

    // Phase 5: Add CustomAction table (empty)
    {
        let mut b = build_base_msi();
        b.create_table("CustomAction", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Type").int16().nullable().build(),
            Column::build("Source").string(72).nullable().build(),
            Column::build("Target").string(255).nullable().build(),
        ]).unwrap();
        // No rows - empty table should be skipped
        test_msi(&mut b, "+ CustomAction (empty, should skip)", &mut phase);
    }

    // Phase 6: Add ALL extra tables at once (matching compiler)
    {
        let mut b = build_base_msi();
        // Registry
        b.create_table("Registry", vec![
            Column::build("Registry").string(72).primary_key().build(),
            Column::build("Root").int16().nullable().build(),
            Column::build("Key").string(255).nullable().build(),
            Column::build("Name").string(255).nullable().build(),
            Column::build("Value").string(255).nullable().build(),
            Column::build("Component_").string(72).nullable().build(),
        ]).unwrap();
        for i in 0..3 {
            let reg_id = format!("reg_{}", i);
            let comp_id = format!("comp_reg_{}", i);
            b.insert_rows("Component", vec![
                vec![Value::from(comp_id.as_str()), Value::from("{11111111-2222-3333-4444-555555555555}"), Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::Null],
            ]).unwrap();
            b.insert_rows("FeatureComponents", vec![
                vec![Value::from("Complete"), Value::from(comp_id.as_str())],
            ]).unwrap();
            b.insert_rows("Registry", vec![
                vec![Value::from(reg_id.as_str()), Value::Int(2), Value::from("Software\\SampleApp"), Value::from("TestValue"), Value::from("TestData"), Value::from(comp_id.as_str())],
            ]).unwrap();
        }
        // Shortcut
        b.create_table("Shortcut", vec![
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
        ]).unwrap();
        b.insert_rows("Component", vec![
            vec![Value::from("comp_shortcut_desktop"), Value::from("{AAAAAAAA-1111-2222-3333-444444444444}"), Value::from("DesktopFolder"), Value::Int(0), Value::Null, Value::Null],
        ]).unwrap();
        b.insert_rows("FeatureComponents", vec![
            vec![Value::from("Complete"), Value::from("comp_shortcut_desktop")],
        ]).unwrap();
        b.insert_rows("Shortcut", vec![
            vec![Value::from("DesktopShortcut"), Value::from("DesktopFolder"), Value::from("Sample App"), Value::from("comp_shortcut_desktop"), Value::from("[INSTALLDIR]sample-app.exe"), Value::Null, Value::from("A sample application"), Value::Int(0), Value::Null, Value::Int(0), Value::Int(1), Value::from("INSTALLDIR")],
        ]).unwrap();
        // Environment
        b.create_table("Environment", vec![
            Column::build("Environment").string(72).primary_key().build(),
            Column::build("Name").string(255).nullable().build(),
            Column::build("Value").string(255).nullable().build(),
            Column::build("Component_").string(72).nullable().build(),
        ]).unwrap();
        for i in 0..2 {
            let env_id = format!("env_{}", i);
            let comp_id = format!("comp_env_{}", i);
            b.insert_rows("Component", vec![
                vec![Value::from(comp_id.as_str()), Value::from("{BBBBBBBB-1111-2222-3333-444444444444}"), Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::Null],
            ]).unwrap();
            b.insert_rows("FeatureComponents", vec![
                vec![Value::from("Complete"), Value::from(comp_id.as_str())],
            ]).unwrap();
            b.insert_rows("Environment", vec![
                vec![Value::from(env_id.as_str()), Value::from("SAMPLE_HOME"), Value::from("[INSTALLDIR]"), Value::from(comp_id.as_str())],
            ]).unwrap();
        }
        // InstallExecuteSequence
        b.create_table("InstallExecuteSequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        let exec_actions: Vec<(&str, Option<&str>, i32)> = vec![
            ("AppSearch", None, 100), ("CostInitialize", None, 120),
            ("CostFinalize", None, 140), ("InstallValidate", None, 150),
            ("InstallInitialize", None, 160), ("ProcessComponents", None, 170),
            ("InstallFiles", None, 200), ("InstallFinalize", None, 400),
        ];
        for (action, cond, seq) in exec_actions {
            let c = match cond { Some(v) => Value::from(v), None => Value::Null };
            b.insert_rows("InstallExecuteSequence", vec![
                vec![Value::from(action), c, Value::Int(seq)],
            ]).unwrap();
        }
        // InstallUISequence
        b.create_table("InstallUISequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        let ui_actions: Vec<(&str, Option<&str>, i32)> = vec![
            ("FatalError", None, 9999), ("UserExit", None, 9999),
            ("ExecuteAction", None, 1300),
        ];
        for (action, cond, seq) in ui_actions {
            let c = match cond { Some(v) => Value::from(v), None => Value::Null };
            b.insert_rows("InstallUISequence", vec![
                vec![Value::from(action), c, Value::Int(seq)],
            ]).unwrap();
        }
        // CustomAction (empty)
        b.create_table("CustomAction", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Type").int16().nullable().build(),
            Column::build("Source").string(72).nullable().build(),
            Column::build("Target").string(255).nullable().build(),
        ]).unwrap();
        // Icon (empty)
        b.create_table("Icon", vec![
            Column::build("Name").string(72).primary_key().build(),
            Column::build("Data").binary().nullable().build(),
        ]).unwrap();
        // ServiceInstall (empty)
        b.create_table("ServiceInstall", vec![
            Column::build("ServiceInstall").string(72).primary_key().build(),
            Column::build("Name").string(255).nullable().build(),
        ]).unwrap();
        // ServiceControl (empty)
        b.create_table("ServiceControl", vec![
            Column::build("ServiceControl").string(72).primary_key().build(),
            Column::build("Name").string(255).nullable().build(),
        ]).unwrap();

        test_msi(&mut b, "+ ALL extra tables combined", &mut phase);
    }

    println!("\nDone!");
}
