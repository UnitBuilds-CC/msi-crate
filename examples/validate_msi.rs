/// Validate MSI database using Windows Installer API
/// Links against msi.dll to call MsiOpenDatabaseW, etc.
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

#[link(name = "msi")]
extern "system" {
    fn MsiOpenDatabaseW(db_path: *const u16, persist: *const u16, db_handle: *mut isize) -> i32;
    fn MsiCloseHandle(handle: isize) -> i32;
    fn MsiGetDatabaseState(handle: isize) -> u32;
}

const ERROR_SUCCESS: i32 = 0;

fn main() {
    let path = OsStr::new("c:\\Users\\visse\\OneDrive\\Documentos\\V.E.L.O.C.I.T.Y.-Installer-master\\target\\t.msi")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    
    let persist = OsStr::new("read-only")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    
    let mut db_handle: isize = 0;
    
    unsafe {
        let result = MsiOpenDatabaseW(path.as_ptr(), persist.as_ptr(), &mut db_handle);
        println!("MsiOpenDatabaseW result: {} (SUCCESS={})", result, result == ERROR_SUCCESS);
        
        if result == ERROR_SUCCESS {
            let state = MsiGetDatabaseState(db_handle);
            println!("Database state: {} (1=ready)", state);
            
            let close_result = MsiCloseHandle(db_handle);
            println!("MsiCloseHandle result: {}", close_result);
        }
    }
    
    // Also test with the system MSI for comparison
    println!("\n--- Testing system MSI ---");
    let sys_path = OsStr::new("C:\\Windows\\Installer\\10d16cbb.msi")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    
    let mut sys_handle: isize = 0;
    unsafe {
        let result = MsiOpenDatabaseW(sys_path.as_ptr(), persist.as_ptr(), &mut sys_handle);
        println!("MsiOpenDatabaseW result: {} (SUCCESS={})", result, result == ERROR_SUCCESS);
        
        if result == ERROR_SUCCESS {
            let state = MsiGetDatabaseState(sys_handle);
            println!("Database state: {} (1=ready)", state);
            MsiCloseHandle(sys_handle);
        }
    }
}
