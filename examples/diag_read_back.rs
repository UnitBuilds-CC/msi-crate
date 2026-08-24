/// Read back our MSI's system tables to verify correctness
/// cargo run --example diag_read_back -p velocity-msi
use velocity_msi::{Column, MsiBuilder, Value};
use std::io::Cursor;

fn make_uuid() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!(
        "{{{:08X}-{:04X}-4{:03X}-{:04X}-{:08X}{:04X}}}",
        (t & 0xFFFFFFFF) as u32,
        ((t >> 32) & 0xFFFF) as u16,
        ((t >> 48) & 0x0FFF) as u16,
        (((t >> 44) & 0x0FFF) as u16) | 0x8000,
        ((t >> 16) & 0xFFFFFFFF) as u32,
        (t & 0xFFFF) as u16,
    )
}

fn main() {
    let pc = make_uuid();
    let uc = make_uuid();
    let mut b = MsiBuilder::new();
    b.set_title("Test");
    b.set_author("V");
    b.set_template("Intel", 1033);
    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).localizable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("V")],
        vec![Value::from("ProductCode"), Value::from(pc.as_str())],
        vec![Value::from("UpgradeCode"), Value::from(uc.as_str())],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();
    b.create_table("File", vec![
        Column::build("File_").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).localizable().build(),
        Column::build("FileSize").int32().build(),
        Column::build("Attributes").nullable().int16().build(),
        Column::build("Sequence").int16().build(),
    ]).unwrap();
    b.insert_rows("File", vec![
        vec![Value::from("MainFile"), Value::from("MainComp"),
             Value::from("testfile.txt"), Value::Int(23), Value::Int(0), Value::Int(1)],
    ]).unwrap();
    let data = b.build().unwrap();

    // Read back using msi crate (it can read V3 MSIs)
    let cursor = Cursor::new(&data);
    let mut package = msi::Package::open(cursor).unwrap();

    println!("=== _Tables entries ===");
    let rows = package.select_rows(msi::Select::table("_Tables")).unwrap();
    for row in rows {
        println!("  Name={:?}", &row["Name"]);
    }

    println!("\n=== _Columns entries for File ===");
    let rows = package.select_rows(msi::Select::table("_Columns")).unwrap();
    for row in rows {
        let table_val = &row["Table"];
        if let msi::Value::Str(ref t) = *table_val {
            if t == "File" {
                let number = &row["Number"];
                let name = &row["Name"];
                let typ = &row["Type"];
                let type_val = match *typ { msi::Value::Int(v) => v as u32, _ => 0 };
                println!("  Number={:?} Column={:?} Type=0x{:04X}", number, name, type_val);
                let size = type_val & 0xFF;
                let valid = (type_val & 0x100) != 0;
                let localizable = (type_val & 0x200) != 0;
                let nonbinary = (type_val & 0x400) != 0;
                let string = (type_val & 0x800) != 0;
                let nullable = (type_val & 0x1000) != 0;
                let pk = (type_val & 0x2000) != 0;
                println!("    size={} valid={} loc={} nonbin={} str={} null={} pk={}",
                    size, valid, localizable, nonbinary, string, nullable, pk);
            }
        }
    }

    println!("\n=== _Validation entries for File ===");
    let rows = package.select_rows(msi::Select::table("_Validation")).unwrap();
    for row in rows {
        let table_val = &row["Table"];
        if let msi::Value::Str(ref t) = *table_val {
            if t == "File" {
                println!("  Col={:?} Nullable={:?} Min={:?} Max={:?} KeyTable={:?} KeyCol={:?} Category={:?} Set={:?}",
                    &row["Column"], &row["Nullable"], &row["MinValue"],
                    &row["MaxValue"], &row["KeyTable"], &row["KeyColumn"],
                    &row["Category"], &row["Set"]);
            }
        }
    }

    println!("\n=== File table data ===");
    let rows = package.select_rows(msi::Select::table("File")).unwrap();
    for row in rows {
        println!("  File_={:?} Component_={:?} FileName={:?} FileSize={:?} Attributes={:?} Sequence={:?}",
            &row["File_"], &row["Component_"], &row["FileName"],
            &row["FileSize"], &row["Attributes"], &row["Sequence"]);
    }

    println!("\n=== All streams ===");
    for name in package.streams() {
        println!("  {:?}", name);
    }

    println!("\n=== Database codepage ===");
    println!("  {:?}", package.database_codepage());
}
