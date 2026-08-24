/// Count actual string pool IDs from _StringData content.
fn main() {
    // From the dump output, _StringData content:
    let data = "1.0.01033ActionCategoryColumnConditionCostFinalizeCostInitializeDefaultDirDescriptionDirectoryDirectory_ParentExecSeq TestINSTALLDIRIdentifierInstallExecuteSequenceKeyColumnKeyTableManufacturerMaxValueMinValueNNullableProductCodeProductLanguageProductNameProductVersionPropertySequenceSetSourceDirTARGETDIRTableTextText;UpperCase;LowerCase;Integer;DoubleInteger;TimeDate;Identifier;Property;Filename;WildCardFilename;Path;Paths;AnyPath;DefaultDir;RegPath;Formatted;FormattedSDDLText;Template;Condition;GUID;Version;Language;Binary;CustomSource;Cabinet;ShortcutUpgradeCodeValueVelTestVelocityYY;N_Validation{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}";

    // Pool entry lengths from dump:
    let lengths = vec![
        5, 4, 6, 8, 6, 9, 12, 14, 10, 11,  // 1-10
        9, 16, 12, 10, 10, 22, 9, 8, 12, 8,  // 11-20
        8, 1, 4, 8, 6, 11, 15, 11, 14, 8,    // 21-30
        8, 3, 9, 9, 5, 4, 245, 4, 11, 5,     // 31-40
        7, 8, 1, 3, 8, 7, 11, 12, 38, 38,    // 41-49
    ];

    // Parse strings from data using lengths
    let mut offset = 0;
    let mut pool_id = 0u32;
    let mut pool_map: Vec<(u32, String)> = Vec::new();

    for &len in &lengths {
        pool_id += 1;
        if offset + len <= data.len() {
            let s = &data[offset..offset + len];
            pool_map.push((pool_id, s.to_string()));
            offset += len;
        } else {
            pool_map.push((pool_id, format!("<truncated at {}/{}>", offset, data.len())));
            break;
        }
    }

    println!("=== Actual String Pool ===");
    for (id, s) in &pool_map {
        println!("  Pool {}: {:?}", id, s);
    }

    // Check Directory table IDs
    println!("\n=== Directory table values ===");
    let find = |name: &str| -> Option<u32> {
        pool_map.iter().find(|(_, s)| s == name).map(|(id, _)| *id)
    };
    for name in &["INSTALLDIR", "TARGETDIR", "SourceDir", "VelTest"] {
        match find(name) {
            Some(id) => println!("  {:?} = pool ID {} (0x{:04x})", name, id, id),
            None => println!("  {:?} = NOT FOUND!", name),
        }
    }

    // Expected binary
    let installdir = find("INSTALLDIR").unwrap();
    let targetdir = find("TARGETDIR").unwrap();
    let sourcedir = find("SourceDir").unwrap();
    let veltest = find("VelTest").unwrap();
    println!("\n=== Expected Directory binary ===");
    println!("  Row 0 (INSTALLDIR): {:04x} {:04x} {:04x}", installdir, targetdir, veltest);
    println!("  Row 1 (TARGETDIR):  {:04x} {:04x} {:04x}", targetdir, 0u16, sourcedir);

    // Compare with actual
    println!("\n=== Actual Directory binary from dump ===");
    println!("  0e 00 22 00 22 00 00 00 29 00 21 00");
    println!("  Row 0: col0=0x{:04x} col1=0x{:04x} col2=0x{:04x}", 0x000e, 0x0022, 0x0022);
    println!("  Row 1: col0=0x{:04x} col1=0x{:04x} col2=0x{:04x}", 0x0022, 0x0000, 0x0029);
    println!("  Wait, that's col-major: col0=[0e,22], col1=[22,00], col2=[29,21]");
    println!("  Row 0 (first values): col0=0x000e col1=0x0022 col2=0x0029");
    println!("  Row 1 (second values): col0=0x0022 col1=0x0000 col2=0x0021");

    // What strings are at pool IDs 0x22=34, 0x29=41, 0x21=33?
    println!("\n=== What's at the actual pool IDs? ===");
    for id in &[0x0eu32, 0x22, 0x29, 0x21] {
        if let Some((_, s)) = pool_map.iter().find(|(pid, _)| pid == id) {
            println!("  Pool {} (0x{:04x}): {:?}", id, id, s);
        } else {
            println!("  Pool {} (0x{:04x}): NOT IN POOL!", id, id);
        }
    }
}
