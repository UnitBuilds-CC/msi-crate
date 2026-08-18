/// Create a minimal valid MSI using the Windows Installer API
/// Then compare its structure with our generated MSI
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::io::Read;

#[link(name = "msi")]
extern "system" {
    fn MsiOpenDatabaseW(db_path: *const u16, persist: *const u16, db_handle: *mut isize) -> i32;
    fn MsiCloseHandle(handle: isize) -> i32;
    fn MsiDatabaseOpenViewW(db_handle: isize, query: *const u16, view_handle: *mut isize) -> i32;
    fn MsiViewExecute(view_handle: isize, record: isize) -> i32;
    fn MsiViewClose(view_handle: isize) -> i32;
    fn MsiCreateRecord(field_count: u32) -> isize;
    fn MsiRecordSetStringW(record: isize, field: u32, value: *const u16) -> i32;
    fn MsiRecordSetInteger(record: isize, field: u32, value: i32) -> i32;
    fn MsiDatabaseCommit(db_handle: isize) -> i32;
}

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn exec_sql(db: isize, sql: &str) {
    let sql_w = to_wide(sql);
    let mut view: isize = 0;
    unsafe {
        let r = MsiDatabaseOpenViewW(db, sql_w.as_ptr(), &mut view);
        if r != 0 { println!("  OpenView failed for '{}': {}", sql, r); return; }
        let r = MsiViewExecute(view, 0);
        if r != 0 { println!("  Execute failed for '{}': {}", sql, r); }
        MsiViewClose(view);
        MsiCloseHandle(view);
    }
}

fn exec_sql_with_record(db: isize, sql: &str, record: isize) {
    let sql_w = to_wide(sql);
    let mut view: isize = 0;
    unsafe {
        let r = MsiDatabaseOpenViewW(db, sql_w.as_ptr(), &mut view);
        if r != 0 { println!("  OpenView failed for '{}': {}", sql, r); return; }
        let r = MsiViewExecute(view, record);
        if r != 0 { println!("  Execute failed for '{}': {}", sql, r); }
        MsiViewClose(view);
        MsiCloseHandle(view);
    }
}

fn main() {
    let path = "c:\\Users\\visse\\OneDrive\\Documentos\\V.E.L.O.C.I.T.Y.-Installer-master\\target\\api_created.msi";
    let path_w = to_wide(path);
    let mode_w = to_wide("read-write");
    
    // Delete existing file
    let _ = std::fs::remove_file(path);
    
    // Create new database
    let mut db: isize = 0;
    unsafe {
        let r = MsiOpenDatabaseW(path_w.as_ptr(), mode_w.as_ptr(), &mut db);
        if r != 0 {
            println!("Failed to create database: error {}", r);
            return;
        }
    }
    println!("Created database at {}", path);
    
    // Create Property table
    exec_sql(db, "CREATE TABLE `Property` (`Property` CHAR(72) NOT NULL, `Value` CHAR(255) NOT NULL LOCALIZABLE PRIMARY KEY `Property`)");
    
    // Insert properties
    let props = vec![
        ("ProductName", "API Test Product"),
        ("ProductVersion", "1.0.0"),
        ("Manufacturer", "API Test Corp"),
        ("ProductCode", "{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}"),
        ("UpgradeCode", "{11111111-2222-3333-4444-555555555555}"),
    ];
    
    for (name, value) in &props {
        unsafe {
            let record = MsiCreateRecord(2);
            let name_w = to_wide(name);
            let value_w = to_wide(value);
            MsiRecordSetStringW(record, 1, name_w.as_ptr());
            MsiRecordSetStringW(record, 2, value_w.as_ptr());
            exec_sql_with_record(db, "INSERT INTO `Property` (`Property`, `Value`) VALUES (?, ?)", record);
            MsiCloseHandle(record);
        }
    }
    
    // Commit
    unsafe {
        let r = MsiDatabaseCommit(db);
        if r != 0 { println!("Commit failed: error {}", r); }
        else { println!("Database committed!"); }
        MsiCloseHandle(db);
    }
    
    // Check file size
    let metadata = std::fs::metadata(path).unwrap();
    println!("File size: {} bytes", metadata.len());
    
    // Now analyze the API-created MSI
    println!("\n=== API-CREATED MSI STREAMS ===");
    let file = std::fs::File::open(path).unwrap();
    let comp = cfb::CompoundFile::open(file).unwrap();
    let entries: Vec<(std::path::PathBuf, bool)> = comp.walk()
        .map(|e| (e.path().to_path_buf(), e.is_stream()))
        .collect();
    
    for (p, is_stream) in &entries {
        if !*is_stream { continue; }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let mut comp2 = cfb::CompoundFile::open(std::fs::File::open(path).unwrap()).unwrap();
        let mut stream = comp2.open_stream(&p).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        let cps: Vec<u16> = name.encode_utf16().collect();
        let prefix = if !cps.is_empty() && cps[0] == 0x4840 { "T" } else { "N" };
        println!("  [{}] '{}' ({} cps) {} bytes", prefix, name, cps.len(), data.len());
        
        // If this is the string pool, show header
        if name.len() > 0 && data.len() >= 4 {
            let cps_check: Vec<u16> = "\u{4840}\u{3F3F}\u{4577}\u{446C}\u{3E6A}\u{44B2}\u{482F}"
                .encode_utf16().collect();
            let name_cps: Vec<u16> = name.encode_utf16().collect();
            if name_cps == cps_check {
                let cp_raw = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                println!("    StringPool codepage: 0x{:08X} (cp={}, long={})", 
                    cp_raw, cp_raw & 0x7FFFFFFF, (cp_raw & 0x80000000) != 0);
            }
        }
    }
    
    // Test with msiexec
    println!("\n=== Testing with msiexec ===");
    let log_path = "c:\\Users\\visse\\OneDrive\\Documentos\\V.E.L.O.C.I.T.Y.-Installer-master\\target\\api_log.txt";
    let _ = std::fs::remove_file(log_path);
    
    let status = std::process::Command::new("msiexec.exe")
        .args(&["/i", path, "/qn", "/l*v", log_path])
        .status()
        .unwrap();
    println!("msiexec exit code: {:?}", status.code());
    
    // Show first lines of log
    if let Ok(log) = std::fs::read_to_string(log_path) {
        for line in log.lines().take(20) {
            println!("{}", line);
        }
    }
}
