/// Quick test: generate MSI with velocity-msi and test with msiexec
use std::io::Read;
use velocity_msi::{MsiBuilder, Column, Value};

fn main() {
    std::fs::create_dir_all("C:\\temp").ok();

    // Test 1: Minimal velocity-msi output
    println!("=== Test 1: velocity-msi minimal MSI ===");
    {
        let mut builder = MsiBuilder::new();
        builder.set_title("Test Product");
        builder.set_author("Test Corp");
        builder.set_template("Intel", 1033);

        builder.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ]).unwrap();
        builder.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("Test Product")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("Manufacturer"), Value::from("Test Corp")],
            vec![Value::from("ProductCode"), Value::from("{12345678-1234-1234-1234-123456789012}")],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
        ]).unwrap();

        let data = builder.build().unwrap();
        let path = "C:\\temp\\velocity_test.msi";
        std::fs::write(path, &data).unwrap();
        println!("Written: {} ({} bytes)", path, data.len());

        let output = std::process::Command::new("msiexec.exe")
            .args(&["/i", path, "/quiet", "/norestart"])
            .output().unwrap();
        let code = output.status.code().unwrap_or(-1);
        println!("msiexec exit code: {} (0=success, 1620=can't open, 1625=blocked by policy)", code);
    }

    // Test 2: Verify the UUID is in the SummaryInfo
    println!("\n=== Test 2: Check SummaryInfo has UUID ===");
    {
        let mut builder = MsiBuilder::new();
        builder.set_title("Test");
        builder.set_author("Test");

        builder.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ]).unwrap();
        builder.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("Test")],
        ]).unwrap();

        let data = builder.build().unwrap();

        // Read back and check SummaryInfo
        let mut comp = cfb::CompoundFile::open(std::io::Cursor::new(data)).unwrap();
        let summary_name = "\u{0005}SummaryInformation";
        let mut buf = Vec::new();
        comp.open_stream(summary_name).unwrap().read_to_end(&mut buf).unwrap();

        println!("SummaryInfo stream: {} bytes", buf.len());

        // Parse property set to find PID 9 (Revision Number)
        // Header: BOM(2) + version(2) + OS(4) + CLSID(16) + reserved(4) + FMTID(16) + offset(4) = 48
        // Section: size(4) + count(4) + entries(8 each)
        let num_props = u32::from_le_bytes([buf[52], buf[53], buf[54], buf[55]]);
        println!("Property count: {}", num_props);

        for i in 0..num_props {
            let base = 56 + (i as usize) * 8;
            let pid = u32::from_le_bytes([buf[base], buf[base+1], buf[base+2], buf[base+3]]);
            let offset = u32::from_le_bytes([buf[base+4], buf[base+5], buf[base+6], buf[base+7]]);
            let abs_offset = 48 + offset as usize;
            let vtype = u32::from_le_bytes([buf[abs_offset], buf[abs_offset+1], buf[abs_offset+2], buf[abs_offset+3]]);

            let pid_name = match pid {
                1 => "CodePage",
                2 => "Title",
                3 => "Subject",
                4 => "Author",
                5 => "Keywords",
                6 => "Comments",
                7 => "Template",
                9 => "RevisionNumber(UUID)",
                12 => "CreateTime",
                13 => "LastSaveTime",
                15 => "WordCount",
                18 => "CreatingApp",
                _ => "Unknown",
            };

            if vtype == 30 { // VT_LPSTR
                let str_len = u32::from_le_bytes([buf[abs_offset+4], buf[abs_offset+5], buf[abs_offset+6], buf[abs_offset+7]]) as usize;
                let str_start = abs_offset + 8;
                let str_end = str_start + str_len.saturating_sub(1);
                if str_end <= buf.len() {
                    let s = String::from_utf8_lossy(&buf[str_start..str_end]);
                    println!("  PID {}: {} = \"{}\"", pid, pid_name, s);
                }
            } else if vtype == 3 { // VT_I4
                let val = i32::from_le_bytes([buf[abs_offset+4], buf[abs_offset+5], buf[abs_offset+6], buf[abs_offset+7]]);
                println!("  PID {}: {} = {}", pid, pid_name, val);
            } else if vtype == 2 { // VT_I2
                let val = i16::from_le_bytes([buf[abs_offset+4], buf[abs_offset+5]]);
                println!("  PID {}: {} = {}", pid, pid_name, val);
            } else if vtype == 64 { // VT_FILETIME
                println!("  PID {}: {} = <FILETIME>", pid, pid_name);
            } else {
                println!("  PID {}: {} (type {})", pid, pid_name, vtype);
            }
        }
    }

    println!("\nDone!");
}
