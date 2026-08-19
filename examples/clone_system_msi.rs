/// DEFINITIVE TEST: Use msi crate to clone a system MSI.
/// If the clone also fails with 1620, the msi crate's writer is broken.
use std::io::{Cursor, Read};

fn main() {
    std::fs::create_dir_all("C:\\temp").ok();

    // Find a small system MSI
    let sys_path = std::fs::read_dir("C:\\Windows\\Installer").unwrap()
        .flatten()
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            if e.path().extension().map_or(false, |ext| ext == "msi") && meta.len() < 100_000 {
                Some(e.path())
            } else { None }
        })
        .next().unwrap();
    println!("System MSI: {} ({} bytes)", sys_path.display(),
        std::fs::metadata(&sys_path).unwrap().len());
    let sys_data = std::fs::read(&sys_path).unwrap();

    // ===== PART 1: Read system MSI and dump everything =====
    println!("\n=== PART 1: Read system MSI ===");
    let pkg = msi::Package::open(Cursor::new(sys_data.clone())).unwrap();

    println!("Database codepage: {:?}", pkg.database_codepage());
    println!("Package type: {:?}", pkg.package_type());

    // Dump SummaryInfo
    let si = pkg.summary_info();
    println!("\nSummaryInfo:");
    println!("  Title: {:?}", si.title());
    println!("  Subject: {:?}", si.subject());
    println!("  Author: {:?}", si.author());
    println!("  Comments: {:?}", si.comments());
    println!("  Arch: {:?}", si.arch());
    println!("  Languages: {:?}", si.languages().iter().map(|l| format!("{}", l.code())).collect::<Vec<_>>());
    println!("  Codepage: {:?}", si.codepage());
    println!("  Creating app: {:?}", si.creating_application());
    println!("  Creation time: {:?}", si.creation_time());
    println!("  UUID: {:?}", si.uuid());
    println!("  Word count: {:?}", si.word_count());

    // Dump tables
    println!("\nTables:");
    for table in pkg.tables() {
        println!("  {} ({} columns):", table.name(), table.columns().len());
        for col in table.columns() {
            println!("    {} - type: {:?}, pk: {}, nullable: {}, localizable: {}",
                col.name(), col.coltype(), col.is_primary_key(), col.is_nullable(), col.is_localizable());
        }
    }
    drop(pkg);

    // ===== PART 2: Clone the system MSI =====
    println!("\n=== PART 2: Clone system MSI ===");
    let mut src = msi::Package::open(Cursor::new(sys_data.clone())).unwrap();
    let db_codepage = src.database_codepage();

    let cursor = Cursor::new(Vec::new());
    let mut dst = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
    dst.set_database_codepage(db_codepage);

    // Copy SummaryInfo properties
    {
        let si = src.summary_info();
        let dst_si = dst.summary_info_mut();
        if let Some(v) = si.title() { dst_si.set_title(v.to_string()); }
        if let Some(v) = si.subject() { dst_si.set_subject(v.to_string()); }
        if let Some(v) = si.author() { dst_si.set_author(v.to_string()); }
        if let Some(v) = si.comments() { dst_si.set_comments(v.to_string()); }
        if let Some(v) = si.arch() { dst_si.set_arch(v.to_string()); }
        let langs = si.languages();
        if !langs.is_empty() { dst_si.set_languages(&langs); }
        dst_si.set_codepage(si.codepage());
        if let Some(v) = si.creating_application() { dst_si.set_creating_application(v.to_string()); }
        if let Some(v) = si.creation_time() { dst_si.set_creation_time(v); }
        if let Some(v) = si.uuid() { dst_si.set_uuid(v); }
        if let Some(v) = si.word_count() { dst_si.set_word_count(v); }
    }

    // Collect table info before mutating dst
    let system_tables = ["_Tables", "_Columns", "_Validation"];
    let mut table_data: Vec<(String, Vec<msi::Column>, Vec<Vec<msi::Value>>)> = Vec::new();

    // First, collect table schemas
    let table_schemas: Vec<(String, Vec<msi::Column>)> = src.tables()
        .filter(|t| !system_tables.contains(&t.name()))
        .map(|t| {
            let columns: Vec<msi::Column> = t.columns().iter().map(|c| {
                let mut builder = msi::Column::build(c.name());
                if c.is_primary_key() { builder = builder.primary_key(); }
                if c.is_nullable() { builder = builder.nullable(); }
                if c.is_localizable() { builder = builder.localizable(); }
                match c.coltype() {
                    msi::ColumnType::Int16 => builder.int16(),
                    msi::ColumnType::Int32 => builder.int32(),
                    msi::ColumnType::Str(len) => builder.string(len),
                }
            }).collect();
            (t.name().to_string(), columns)
        })
        .collect();

    // Then, read rows for each table
    for (table_name, columns) in table_schemas {
        let rows_iter = src.select_rows(msi::Select::table(&table_name)).unwrap();
        let values: Vec<Vec<msi::Value>> = rows_iter.map(|row| {
            row.columns().iter().enumerate().map(|(i, _c)| {
                match &row[i] {
                    msi::Value::Int(n) => msi::Value::Int(*n),
                    msi::Value::Str(s) => msi::Value::Str(s.clone()),
                    msi::Value::Null => msi::Value::Null,
                }
            }).collect()
        }).collect();

        println!("  Collected table '{}' ({} cols, {} rows)", table_name, columns.len(), values.len());
        table_data.push((table_name, columns, values));
    }
    drop(src);

    // Create tables and insert rows in the new package
    for (name, columns, rows) in &table_data {
        dst.create_table(name.clone(), columns.clone()).unwrap();
        if !rows.is_empty() {
            let mut insert = msi::Insert::into(name.clone());
            for row in rows {
                insert = insert.row(row.clone());
            }
            dst.insert_rows(insert).unwrap();
        }
    }

    // Write the cloned MSI
    let cursor = dst.into_inner().unwrap();
    let cloned_data = cursor.into_inner();
    let clone_path = "C:\\temp\\cloned_system.msi";
    std::fs::write(clone_path, &cloned_data).unwrap();
    println!("\nCloned MSI written: {} ({} bytes)", clone_path, cloned_data.len());

    // Test with msiexec
    println!("\n=== Test cloned MSI ===");
    let output = std::process::Command::new("msiexec.exe")
        .args(&["/i", clone_path, "/quiet", "/norestart"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("msiexec exit code: {} (0=success, 1620=can't open, 1625=blocked by policy)", exit_code);

    // ===== PART 3: Minimal MSI test =====
    println!("\n=== PART 3: Minimal MSI (just Property table) ===");
    {
        let cursor = Cursor::new(Vec::new());
        let mut pkg = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
        pkg.set_database_codepage(msi::CodePage::Windows1252);

        let columns = vec![
            msi::Column::build("Property").primary_key().id_string(72),
            msi::Column::build("Value").nullable().formatted_string(255),
        ];
        pkg.create_table("Property", columns).unwrap();
        pkg.insert_rows(msi::Insert::into("Property")
            .row(vec![msi::Value::from("ProductName"), msi::Value::from("Test Product")])
            .row(vec![msi::Value::from("ProductVersion"), msi::Value::from("1.0.0")])
            .row(vec![msi::Value::from("Manufacturer"), msi::Value::from("Test Corp")])
            .row(vec![msi::Value::from("ProductCode"), msi::Value::from("{12345678-1234-1234-1234-123456789012}")])
            .row(vec![msi::Value::from("ProductLanguage"), msi::Value::from("1033")])
        ).unwrap();

        pkg.summary_info_mut().set_arch("Intel");
        pkg.summary_info_mut().set_languages(&[msi::Language::from_tag("en-US")]);

        let cursor = pkg.into_inner().unwrap();
        let data = cursor.into_inner();
        let path = "C:\\temp\\minimal_msi.msi";
        std::fs::write(path, &data).unwrap();
        println!("Written: {} ({} bytes)", path, data.len());

        let output = std::process::Command::new("msiexec.exe")
            .args(&["/i", path, "/quiet", "/norestart"])
            .output().unwrap();
        println!("msiexec exit code: {}", output.status.code().unwrap_or(-1));
    }

    // ===== PART 4: Compare stream sizes =====
    println!("\n=== PART 4: Compare stream sizes ===");
    fn dump_streams(data: &[u8], label: &str) {
        let mut comp = cfb::CompoundFile::open(Cursor::new(data.to_vec())).unwrap();
        let mut paths = Vec::new();
        for entry in comp.walk() {
            if entry.is_stream() {
                paths.push(entry.path().to_str().unwrap().to_string());
            }
        }
        println!("{} streams:", label);
        for path in &paths {
            let mut buf = Vec::new();
            comp.open_stream(path).unwrap().read_to_end(&mut buf).unwrap();
            // Decode stream name if it's a table
            let (decoded, is_table) = decode_stream_name(path);
            if is_table {
                println!("  '{}' = {} ({} bytes)", path, decoded, buf.len());
            } else {
                println!("  '{}' ({} bytes)", path, buf.len());
            }
        }
    }
    dump_streams(&sys_data, "System MSI");
    dump_streams(&cloned_data, "Clone");

    println!("\nDone!");
}

fn decode_stream_name(name: &str) -> (String, bool) {
    let mut output = String::new();
    let mut is_table = false;
    let prefix = '\u{4840}';
    let mut chars = name.chars().peekable();
    if chars.peek() == Some(&prefix) {
        is_table = true;
        chars.next();
    }
    for chr in chars {
        let value = chr as u32;
        if (0x3800..0x4800).contains(&value) {
            let value = value - 0x3800;
            output.push(from_b64(value & 0x3f));
            output.push(from_b64(value >> 6));
        } else if (0x4800..0x4840).contains(&value) {
            output.push(from_b64(value - 0x4800));
        } else {
            output.push(chr);
        }
    }
    (output, is_table)
}

fn from_b64(value: u32) -> char {
    if value < 10 { char::from_u32(value + '0' as u32).unwrap() }
    else if value < 36 { char::from_u32(value - 10 + 'A' as u32).unwrap() }
    else if value < 62 { char::from_u32(value - 36 + 'a' as u32).unwrap() }
    else if value == 62 { '.' }
    else { '_' }
}
