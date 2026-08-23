//! Minimal cabinet (MSCF) file builder for MSI packages.
//!
//! Creates cabinet files with MSZIP compression (type 1), which uses
//! deflate (zlib format) per data block with a "CK" signature prefix.

use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::io::Write;

/// A file to be included in the cabinet
pub struct CabinetFile {
    /// File name (as it appears in the cabinet, e.g. "myfile.txt")
    pub name: String,
    /// File content (raw bytes)
    pub data: Vec<u8>,
}

/// Build a cabinet file containing the given files.
/// Returns the complete cabinet bytes.
pub fn build_cabinet(files: &[CabinetFile]) -> Vec<u8> {
    let num_folders: u16 = 1; // Single folder for all files
    let num_files: u16 = files.len() as u16;

    // Concatenate all file data for the folder
    let mut folder_data = Vec::new();
    for file in files {
        folder_data.write_all(&file.data).unwrap();
    }

    // Compress with MSZIP (raw deflate)
    // MSZIP uses "CK" marker + raw deflate data (not zlib format).
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&folder_data).unwrap();
    let compressed = encoder.finish().unwrap();

    // MSZIP block: "CK" magic + zlib-compressed data
    let mszip_magic = 2u32; // "CK" = 2 bytes
    let cb_data = (mszip_magic + compressed.len() as u32) as u16;
    let cb_uncomp = folder_data.len() as u16;

    let header_size: u32 = 36;
    let folder_table_size: u32 = 8 * num_folders as u32;
    let data_offset = header_size + folder_table_size;
    let data_block_size = 8 + cb_data as u32; // CFDATA header (8) + data (CK + compressed)
    let file_table_offset = data_offset + data_block_size;

    // Calculate file table size
    // CFFILE: cbFile(4) + uoffFolderStart(4) + iFolder(2) + flTime(2) + flDate(2)
    //         + attribs(2) + name (variable) + null terminator (1)
    // Total fixed: 16 bytes per file
    let mut file_table_size: u32 = 0;
    for file in files {
        file_table_size += 16 + file.name.len() as u32 + 1;
    }
    let total_size = file_table_offset + file_table_size;

    let mut buf = Vec::with_capacity(total_size as usize);

    // === CFHEADER (36 bytes) ===
    buf.write_all(b"MSCF").unwrap();
    buf.write_all(&0u32.to_le_bytes()).unwrap();          // reserved1
    buf.write_all(&total_size.to_le_bytes()).unwrap();    // cbCabinet
    buf.write_all(&0u32.to_le_bytes()).unwrap();          // reserved2
    buf.write_all(&file_table_offset.to_le_bytes()).unwrap(); // coffFiles
    buf.write_all(&0u32.to_le_bytes()).unwrap();          // reserved3
    buf.push(3);  // versionMinor
    buf.push(1);  // versionMajor
    buf.write_all(&num_folders.to_le_bytes()).unwrap();   // cFolders
    buf.write_all(&num_files.to_le_bytes()).unwrap();     // cFiles
    buf.write_all(&0u16.to_le_bytes()).unwrap();          // flags
    buf.write_all(&1u16.to_le_bytes()).unwrap();          // setID
    buf.write_all(&0u16.to_le_bytes()).unwrap();          // iCabinet

    // === CFOLDER (8 bytes) ===
    // Note: cbCFData is USHORT (2 bytes), not ULONG
    buf.write_all(&data_offset.to_le_bytes()).unwrap();     // coffCabStart (4 bytes)
    buf.write_all(&((data_block_size as u16).to_le_bytes())).unwrap(); // cbCFData (2 bytes)
    buf.write_all(&1u16.to_le_bytes()).unwrap();            // typeCompression = MSZIP (2 bytes)

    // === CFDATA (8 + data bytes) ===
    buf.write_all(&0u32.to_le_bytes()).unwrap();          // cChecksum
    buf.write_all(&cb_data.to_le_bytes()).unwrap();       // cbData (compressed + CK)
    buf.write_all(&cb_uncomp.to_le_bytes()).unwrap();     // cbUncomp (original size)
    buf.write_all(b"CK").unwrap();                        // MSZIP signature
    buf.write_all(&compressed).unwrap();                  // zlib-compressed data

    // === CFFILE entries ===
    let mut current_offset: u32 = 0;
    for file in files {
        let file_size = file.data.len() as u32;
        buf.write_all(&file_size.to_le_bytes()).unwrap();      // cbFile (4 bytes)
        buf.write_all(&current_offset.to_le_bytes()).unwrap(); // uoffFolderStart (4 bytes)
        buf.write_all(&0u16.to_le_bytes()).unwrap();           // iFolder (2 bytes)
        buf.write_all(&0u16.to_le_bytes()).unwrap();           // flTime (2 bytes)
        buf.write_all(&(48u16 << 9 | 1u16 << 5 | 24u16).to_le_bytes()).unwrap(); // flDate (2 bytes)
        buf.write_all(&0x20u16.to_le_bytes()).unwrap();        // attribs: ARCHIVE (2 bytes)
        buf.write_all(file.name.as_bytes()).unwrap();          // name (variable)
        buf.push(0); // null terminator
        current_offset += file_size;
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_empty_cabinet() {
        let cab = build_cabinet(&[]);
        assert!(cab.len() >= 36);
        assert_eq!(&cab[0..4], b"MSCF");
    }

    #[test]
    fn test_build_cabinet_with_file() {
        let files = vec![
            CabinetFile {
                name: "test.txt".to_string(),
                data: b"Hello, World!".to_vec(),
            },
        ];
        let cab = build_cabinet(&files);
        assert_eq!(&cab[0..4], b"MSCF");
        assert!(cab.len() > 36);
    }
}
