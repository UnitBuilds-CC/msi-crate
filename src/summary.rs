//! Summary Information stream for MSI packages
//!
//! Implements the OLE Property Set binary format as specified by Microsoft.
//! The Summary Information stream (\x05SummaryInformation) contains metadata
//! about the MSI package.
//!
//! Header layout (48 bytes for 1 section):
//!   BOM (2) + version (2) + OS (4) + CLSID (16) + reserved=1 (4) + FMTID (16) + offset=48 (4)
//!
//! Section layout:
//!   size (4) + prop_count (4) + index entries (8 each) + padding + values

use crate::error::Result;
use crate::string_pool::StringPool;
use chrono::{DateTime, Utc};
use std::io::Write;

// OLE Property Set VT_* type codes
const VT_I2: u32 = 2;
const VT_I4: u32 = 3;
const VT_LPSTR: u32 = 30;
const VT_FILETIME: u32 = 64;

// Summary Information property IDs
const PID_CODEPAGE: u32 = 1;
const PID_TITLE: u32 = 2;
const PID_SUBJECT: u32 = 3;
const PID_AUTHOR: u32 = 4;
const PID_KEYWORDS: u32 = 5;
const PID_COMMENTS: u32 = 6;
const PID_TEMPLATE: u32 = 7;
const PID_CREATE_TIME: u32 = 12;
const PID_LAST_SAVE_TIME: u32 = 13;
const PID_WORD_COUNT: u32 = 15;
const PID_CREATING_APP: u32 = 18;

/// FMTID for Summary Information: {F29F85E0-4FF9-1068-AB91-08002B27B3D9}
/// Mixed-endian UUID encoding (first 3 fields LE, last 2 fields BE).
const FMTID: [u8; 16] =
    *b"\xe0\x85\x9f\xf2\xf9\x4f\x68\x10\xab\x91\x08\x00\x2b\x27\xb3\xd9";

/// Summary Information properties for an MSI package
#[derive(Debug)]
pub struct SummaryInfo {
    pub title: Option<String>,
    pub subject: Option<String>,
    pub author: Option<String>,
    pub keywords: Option<String>,
    pub comments: Option<String>,
    /// Template: "arch;language" e.g. "x64;1033"
    pub template: Option<String>,
    pub created: Option<DateTime<Utc>>,
    pub modified: Option<DateTime<Utc>>,
    /// Code page (must be 1252 for Windows-1252)
    pub codepage: i16,
    /// Word count (2 for MSI packages)
    pub word_count: i32,
    /// Creating application string
    pub creating_app: Option<String>,
}

/// A property to be serialized into the OLE Property Set
struct Prop {
    id: u32,
    #[allow(dead_code)]
    vtype: u32,
    /// Value data (type code + value, padded to 4 bytes)
    data: Vec<u8>,
}

impl Prop {
    /// Total size including padding (always multiple of 4)
    fn padded_size(&self) -> u32 {
        self.data.len() as u32
    }
}

impl SummaryInfo {
    /// Create a new SummaryInfo with MSI defaults
    pub fn new() -> Self {
        Self {
            title: None,
            subject: None,
            author: None,
            keywords: None,
            comments: None,
            template: None,
            created: None,
            modified: None,
            codepage: 1252,
            word_count: 2,
            creating_app: None,
        }
    }

    /// Serialize to the OLE Property Set binary format.
    ///
    /// Codepage (PID 1) is always written first so readers know the
    /// encoding for all subsequent string properties.
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(256);

        // Collect all properties (codepage first!)
        let mut props: Vec<Prop> = Vec::new();

        // PID 1: Code page (VT_I2) - MUST be first
        props.push(Prop {
            id: PID_CODEPAGE,
            vtype: VT_I2,
            data: {
                let mut d = Vec::with_capacity(8);
                d.write_all(&VT_I2.to_le_bytes())?;
                d.write_all(&self.codepage.to_le_bytes())?;
                d.write_all(&0u16.to_le_bytes())?; // pad to 8 bytes
                d
            },
        });

        if let Some(ref s) = self.title {
            props.push(Self::lpstr_prop(PID_TITLE, s)?);
        }
        if let Some(ref s) = self.subject {
            props.push(Self::lpstr_prop(PID_SUBJECT, s)?);
        }
        if let Some(ref s) = self.author {
            props.push(Self::lpstr_prop(PID_AUTHOR, s)?);
        }
        if let Some(ref s) = self.keywords {
            props.push(Self::lpstr_prop(PID_KEYWORDS, s)?);
        }
        if let Some(ref s) = self.comments {
            props.push(Self::lpstr_prop(PID_COMMENTS, s)?);
        }
        if let Some(ref s) = self.template {
            props.push(Self::lpstr_prop(PID_TEMPLATE, s)?);
        }
        if let Some(ref dt) = self.created {
            props.push(Self::filetime_prop(PID_CREATE_TIME, dt));
        }
        if let Some(ref dt) = self.modified {
            props.push(Self::filetime_prop(PID_LAST_SAVE_TIME, dt));
        }

        // PID 15: Word count (VT_I4)
        props.push(Prop {
            id: PID_WORD_COUNT,
            vtype: VT_I4,
            data: {
                let mut d = Vec::with_capacity(8);
                d.write_all(&VT_I4.to_le_bytes())?;
                d.write_all(&self.word_count.to_le_bytes())?;
                d
            },
        });

        if let Some(ref s) = self.creating_app {
            props.push(Self::lpstr_prop(PID_CREATING_APP, s)?);
        }

        let num_props = props.len() as u32;

        // === Property Set Header (48 bytes for 1 section) ===
        buf.write_all(&0xFFFEu16.to_le_bytes())?; // Byte order mark
        buf.write_all(&0x0206u16.to_le_bytes())?; // Format version (matches Windows Installer)
        buf.write_all(&6u16.to_le_bytes())?; // OS version (low word)
        buf.write_all(&2u16.to_le_bytes())?; // OS version (high word, Win32=2)
        buf.write_all(&[0u8; 16])?; // CLSID (zeros)
        buf.write_all(&1u32.to_le_bytes())?; // Section count = 1
        buf.write_all(&FMTID)?; // FMTID (16 bytes)
        buf.write_all(&48u32.to_le_bytes())?; // Section offset = 48

        // === Build Section ===
        // Calculate section size:
        //   header (8) + index entries (8 * num_props) + values
        let index_end = 8 + num_props * 8;
        let mut current_offset = index_end;
        // Align to 4 bytes (should already be aligned since 8 + 8n is always 4-aligned)
        current_offset = (current_offset + 3) & !3;

        let mut section_size = current_offset;
        for prop in &props {
            section_size += prop.padded_size();
        }

        // Write section header
        buf.write_all(&section_size.to_le_bytes())?;
        buf.write_all(&num_props.to_le_bytes())?;

        // Write property index entries with calculated offsets
        let mut data_offset = current_offset;
        for prop in &props {
            buf.write_all(&prop.id.to_le_bytes())?;
            buf.write_all(&data_offset.to_le_bytes())?;
            data_offset += prop.padded_size();
        }

        // Pad between index and values (if needed for alignment)
        while (buf.len() - 48) < index_end as usize {
            buf.push(0);
        }
        // Ensure 4-byte alignment from section start
        while (buf.len() - 48) % 4 != 0 {
            buf.push(0);
        }

        // Write property values
        for prop in &props {
            buf.write_all(&prop.data)?;
        }

        Ok(buf)
    }

    /// Create a VT_LPSTR property
    fn lpstr_prop(id: u32, s: &str) -> Result<Prop> {
        let encoded = StringPool::encode(s)?;
        let str_len_with_null = (encoded.len() + 1) as u32;
        let mut data = Vec::with_capacity(16 + encoded.len());
        data.write_all(&VT_LPSTR.to_le_bytes())?; // type (4 bytes)
        data.write_all(&str_len_with_null.to_le_bytes())?; // length (4 bytes)
        data.write_all(&encoded)?; // string bytes
        data.push(0); // null terminator
        // Pad string data (length + string + null) to 4-byte boundary
        let str_data_len = 4 + str_len_with_null; // length field + string + null
        let padded_str_data = ((str_data_len + 3) >> 2) << 2;
        let padding = padded_str_data - str_data_len;
        for _ in 0..padding {
            data.push(0);
        }
        Ok(Prop { id, vtype: VT_LPSTR, data })
    }

    /// Create a VT_FILETIME property
    fn filetime_prop(id: u32, dt: &DateTime<Utc>) -> Prop {
        let timestamp = dt.timestamp();
        // Convert Unix epoch to FILETIME (100ns intervals since 1601-01-01)
        let filetime = ((timestamp as i64 + 11644473600i64) as u64) * 10_000_000;
        let mut data = Vec::with_capacity(12);
        data.write_all(&VT_FILETIME.to_le_bytes()).unwrap(); // type (4 bytes)
        data.write_all(&filetime.to_le_bytes()).unwrap(); // FILETIME (8 bytes)
        Prop { id, vtype: VT_FILETIME, data }
    }
}

impl Default for SummaryInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_u16(data: &[u8], off: usize) -> u16 {
        u16::from_le_bytes([data[off], data[off + 1]])
    }
    fn read_u32(data: &[u8], off: usize) -> u32 {
        u32::from_le_bytes([data[off], data[off+1], data[off+2], data[off+3]])
    }

    #[test]
    fn test_default_values() {
        let si = SummaryInfo::new();
        assert_eq!(si.codepage, 1252);
        assert_eq!(si.word_count, 2);
        assert!(si.title.is_none());
        assert!(si.author.is_none());
    }

    #[test]
    fn test_serialize_minimal() {
        let si = SummaryInfo::new();
        let data = si.serialize().unwrap();

        // Property Set Header: 48 bytes for 1 section
        assert!(data.len() >= 48);

        // BOM = 0xFFFE
        assert_eq!(read_u16(&data, 0), 0xFFFE);
        // Section count = 1
        assert_eq!(read_u32(&data, 24), 1);
        // Section offset = 48
        assert_eq!(read_u32(&data, 44), 48);

        // Section starts at offset 48
        let section_size = read_u32(&data, 48);
        let prop_count = read_u32(&data, 52);
        // Minimal: codepage + word_count = 2 properties
        assert_eq!(prop_count, 2);
        // Section size should be > 0
        assert!(section_size > 16);
    }

    #[test]
    fn test_serialize_with_all_properties() {
        let mut si = SummaryInfo::new();
        si.title = Some("Test Title".to_string());
        si.subject = Some("Test Subject".to_string());
        si.author = Some("Test Author".to_string());
        si.comments = Some("Test Comments".to_string());
        si.template = Some("x64;1033".to_string());
        si.creating_app = Some("Velocity Installer".to_string());
        si.created = Some(chrono::Utc::now());
        si.modified = Some(chrono::Utc::now());

        let data = si.serialize().unwrap();

        // Should have: codepage + title + subject + author + comments + template
        //              + created + modified + word_count + creating_app = 10 props
        let prop_count = read_u32(&data, 52);
        assert_eq!(prop_count, 10, "Should have 10 properties");

        // Total size should be reasonable
        assert!(data.len() > 200, "Full property set should be > 200 bytes");
    }

    #[test]
    fn test_codepage_is_first_property() {
        let si = SummaryInfo::new();
        let data = si.serialize().unwrap();

        // First property index entry starts at offset 56 (48 header + 8 section header)
        let first_pid = read_u32(&data, 56);
        assert_eq!(first_pid, PID_CODEPAGE, "First property must be codepage (PID 1)");
    }

    #[test]
    fn test_fmtid_matches_spec() {
        let si = SummaryInfo::new();
        let data = si.serialize().unwrap();

        // FMTID is at offset 28 (after BOM(2) + version(2) + OS(4) + CLSID(16) + count(4))
        let fmtid = &data[28..44];
        assert_eq!(fmtid, &FMTID, "FMTID must match Summary Information GUID");
    }

    #[test]
    fn test_section_size_is_4byte_aligned() {
        let mut si = SummaryInfo::new();
        si.title = Some("Test".to_string());
        let data = si.serialize().unwrap();

        let section_size = read_u32(&data, 48) as usize;
        assert_eq!(section_size % 4, 0, "Section size must be 4-byte aligned");
    }
}
