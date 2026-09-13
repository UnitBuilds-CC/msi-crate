//! Minimal cabinet (MSCF) file builder for MSI packages.
//!
//! Creates cabinet files with MSZIP compression (type 1), which uses
//! deflate per data block with a "CK" signature prefix.
//!
//! Follows the MS-CAB specification: CFData has a 12-byte header
//! (cChecksum:u32, cbData:u32, cbUncomp:u32). Large folder data is
//! split into multiple data blocks to avoid size overflow.

use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::io::Write;

const CHUNK_SIZE: usize = 32 * 1024;

/// A file to be included in the cabinet
pub struct CabinetFile {
    /// File name (as it appears in the cabinet, e.g. "myfile.txt")
    pub name: String,
    /// File content (raw bytes)
    pub data: Vec<u8>,
}

struct DataBlock {
    compressed: Vec<u8>,
    uncompressed_len: u32,
}

fn compress_chunk(data: &[u8]) -> DataBlock {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    let compressed = encoder.finish().unwrap();
    DataBlock {
        compressed,
        uncompressed_len: data.len() as u32,
    }
}

/// Build a cabinet file containing the given files.
/// Returns the complete cabinet bytes.
pub fn build_cabinet(files: &[CabinetFile]) -> Vec<u8> {
    let num_files: u16 = files.len() as u16;

    let mut folder_data = Vec::new();
    for file in files {
        folder_data.write_all(&file.data).unwrap();
    }

    let blocks: Vec<DataBlock> = if folder_data.is_empty() {
        vec![compress_chunk(&[])]
    } else {
        folder_data
            .chunks(CHUNK_SIZE.max(1))
            .map(compress_chunk)
            .collect()
    };

    let cfdata_header_size: u32 = 12; // cChecksum(4) + cbData(4) + cbUncomp(4)
    let cfolder_size: u32 = 8;        // coffCabStart(4) + cbCFData(2) + typeCompress(2)
    let cfheader_size: u32 = 36;

    let mut total_data_bytes: u32 = 0;
    for block in &blocks {
        let cb_data = 2 + block.compressed.len() as u32; // "CK" + compressed
        total_data_bytes += cfdata_header_size + cb_data;
    }

    let data_offset = cfheader_size + cfolder_size;
    let file_table_offset = data_offset + total_data_bytes;

    let mut file_table_size: u32 = 0;
    for file in files {
        file_table_size += 16 + file.name.len() as u32 + 1;
    }
    let total_size = file_table_offset + file_table_size;

    let mut buf = Vec::with_capacity(total_size as usize);

    // === CFHEADER (36 bytes) ===
    buf.write_all(b"MSCF").unwrap();
    buf.write_all(&0u32.to_le_bytes()).unwrap();             // reserved1
    buf.write_all(&total_size.to_le_bytes()).unwrap();       // cbCabinet
    buf.write_all(&0u32.to_le_bytes()).unwrap();             // reserved2
    buf.write_all(&file_table_offset.to_le_bytes()).unwrap(); // coffFiles
    buf.write_all(&0u32.to_le_bytes()).unwrap();             // reserved3
    buf.push(3);  // versionMinor
    buf.push(1);  // versionMajor
    buf.write_all(&1u16.to_le_bytes()).unwrap();             // cFolders
    buf.write_all(&num_files.to_le_bytes()).unwrap();        // cFiles
    buf.write_all(&0u16.to_le_bytes()).unwrap();             // flags
    buf.write_all(&1u16.to_le_bytes()).unwrap();             // setID
    buf.write_all(&0u16.to_le_bytes()).unwrap();             // iCabinet

    // === CFOLDER (8 bytes) ===
    buf.write_all(&data_offset.to_le_bytes()).unwrap();      // coffCabStart
    let first_block_cb = if let Some(b) = blocks.first() {
        (cfdata_header_size + 2 + b.compressed.len() as u32) as u16
    } else {
        0
    };
    buf.write_all(&first_block_cb.to_le_bytes()).unwrap();   // cbCFData (USHORT)
    buf.write_all(&1u16.to_le_bytes()).unwrap();             // typeCompress = MSZIP

    // === CFDATA blocks (12-byte header each) ===
    for block in &blocks {
        let cb_data = (2 + block.compressed.len()) as u32; // "CK" + compressed
        buf.write_all(&0u32.to_le_bytes()).unwrap();         // cChecksum
        buf.write_all(&cb_data.to_le_bytes()).unwrap();      // cbData (ULONG)
        buf.write_all(&block.uncompressed_len.to_le_bytes()).unwrap(); // cbUncomp (ULONG)
        buf.write_all(b"CK").unwrap();                       // MSZIP signature
        buf.write_all(&block.compressed).unwrap();
    }

    // === CFFILE entries ===
    let mut current_offset: u32 = 0;
    for file in files {
        let file_size = file.data.len() as u32;
        buf.write_all(&file_size.to_le_bytes()).unwrap();      // cbFile
        buf.write_all(&current_offset.to_le_bytes()).unwrap(); // uoffFolderStart
        buf.write_all(&0u16.to_le_bytes()).unwrap();           // iFolder
        buf.write_all(&0u16.to_le_bytes()).unwrap();           // flTime
        buf.write_all(&(48u16 << 9 | 1u16 << 5 | 24u16).to_le_bytes()).unwrap(); // flDate
        buf.write_all(&0x20u16.to_le_bytes()).unwrap();        // attribs: ARCHIVE
        buf.write_all(file.name.as_bytes()).unwrap();          // name
        buf.push(0);                                           // null terminator
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

    #[test]
    fn test_build_cabinet_large_file() {
        let data = vec![0x42u8; 100_000];
        let files = vec![CabinetFile {
            name: "large.bin".to_string(),
            data,
        }];
        let cab = build_cabinet(&files);
        assert_eq!(&cab[0..4], b"MSCF");

        let cfheader_size: u32 = 36;
        let cfolder_size: u32 = 8;
        let data_offset = cfheader_size + cfolder_size;

        let coff_cab_start = u32::from_le_bytes([
            cab[36], cab[37], cab[38], cab[39],
        ]);
        assert_eq!(coff_cab_start, data_offset);

        // CFDATA at data_offset: cChecksum(4) + cbData(4) + cbUncomp(4)
        let first_cbdata = u32::from_le_bytes([
            cab[data_offset as usize + 4],
            cab[data_offset as usize + 5],
            cab[data_offset as usize + 6],
            cab[data_offset as usize + 7],
        ]);
        let first_cbuncomp = u32::from_le_bytes([
            cab[data_offset as usize + 8],
            cab[data_offset as usize + 9],
            cab[data_offset as usize + 10],
            cab[data_offset as usize + 11],
        ]);
        assert!(first_cbdata > 0);
        assert_eq!(first_cbuncomp, CHUNK_SIZE as u32);
    }

    #[test]
    fn test_cab_data_fields_are_u32() {
        let files = vec![CabinetFile {
            name: "test.txt".to_string(),
            data: b"Hello".to_vec(),
        }];
        let cab = build_cabinet(&files);

        let cfdata_start = 36 + 8; // CFHEADER + CFOLDER

        // CFDATA: cChecksum(4) + cbData(4) + cbUncomp(4) + "CK" + compressed
        let cb_data = u32::from_le_bytes([
            cab[cfdata_start + 4],
            cab[cfdata_start + 5],
            cab[cfdata_start + 6],
            cab[cfdata_start + 7],
        ]);
        let cb_uncomp = u32::from_le_bytes([
            cab[cfdata_start + 8],
            cab[cfdata_start + 9],
            cab[cfdata_start + 10],
            cab[cfdata_start + 11],
        ]);
        assert_eq!(cb_uncomp, 5);
        assert!(cb_data > 2, "cbData must include CK marker + compressed bytes");
        assert_eq!(&cab[cfdata_start + 12..cfdata_start + 14], b"CK");
    }
}
