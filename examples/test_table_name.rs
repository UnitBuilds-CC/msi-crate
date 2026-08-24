//! Test: Is the issue with 6-column tables or specifically with "File" name?

use std::process::Command;
use velocity_msi::{MsiBuilder, Column, Value};

fn test_msi(label: &str, builder: &mut MsiBuilder) -> i32 {
    let data = builder.build().unwrap();
    let path = format!("C:\\temp\\name_test\\{}.msi", label);
    std::fs::create_dir_all("C:\\temp\\name_test").ok();
    std::fs::write(&path, &data).unwrap();
    
    let log = format!("C:\\temp\\name_test\\{}.log", label);
    let status = Command::new("msiexec")
        .args(&["/i", &path, "/qn", "/norestart", "/l*v", &log])
        .status();
    
    let code = match status {
        Ok(s) => s.code().unwrap_or(-1),
        Err(_) => -1,
    };
    if code == 0 {
        let _ = Command::new("msiexec").args(&["/x", &path, "/qn", "/norestart"]).status();
    }
    code
}

fn main() {
    println!("=== Test: Table name vs table size ===\n");
    
    // Test 1: 6-column table named "MyTable" (NOT "File")
    {
        let mut b = MsiBuilder::new();
        b.set_title("Test");
        b.set_author("TestCo");
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("TestProduct")],
            vec![Value::from("ProductCode"), Value::from("{B1C2D3E4-F5A6-7890-ABCD-EF1234567890}")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
            vec![Value::from("Manufacturer"), Value::from("TestCo")],
            vec![Value::from("UpgradeCode"), Value::from("{C2D3E4F5-A6B7-8901-BCDE-F12345678901}")],
        ]).unwrap();
        
        // 6-column table named "MyTable"
        b.create_table("MyTable", vec![
            Column::build("Col1").string(72).primary_key().build(),
            Column::build("Col2").string(72).build(),
            Column::build("Col3").string(255).build(),
            Column::build("Col4").int32().build(),
            Column::build("Col5").int16().nullable().build(),
            Column::build("Col6").int32().build(),
        ]).unwrap();
        b.insert_rows("MyTable", vec![
            vec![Value::from("F1"), Value::from("MainComp"), Value::from("test.txt"), Value::from(13i32), Value::from(0i32), Value::from(1i32)],
        ]).unwrap();
        
        let code = test_msi("t1_mytable", &mut b);
        println!("Test 1 (6-col 'MyTable'): exit {} {}", code,
            if code == 0 { "✓" } else { "✗" });
    }
    
    // Test 2: Same schema as MyTable but named "File"
    {
        let mut b = MsiBuilder::new();
        b.set_title("Test");
        b.set_author("TestCo");
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("TestProduct")],
            vec![Value::from("ProductCode"), Value::from("{B1C2D3E4-F5A6-7890-ABCD-EF1234567890}")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
            vec![Value::from("Manufacturer"), Value::from("TestCo")],
            vec![Value::from("UpgradeCode"), Value::from("{C2D3E4F5-A6B7-8901-BCDE-F12345678901}")],
        ]).unwrap();
        
        // Same 6-column schema but named "File"
        b.create_table("File", vec![
            Column::build("Col1").string(72).primary_key().build(),
            Column::build("Col2").string(72).build(),
            Column::build("Col3").string(255).build(),
            Column::build("Col4").int32().build(),
            Column::build("Col5").int16().nullable().build(),
            Column::build("Col6").int32().build(),
        ]).unwrap();
        b.insert_rows("File", vec![
            vec![Value::from("F1"), Value::from("MainComp"), Value::from("test.txt"), Value::from(13i32), Value::from(0i32), Value::from(1i32)],
        ]).unwrap();
        
        let code = test_msi("t2_file_generic", &mut b);
        println!("Test 2 (6-col 'File', generic cols): exit {} {}", code,
            if code == 0 { "✓" } else { "✗" });
    }
    
    // Test 3: "File" table with correct column names but wrong count (3 cols)
    {
        let mut b = MsiBuilder::new();
        b.set_title("Test");
        b.set_author("TestCo");
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("TestProduct")],
            vec![Value::from("ProductCode"), Value::from("{B1C2D3E4-F5A6-7890-ABCD-EF1234567890}")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
            vec![Value::from("Manufacturer"), Value::from("TestCo")],
            vec![Value::from("UpgradeCode"), Value::from("{C2D3E4F5-A6B7-8901-BCDE-F12345678901}")],
        ]).unwrap();
        
        // "File" table with only 3 columns
        b.create_table("File", vec![
            Column::build("File_").string(72).primary_key().build(),
            Column::build("Component_").string(72).build(),
            Column::build("FileName").string(255).build(),
        ]).unwrap();
        b.insert_rows("File", vec![
            vec![Value::from("F1"), Value::from("MainComp"), Value::from("test.txt")],
        ]).unwrap();
        
        let code = test_msi("t3_file_3col", &mut b);
        println!("Test 3 ('File' with 3 cols): exit {} {}", code,
            if code == 0 { "✓" } else { "✗" });
    }
    
    // Test 4: "File" table with correct column names, 6 cols, but different types
    {
        let mut b = MsiBuilder::new();
        b.set_title("Test");
        b.set_author("TestCo");
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("TestProduct")],
            vec![Value::from("ProductCode"), Value::from("{B1C2D3E4-F5A6-7890-ABCD-EF1234567890}")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
            vec![Value::from("Manufacturer"), Value::from("TestCo")],
            vec![Value::from("UpgradeCode"), Value::from("{C2D3E4F5-A6B7-8901-BCDE-F12345678901}")],
        ]).unwrap();
        
        // "File" table with correct names but ALL string columns
        b.create_table("File", vec![
            Column::build("File_").string(72).primary_key().build(),
            Column::build("Component_").string(72).build(),
            Column::build("FileName").string(255).build(),
            Column::build("FileSize").string(32).build(),
            Column::build("Attributes").string(16).nullable().build(),
            Column::build("Sequence").string(32).build(),
        ]).unwrap();
        b.insert_rows("File", vec![
            vec![Value::from("F1"), Value::from("MC"), Value::from("test.txt"), Value::from("13"), Value::from("0"), Value::from("1")],
        ]).unwrap();
        
        let code = test_msi("t4_file_allstring", &mut b);
        println!("Test 4 ('File' all strings): exit {} {}", code,
            if code == 0 { "✓" } else { "✗" });
    }
    
    println!("\nDone.");
}
