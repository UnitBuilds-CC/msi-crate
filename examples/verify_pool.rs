/// Verify string pool IDs for Directory table.
fn main() {
    // Rebuild the string pool to verify IDs
    let mut pool = velocity_msi::StringPool::new(false);

    // Simulate what MsiBuilder::build() does:
    // 1. create_system_tables() interns system strings
    // 2. User tables add their strings
    // 3. build() adds _Tables/_Columns/_Validation strings

    // System table strings (from create_system_tables)
    for s in &["_Validation", "Table", "Column", "Nullable", "MinValue", "MaxValue",
               "KeyTable", "KeyColumn", "Category", "Set", "Description", "N", "Y",
               "Identifier", "Text", "Y;N"] {
        pool.intern(s);
    }
    // all_categories string
    pool.intern("Text;UpperCase;LowerCase;Integer;DoubleInteger;TimeDate;Identifier;Property;Filename;WildCardFilename;Path;Paths;AnyPath;DefaultDir;RegPath;Formatted;FormattedSDDLText;Template;Condition;GUID;Version;Language;Binary;CustomSource;Cabinet;Shortcut");

    // _Validation self-entries (Table/Column/Nullable/MinValue/MaxValue/KeyTable/KeyColumn/Category/Set/Description)
    // These are already interned above

    // User table strings
    // Property table
    for s in &["Property", "Value", "ProductName", "ExecSeq Test", "ProductVersion",
               "1.0.0", "Manufacturer", "Velocity", "ProductCode",
               "{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}", "UpgradeCode",
               "{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}", "ProductLanguage", "1033"] {
        pool.intern(s);
    }

    // Directory table
    for s in &["Directory", "Directory_Parent", "DefaultDir", "TARGETDIR", "SourceDir",
               "INSTALLDIR", "VelTest"] {
        pool.intern(s);
    }

    // InstallExecuteSequence table
    for s in &["InstallExecuteSequence", "Action", "Condition", "Sequence",
               "CostInitialize", "CostFinalize"] {
        pool.intern(s);
    }

    // Title/Author/Template
    pool.intern("ExecSeq Test");
    pool.intern("Velocity");
    pool.intern("Intel;1033");

    // Reindex
    pool.reindex();

    // Print all strings with their IDs
    println!("=== String Pool (after reindex) ===");
    let mut entries: Vec<(u32, &str)> = Vec::new();
    for (s, id, _) in pool.iter() {
        entries.push((id, s));
    }
    entries.sort_by_key(|&(id, _)| id);
    for (id, s) in &entries {
        println!("  Pool {}: {:?}", id, s);
    }

    // Check specific IDs
    println!("\n=== Directory table IDs ===");
    for s in &["INSTALLDIR", "TARGETDIR", "SourceDir", "VelTest"] {
        if let Some(id) = pool.get_id(s) {
            println!("  {:?} = pool ID {}", s, id);
        } else {
            println!("  {:?} = NOT FOUND!", s);
        }
    }

    // Expected Directory binary
    println!("\n=== Expected Directory binary ===");
    let installdir = pool.get_id("INSTALLDIR").unwrap();
    let targetdir = pool.get_id("TARGETDIR").unwrap();
    let sourcedir = pool.get_id("SourceDir").unwrap();
    let veltest = pool.get_id("VelTest").unwrap();
    println!("  Row 0 (INSTALLDIR): col0={:04x} col1={:04x} col2={:04x}",
             installdir, targetdir, veltest);
    println!("  Row 1 (TARGETDIR):  col0={:04x} col1={:04x} col2={:04x}",
             targetdir, 0u16, sourcedir);
    println!("  Binary: {:02x?}", &[
        (installdir & 0xff) as u8, (installdir >> 8) as u8,
        (targetdir & 0xff) as u8, (targetdir >> 8) as u8,
        (targetdir & 0xff) as u8, (targetdir >> 8) as u8,
        0x00, 0x00,
        (veltest & 0xff) as u8, (veltest >> 8) as u8,
        (sourcedir & 0xff) as u8, (sourcedir >> 8) as u8,
    ]);
}
