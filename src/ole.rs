//! OLE Compound File writer - from scratch
//!
//! Implements MS-CFB for V3 files (512-byte sectors, 64-byte mini-sectors).
//! V3 is the format required by Windows Installer (MSI) packages.
//! Supports both mini streams (< 4096 bytes) and regular streams (>= 4096 bytes).

const SECTOR_SHIFT: u16 = 9;  // V3: 512 bytes (2^9)
const SECTOR_SIZE: usize = 512;
const MINI_SECTOR_SHIFT: u16 = 6;
const MINI_SECTOR_SIZE: usize = 64;
const MINI_STREAM_CUTOFF: u32 = 4096;
const HEADER_SIZE: usize = 512;  // V3: 512 bytes
const DIR_ENTRY_SIZE: usize = 128;
const DIFAT_IN_HEADER: usize = 109;
const ENTRIES_PER_DIR_SECTOR: usize = SECTOR_SIZE / DIR_ENTRY_SIZE; // 4 for V3

const FREE_SECT: u32 = 0xFFFF_FFFF;
const ENDOFCHAIN: u32 = 0xFFFF_FFFE;
const FATSECT: u32 = 0xFFFF_FFFD;

const OBJTYPE_ROOT: u8 = 5;
const OBJTYPE_STREAM: u8 = 2;

/// MSI CLSID: {000C1084-0000-0000-C000-000000000046}
const MSI_CLSID: [u8; 16] = [
    0x84, 0x10, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
];

/// A stream to write into the OLE file
#[derive(Clone)]
pub struct OleStream {
    pub name: String,
    pub data: Vec<u8>,
}

/// Build a complete OLE V3 compound file from the given streams.
pub fn build_ole_file(streams: &[OleStream]) -> Vec<u8> {
    OleWriter::build(streams)
}

struct OleWriter {
    names: Vec<String>,
    data: Vec<Vec<u8>>,
    // Per-stream layout info (index matches names/data, 0=root)
    is_mini: Vec<bool>,         // true = mini stream, false = regular sectors
    start_mini: Vec<u32>,       // starting mini sector (for mini streams)
    start_sector: Vec<u32>,     // starting regular sector (for large streams)
    mini_stream: Vec<u8>,       // concatenated mini stream data
    // Sector layout
    num_fat_sectors: usize,
    num_difat_sectors: usize,   // DIFAT sectors for >109 FAT sectors
    num_dir_sectors: usize,
    num_minifat_sectors: usize,
    mini_stream_sectors: usize,
    first_fat_sector: usize,
    first_difat_sector: usize,
    first_dir_sector: usize,
    first_minifat_sector: usize,
    first_mini_sector: usize,
    // Large stream data: (sector_start, data)
    large_streams: Vec<(usize, Vec<u8>)>,
    total_sectors: usize,
}

impl OleWriter {
    fn build(streams: &[OleStream]) -> Vec<u8> {
        let n = streams.len();
        let mut w = OleWriter {
            names: Vec::with_capacity(n + 1),
            data: Vec::with_capacity(n + 1),
            is_mini: vec![true; n + 1],
            start_mini: vec![0; n + 1],
            start_sector: vec![0; n + 1],
            mini_stream: Vec::new(),
            num_fat_sectors: 1,
            num_difat_sectors: 0,
            num_dir_sectors: 1,
            num_minifat_sectors: 1,
            mini_stream_sectors: 0,
            first_fat_sector: 0,
            first_difat_sector: 0,
            first_dir_sector: 0,
            first_minifat_sector: 0,
            first_mini_sector: 0,
            large_streams: Vec::new(),
            total_sectors: 0,
        };

        // Root entry at index 0
        w.names.push("Root Entry".to_string());
        w.data.push(Vec::new());

        for s in streams {
            w.names.push(s.name.clone());
            w.data.push(s.data.clone());
        }

        w.compute_layout();
        w.write_file()
    }

    fn compute_layout(&mut self) {
        // Sort stream IDs by name (case-insensitive shortlex) to match directory order.
        // The mini stream MUST be laid out in the same order as directory entries,
        // because directory entries reference mini-stream positions by index.
        let mut sorted_ids: Vec<usize> = (1..self.names.len()).collect();
        sorted_ids.sort_by(|&a, &b| {
            let name_a = &self.names[a];
            let name_b = &self.names[b];
            let len_a = name_a.encode_utf16().count();
            let len_b = name_b.encode_utf16().count();
            match len_a.cmp(&len_b) {
                std::cmp::Ordering::Equal => {
                    let chars_a = name_a.chars().map(|c| c.to_uppercase().next().unwrap_or(c));
                    let chars_b = name_b.chars().map(|c| c.to_uppercase().next().unwrap_or(c));
                    chars_a.cmp(chars_b)
                }
                other => other,
            }
        });

        // Separate mini and large streams, laying out mini streams in SORTED order
        let mut mini_total = 0usize;
        let mut large_total = 0usize;

        for &i in &sorted_ids {
            let len = self.data[i].len();
            if len < MINI_STREAM_CUTOFF as usize {
                self.is_mini[i] = true;
                let padded = len.div_ceil(MINI_SECTOR_SIZE) * MINI_SECTOR_SIZE;
                self.start_mini[i] = (mini_total / MINI_SECTOR_SIZE) as u32;
                mini_total += padded;
            } else {
                self.is_mini[i] = false;
                let sectors = len.div_ceil(SECTOR_SIZE);
                large_total += sectors;
            }
        }

        // Build mini stream data in SORTED order.
        // Directory entries at positions 1..=N correspond to sorted_ids[0..N],
        // so start_mini must be assigned in sorted order to match.
        self.mini_stream = vec![0u8; mini_total];
        let mut offset = 0;
        for &i in &sorted_ids {
            if self.is_mini[i] {
                let data = &self.data[i];
                self.mini_stream[offset..offset + data.len()].copy_from_slice(data);
                let padded = data.len().div_ceil(MINI_SECTOR_SIZE) * MINI_SECTOR_SIZE;
                offset += padded;
            }
        }

        self.mini_stream_sectors = mini_total.div_ceil(SECTOR_SIZE);

        // Calculate MiniFAT requirements: each MiniFAT sector holds 128 entries
        let mini_entries_per_minifat = SECTOR_SIZE / 4; // 128
        let total_mini_sectors = mini_total.div_ceil(MINI_SECTOR_SIZE);
        self.num_minifat_sectors = (total_mini_sectors.max(1)).div_ceil(mini_entries_per_minifat);

        // Iteratively compute sector layout including DIFAT sectors.
        // Layout: [FAT sectors] [DIFAT sectors] [dir sectors] [minifat sectors] [mini stream] [large stream data]
        //
        // DIFAT (Double Indirect FAT) is needed when we have more than 109 FAT sectors.
        // The header DIFAT array holds 109 entries; overflow goes into DIFAT sectors.
        // Each DIFAT sector holds 127 FAT sector indices + 1 next-DIFAT pointer.
        loop {
            let fixed = self.num_fat_sectors + self.num_dir_sectors + self.num_minifat_sectors;
            let variable = self.mini_stream_sectors + large_total;
            let total = fixed + variable;

            // How many FAT entries do we need?
            let entries_per_fat = SECTOR_SIZE / 4; // 1024
            let needed_fat = total.div_ceil(entries_per_fat);

            // How many directory sectors do we need?
            let needed_dir = self.names.len().div_ceil(ENTRIES_PER_DIR_SECTOR);

            if needed_fat == self.num_fat_sectors && needed_dir == self.num_dir_sectors {
                // Layout is stable - calculate DIFAT requirements
                if needed_fat > DIFAT_IN_HEADER {
                    let overflow = needed_fat - DIFAT_IN_HEADER;
                    let difat_entries_per_sector = SECTOR_SIZE / 4 - 1; // 127
                    self.num_difat_sectors = overflow.div_ceil(difat_entries_per_sector);
                } else {
                    self.num_difat_sectors = 0;
                }

                // Assign sector positions
                // Layout: [FAT] [DIFAT] [DIR] [MINIFAT] [MINI STREAM] [LARGE STREAMS]
                self.first_fat_sector = 0;
                self.first_difat_sector = self.num_fat_sectors;
                self.first_dir_sector = self.num_fat_sectors + self.num_difat_sectors;
                self.first_minifat_sector = self.first_dir_sector + self.num_dir_sectors;
                self.first_mini_sector = self.first_minifat_sector + self.num_minifat_sectors;

                // Assign regular sectors to large streams
                let mut next_sector = self.first_mini_sector + self.mini_stream_sectors;
                self.large_streams.clear();
                for i in 1..self.names.len() {
                    if !self.is_mini[i] {
                        self.start_sector[i] = next_sector as u32;
                        let sectors = self.data[i].len().div_ceil(SECTOR_SIZE);
                        self.large_streams.push((next_sector, self.data[i].clone()));
                        next_sector += sectors;
                    }
                }

                self.total_sectors = next_sector;
                break;
            }

            self.num_fat_sectors = needed_fat;
            self.num_dir_sectors = needed_dir;
        }
    }

    fn write_file(&self) -> Vec<u8> {
        let file_size = HEADER_SIZE + self.total_sectors * SECTOR_SIZE;
        let mut file = vec![0u8; file_size];

        self.write_header(&mut file);
        self.write_fat(&mut file);
        self.write_directory(&mut file);
        self.write_minifat(&mut file);
        self.write_mini_stream(&mut file);
        self.write_large_streams(&mut file);

        file
    }

    fn sector_offset(&self, sector: usize) -> usize {
        HEADER_SIZE + sector * SECTOR_SIZE
    }

    fn write_header(&self, file: &mut [u8]) {
        let mut o = 0;
        file[o..o + 8].copy_from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
        o += 8;
        o += 16; // CLSID (zeros)
        file[o..o + 2].copy_from_slice(&0x003Eu16.to_le_bytes()); o += 2; // minor ver (standard CFB value)
        file[o..o + 2].copy_from_slice(&3u16.to_le_bytes()); o += 2; // major ver (V3 for MSI)
        file[o..o + 2].copy_from_slice(&0xFFFEu16.to_le_bytes()); o += 2; // byte order
        file[o..o + 2].copy_from_slice(&SECTOR_SHIFT.to_le_bytes()); o += 2;
        file[o..o + 2].copy_from_slice(&MINI_SECTOR_SHIFT.to_le_bytes()); o += 2;
        o += 6; // reserved
        // For V3, NumDirSectors = 0 (per MS-CFB spec, SHOULD be 0 for V3).
        file[o..o + 4].copy_from_slice(&0u32.to_le_bytes()); o += 4;
        file[o..o + 4].copy_from_slice(&(self.num_fat_sectors as u32).to_le_bytes()); o += 4;
        file[o..o + 4].copy_from_slice(&(self.first_dir_sector as u32).to_le_bytes()); o += 4;
        o += 4; // transaction sig
        file[o..o + 4].copy_from_slice(&MINI_STREAM_CUTOFF.to_le_bytes()); o += 4;
        file[o..o + 4].copy_from_slice(&(self.first_minifat_sector as u32).to_le_bytes()); o += 4;
        file[o..o + 4].copy_from_slice(&(self.num_minifat_sectors as u32).to_le_bytes()); o += 4;
        // DIFAT: first sector and count
        let first_difat = if self.num_difat_sectors > 0 { self.first_difat_sector as u32 } else { FREE_SECT };
        file[o..o + 4].copy_from_slice(&first_difat.to_le_bytes()); o += 4;
        file[o..o + 4].copy_from_slice(&(self.num_difat_sectors as u32).to_le_bytes()); o += 4;
        // DIFAT array: first N entries are FAT sector numbers, rest must be FREE_SECT
        for i in 0..DIFAT_IN_HEADER {
            let val = if i < self.num_fat_sectors {
                (self.first_fat_sector + i) as u32
            } else {
                FREE_SECT  // MS-CFB: unused header DIFAT entries = FREE_SECT (-1)
            };
            file[o..o + 4].copy_from_slice(&val.to_le_bytes());
            o += 4;
        }
    }

    fn write_fat(&self, file: &mut [u8]) {
        // Initialize all FAT entries to FREE
        for i in 0..self.num_fat_sectors {
            let base = self.sector_offset(self.first_fat_sector + i);
            for j in (0..SECTOR_SIZE).step_by(4) {
                file[base + j..base + j + 4].copy_from_slice(&FREE_SECT.to_le_bytes());
            }
        }

        // Mark FAT sectors as FATSECT
        for i in 0..self.num_fat_sectors {
            let off = self.sector_offset(self.first_fat_sector + i);
            let idx = i * (SECTOR_SIZE / 4);
            let fat_off = off + idx * 4;
            file[fat_off..fat_off + 4].copy_from_slice(&FATSECT.to_le_bytes());
        }

        // Mark DIFAT sectors as FATSECT
        for i in 0..self.num_difat_sectors {
            let off = self.sector_offset(self.first_difat_sector + i);
            let idx = i * (SECTOR_SIZE / 4);
            let fat_off = off + idx * 4;
            file[fat_off..fat_off + 4].copy_from_slice(&FATSECT.to_le_bytes());
        }

        // Directory sectors → ENDOFCHAIN chain
        for i in 0..self.num_dir_sectors {
            let sect = self.first_dir_sector + i;
            let fat_off = self.sector_offset(self.first_fat_sector) + sect * 4;
            let val = if i + 1 < self.num_dir_sectors {
                (sect + 1) as u32
            } else {
                ENDOFCHAIN
            };
            file[fat_off..fat_off + 4].copy_from_slice(&val.to_le_bytes());
        }

        // MiniFAT sectors → ENDOFCHAIN chain
        for i in 0..self.num_minifat_sectors {
            let sect = self.first_minifat_sector + i;
            let fat_off = self.sector_offset(self.first_fat_sector) + sect * 4;
            let val = if i + 1 < self.num_minifat_sectors {
                (sect + 1) as u32
            } else {
                ENDOFCHAIN
            };
            file[fat_off..fat_off + 4].copy_from_slice(&val.to_le_bytes());
        }

        // Mini stream sectors → chain
        for i in 0..self.mini_stream_sectors {
            let sect = self.first_mini_sector + i;
            let fat_off = self.sector_offset(self.first_fat_sector) + sect * 4;
            let val = if i + 1 < self.mini_stream_sectors {
                (sect + 1) as u32
            } else {
                ENDOFCHAIN
            };
            file[fat_off..fat_off + 4].copy_from_slice(&val.to_le_bytes());
        }

        // Large stream sectors → chains
        for (start, data) in &self.large_streams {
            let num_sectors = data.len().div_ceil(SECTOR_SIZE);
            for i in 0..num_sectors {
                let sect = start + i;
                let fat_off = self.sector_offset(self.first_fat_sector) + sect * 4;
                let val = if i + 1 < num_sectors {
                    (sect + 1) as u32
                } else {
                    ENDOFCHAIN
                };
                file[fat_off..fat_off + 4].copy_from_slice(&val.to_le_bytes());
            }
        }

        // Write DIFAT sectors (containing FAT sector indices)
        if self.num_difat_sectors > 0 {
            let difat_entries_per_sector = SECTOR_SIZE / 4 - 1; // 127

            for d in 0..self.num_difat_sectors {
                let base = self.sector_offset(self.first_difat_sector + d);

                // Initialize entire DIFAT sector to FREE_SECT
                for j in (0..SECTOR_SIZE).step_by(4) {
                    file[base + j..base + j + 4].copy_from_slice(&FREE_SECT.to_le_bytes());
                }

                // Fill DIFAT entries with FAT sector indices
                let start_idx = DIFAT_IN_HEADER + d * difat_entries_per_sector;
                let end_idx = (start_idx + difat_entries_per_sector).min(self.num_fat_sectors);

                for i in start_idx..end_idx {
                    let entry_off = base + (i - start_idx) * 4;
                    file[entry_off..entry_off + 4].copy_from_slice(&((self.first_fat_sector + i) as u32).to_le_bytes());
                }

                // Next DIFAT sector pointer (last entry in the sector)
                let next_difat = if d + 1 < self.num_difat_sectors {
                    (self.first_difat_sector + d + 1) as u32
                } else {
                    ENDOFCHAIN
                };
                let next_off = base + difat_entries_per_sector * 4;
                file[next_off..next_off + 4].copy_from_slice(&next_difat.to_le_bytes());
            }
        }
    }

    fn write_directory(&self, file: &mut [u8]) {
        let base = self.sector_offset(self.first_dir_sector);
        let num_entries = self.names.len();

        // Sort stream IDs according to CFB directory ordering:
        // case-insensitive shortlex order (matching the cfb crate).
        // 1. Shorter names (by UTF-16 code unit count) come first.
        // 2. Same-length names compared by uppercased char code points.
        let mut sorted_ids: Vec<usize> = (1..num_entries).collect();
        sorted_ids.sort_by(|&a, &b| {
            let name_a = &self.names[a];
            let name_b = &self.names[b];
            let len_a = name_a.encode_utf16().count();
            let len_b = name_b.encode_utf16().count();
            match len_a.cmp(&len_b) {
                std::cmp::Ordering::Equal => {
                    let chars_a = name_a.chars().map(|c| c.to_uppercase().next().unwrap_or(c));
                    let chars_b = name_b.chars().map(|c| c.to_uppercase().next().unwrap_or(c));
                    chars_a.cmp(chars_b)
                }
                other => other,
            }
        });

        // Build balanced BST using DIRECTORY POSITIONS as node IDs.
        // Position 0 = root of BST, positions 1..N = children.
        // This ensures tree pointers reference directory entry positions
        // that satisfy the BST property (left < parent < right by name).
        let tree = Self::build_dir_tree_by_position(&sorted_ids);
        let tree_root = tree.root;

        // Write root entry at position 0
        self.write_dir_entry_pos(file, base, 0, 0, tree_root, &tree, &sorted_ids);

        // Write stream entries at positions 1..N (sorted_ids[i-1] = stream for position i)
        for i in 0..sorted_ids.len() {
            let dir_pos = i + 1;
            let stream_id = sorted_ids[i];
            self.write_dir_entry_pos(file, base, dir_pos, stream_id, tree_root, &tree, &sorted_ids);
        }
    }

    /// Write a directory entry at the given directory position.
    /// `dir_pos` = position in directory (0 = root, 1+ = streams in sorted order).
    /// `stream_id` = index into names/data arrays.
    /// Tree node IDs are directory positions (0, 1, 2, ...).
    fn write_dir_entry_pos(&self, file: &mut [u8], base: usize, dir_pos: usize, stream_id: usize,
                           root_child: i32, tree: &DirTree, _sorted_ids: &[usize]) {
        let off = base + dir_pos * DIR_ENTRY_SIZE;
        let name_utf16: Vec<u16> = self.names[stream_id].encode_utf16().collect();
        let name_byte_len = name_utf16.len() * 2;
        let copy_bytes = name_byte_len.min(64);
        let name_bytes: Vec<u8> = name_utf16.iter()
            .flat_map(|&c| c.to_le_bytes())
            .take(copy_bytes)
            .collect();
        file[off..off + name_bytes.len()].copy_from_slice(&name_bytes);

        let name_len_with_null = (name_utf16.len() as u16 + 1) * 2;
        file[off + 64..off + 66].copy_from_slice(&name_len_with_null.to_le_bytes());
        file[off + 66] = if dir_pos == 0 { OBJTYPE_ROOT } else { OBJTYPE_STREAM };
        // Color: 0 = red, 1 = black
        let color = if dir_pos == 0 { 1 } else { tree.colors.get(&dir_pos).copied().unwrap_or(1) };
        file[off + 67] = color;

        let (left, right, child) = if dir_pos == 0 {
            (-1i32, -1i32, root_child)
        } else {
            let l = tree.left.get(&dir_pos).copied().unwrap_or(-1);
            let r = tree.right.get(&dir_pos).copied().unwrap_or(-1);
            (l, r, -1)
        };
        file[off + 68..off + 72].copy_from_slice(&left.to_le_bytes());
        file[off + 72..off + 76].copy_from_slice(&right.to_le_bytes());
        file[off + 76..off + 80].copy_from_slice(&child.to_le_bytes());

        // CLSID on root entry - MSI CLSID: {000C1084-0000-0000-C000-000000000046}
        if dir_pos == 0 {
            file[off + 80..off + 96].copy_from_slice(&MSI_CLSID);
        }

        // Starting sector/mini-sector
        if dir_pos == 0 {
            file[off + 116..off + 120].copy_from_slice(&(self.first_mini_sector as u32).to_le_bytes());
        } else if self.is_mini[stream_id] {
            file[off + 116..off + 120].copy_from_slice(&self.start_mini[stream_id].to_le_bytes());
        } else {
            file[off + 116..off + 120].copy_from_slice(&self.start_sector[stream_id].to_le_bytes());
        }

        let size = if dir_pos == 0 {
            self.mini_stream.len() as u64
        } else {
            self.data[stream_id].len() as u64
        };
        file[off + 120..off + 128].copy_from_slice(&size.to_le_bytes());
    }

    fn write_minifat(&self, file: &mut [u8]) {
        // Initialize ALL MiniFAT sectors to FREE (not just the first one)
        for s in 0..self.num_minifat_sectors {
            let base = self.sector_offset(self.first_minifat_sector + s);
            for j in (0..SECTOR_SIZE).step_by(4) {
                file[base + j..base + j + 4].copy_from_slice(&FREE_SECT.to_le_bytes());
            }
        }

        // Write MiniFAT entries at the correct offsets
        // MiniFAT sectors are contiguous, so we can use a linear offset from the first sector.
        let minifat_base = self.sector_offset(self.first_minifat_sector);
        for i in 1..self.names.len() {
            if !self.is_mini[i] { continue; }
            let data_len = self.data[i].len() as u32;
            if data_len == 0 { continue; }
            let num_ms = data_len.div_ceil(MINI_SECTOR_SIZE as u32);
            let start = self.start_mini[i];
            for j in 0..num_ms {
                let mini_sect = start + j;
                let value = if j + 1 == num_ms { ENDOFCHAIN } else { mini_sect + 1 };
                let off = minifat_base + (mini_sect as usize) * 4;
                file[off..off + 4].copy_from_slice(&value.to_le_bytes());
            }
        }
    }

    fn write_mini_stream(&self, file: &mut [u8]) {
        let base = self.sector_offset(self.first_mini_sector);
        file[base..base + self.mini_stream.len()].copy_from_slice(&self.mini_stream);
    }

    fn write_large_streams(&self, file: &mut [u8]) {
        for (start, data) in &self.large_streams {
            let base = self.sector_offset(*start);
            file[base..base + data.len()].copy_from_slice(data);
        }
    }
}

/// Precomputed directory tree structure (balanced BST with red-black coloring).
struct DirTree {
    root: i32,
    left: std::collections::HashMap<usize, i32>,
    right: std::collections::HashMap<usize, i32>,
    colors: std::collections::HashMap<usize, u8>,
}

impl OleWriter {
    /// Build a balanced BST using directory positions as node IDs.
    ///
    /// Position 0 = root entry. Positions 1..N = stream entries in sorted order.
    /// The BST is built so that for any node at position P:
    /// - Left subtree nodes have positions < P (smaller names)
    /// - Right subtree nodes have positions > P (larger names)
    /// This satisfies the OLE directory BST property.
    fn build_dir_tree_by_position(sorted_ids: &[usize]) -> DirTree {
        let mut tree = DirTree {
            root: -1,
            left: std::collections::HashMap::new(),
            right: std::collections::HashMap::new(),
            colors: std::collections::HashMap::new(),
        };

        if sorted_ids.is_empty() {
            return tree;
        }

        // Node 0 = root. Nodes 1..=N = streams in sorted order.
        // Build a balanced BST from positions 1..=N.
        let n = sorted_ids.len();
        let positions: Vec<usize> = (1..=n).collect();

        fn build_subtree(positions: &[usize], tree: &mut DirTree) -> i32 {
            if positions.is_empty() {
                return -1;
            }
            let mid = positions.len() / 2;
            let pos = positions[mid];
            tree.colors.insert(pos, 1); // all black
            let left = build_subtree(&positions[..mid], tree);
            let right = build_subtree(&positions[mid + 1..], tree);
            tree.left.insert(pos, left);
            tree.right.insert(pos, right);
            pos as i32
        }

        tree.root = build_subtree(&positions, &mut tree);
        tree
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Header helpers ──────────────────────────────────────────────

    fn read_u16(data: &[u8], off: usize) -> u16 {
        u16::from_le_bytes([data[off], data[off + 1]])
    }
    fn read_u32(data: &[u8], off: usize) -> u32 {
        u32::from_le_bytes([data[off], data[off+1], data[off+2], data[off+3]])
    }

    /// Parse the 76-byte OLE header and return key fields
    struct OleHeader {
        major_version: u16,
        sector_shift: u16,
        mini_sector_shift: u16,
        num_dir_sectors: u32,
        num_fat_sectors: u32,
        first_dir_sector: u32,
        mini_stream_cutoff: u32,
        first_minifat_sector: u32,
        num_minifat_sectors: u32,
        difat_entries: Vec<u32>,
    }

    fn parse_header(data: &[u8]) -> OleHeader {
        assert_eq!(&data[0..8], &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
        let major_version = read_u16(data, 26);   // offset 26: major version
        let sector_shift = read_u16(data, 30);
        let mini_sector_shift = read_u16(data, 32);
        let num_dir_sectors = read_u32(data, 40);
        let num_fat_sectors = read_u32(data, 44);
        let first_dir_sector = read_u32(data, 48);
        let mini_stream_cutoff = read_u32(data, 56);
        let first_minifat_sector = read_u32(data, 60);
        let num_minifat_sectors = read_u32(data, 64);

        let mut difat_entries = Vec::new();
        for i in 0..DIFAT_IN_HEADER {
            let off = 76 + i * 4;
            let val = read_u32(data, off);
            if val != FREE_SECT {
                difat_entries.push(val);
            }
        }

        OleHeader {
            major_version,
            sector_shift,
            mini_sector_shift,
            num_dir_sectors,
            num_fat_sectors,
            first_dir_sector,
            mini_stream_cutoff,
            first_minifat_sector,
            num_minifat_sectors,
            difat_entries,
        }
    }

    /// Read a FAT entry from the first FAT sector
    fn read_fat_entry(data: &[u8], header: &OleHeader, sector: u32) -> u32 {
        let fat_start = HEADER_SIZE + header.difat_entries[0] as usize * SECTOR_SIZE;
        read_u32(data, fat_start + sector as usize * 4)
    }

    /// Parse a directory entry at a given index
    struct DirEntry {
        name: String,
        obj_type: u8,
        #[allow(dead_code)]
        left: i32,
        #[allow(dead_code)]
        right: i32,
        child: i32,
        start_sector: u32,
        stream_size: u64,
    }

    fn parse_dir_entry(data: &[u8], header: &OleHeader, idx: usize) -> DirEntry {
        let base = HEADER_SIZE + header.first_dir_sector as usize * SECTOR_SIZE + idx * DIR_ENTRY_SIZE;
        let name_len = read_u16(data, base + 64) as usize; // includes null terminator
        let name_bytes = name_len.saturating_sub(2); // exclude null
        let mut name_utf16 = Vec::new();
        for i in 0..(name_bytes / 2) {
            name_utf16.push(read_u16(data, base + i * 2));
        }
        let name = String::from_utf16_lossy(&name_utf16);
        let obj_type = data[base + 66];
        let left = read_u32(data, base + 68) as i32;
        let right = read_u32(data, base + 72) as i32;
        let child = read_u32(data, base + 76) as i32;
        let start_sector = read_u32(data, base + 116);
        let stream_size = u64::from_le_bytes([
            data[base+120], data[base+121], data[base+122], data[base+123],
            data[base+124], data[base+125], data[base+126], data[base+127],
        ]);
        DirEntry { name, obj_type, left, right, child, start_sector, stream_size }
    }

    /// Follow a FAT chain starting from `start` and return all sector indices
    fn follow_chain(data: &[u8], header: &OleHeader, start: u32) -> Vec<u32> {
        let mut chain = Vec::new();
        let mut current = start;
        loop {
            chain.push(current);
            let next = read_fat_entry(data, header, current);
            if next == ENDOFCHAIN || next == FREE_SECT { break; }
            current = next;
        }
        chain
    }

    /// Read stream data from sector chain
    fn read_stream_data(data: &[u8], header: &OleHeader, start: u32, size: usize) -> Vec<u8> {
        let chain = follow_chain(data, header, start);
        let mut result = Vec::with_capacity(size);
        for (i, &sect) in chain.iter().enumerate() {
            let off = HEADER_SIZE + sect as usize * SECTOR_SIZE;
            let remaining = size - i * SECTOR_SIZE;
            let to_read = remaining.min(SECTOR_SIZE);
            result.extend_from_slice(&data[off..off + to_read]);
        }
        result
    }

    /// Read mini-stream data for a given mini-sector start and size
    fn read_mini_stream_data(
        data: &[u8], header: &OleHeader,
        mini_start: u32, size: usize,
        root_start: u32, root_size: u64,
    ) -> Vec<u8> {
        // First, read the root entry's mini-stream container
        let mini_container = read_stream_data(data, header, root_start, root_size as usize);

        // Follow the MiniFAT chain
        let minifat_base = HEADER_SIZE + header.first_minifat_sector as usize * SECTOR_SIZE;
        let mut chain = Vec::new();
        let mut current = mini_start;
        loop {
            chain.push(current);
            let off = minifat_base + current as usize * 4;
            let next = read_u32(data, off);
            if next == ENDOFCHAIN || next == FREE_SECT { break; }
            current = next;
        }

        let mut result = Vec::with_capacity(size);
        for (i, &ms) in chain.iter().enumerate() {
            let off = ms as usize * MINI_SECTOR_SIZE;
            let remaining = size - i * MINI_SECTOR_SIZE;
            let to_read = remaining.min(MINI_SECTOR_SIZE);
            result.extend_from_slice(&mini_container[off..off + to_read]);
        }
        result
    }

    // ── Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_empty_file() {
        let data = build_ole_file(&[]);
        let h = parse_header(&data);

        assert_eq!(h.major_version, 3, "Must be V3");
        assert_eq!(h.sector_shift, 9, "512-byte sectors");
        assert_eq!(h.mini_sector_shift, 6, "64-byte mini-sectors");
        assert_eq!(h.mini_stream_cutoff, 4096);
        assert_eq!(h.num_dir_sectors, 0, "V3: NumDirSectors = 0");
        assert_eq!(h.num_fat_sectors, 1);
        assert_eq!(h.num_minifat_sectors, 1);
        assert_eq!(h.difat_entries.len(), 1);

        // Root entry should exist
        let root = parse_dir_entry(&data, &h, 0);
        assert_eq!(root.name, "Root Entry");
        assert_eq!(root.obj_type, OBJTYPE_ROOT);
        assert_eq!(root.child, -1, "No children for empty file");
        assert_eq!(root.stream_size, 0);
    }

    #[test]
    fn test_single_small_stream() {
        let streams = vec![OleStream {
            name: "TestStream".to_string(),
            data: vec![0x41, 0x42, 0x43, 0x44], // "ABCD"
        }];
        let data = build_ole_file(&streams);
        let h = parse_header(&data);

        assert_eq!(h.major_version, 3);

        // Root entry should have a child
        let root = parse_dir_entry(&data, &h, 0);
        assert_ne!(root.child, -1, "Root should have child");
        assert_eq!(root.stream_size, 64, "Mini stream container: 4 bytes padded to 64");

        // Find the stream entry (index 1)
        let entry = parse_dir_entry(&data, &h, 1);
        assert_eq!(entry.name, "TestStream");
        assert_eq!(entry.obj_type, OBJTYPE_STREAM);
        assert_eq!(entry.stream_size, 4);

        // Read back via mini-stream
        let root_entry = parse_dir_entry(&data, &h, 0);
        let recovered = read_mini_stream_data(
            &data, &h, entry.start_sector, entry.stream_size as usize,
            root_entry.start_sector, root_entry.stream_size,
        );
        assert_eq!(recovered, vec![0x41, 0x42, 0x43, 0x44]);
    }

    #[test]
    fn test_multiple_streams() {
        let streams = vec![
            OleStream { name: "Alpha".to_string(), data: vec![1, 2, 3] },
            OleStream { name: "Beta".to_string(), data: vec![4, 5] },
            OleStream { name: "Gamma".to_string(), data: vec![6] },
        ];
        let data = build_ole_file(&streams);
        let h = parse_header(&data);

        // Should have 4 directory entries (root + 3 streams)
        // All are small → mini stream
        let root = parse_dir_entry(&data, &h, 0);
        assert_ne!(root.child, -1, "Root should have children");
        // Mini stream: 3 padded to 64 + 2 padded to 64 + 1 padded to 64 = 192
        assert_eq!(root.stream_size, 192, "Mini stream should be 192 bytes (3×64 padded)");

        // Verify each stream's data
        let root_entry = parse_dir_entry(&data, &h, 0);
        for i in 1..=3 {
            let entry = parse_dir_entry(&data, &h, i);
            assert_eq!(entry.obj_type, OBJTYPE_STREAM);
            let recovered = read_mini_stream_data(
                &data, &h, entry.start_sector, entry.stream_size as usize,
                root_entry.start_sector, root_entry.stream_size,
            );
            match entry.name.as_str() {
                "Alpha" => assert_eq!(recovered, vec![1, 2, 3]),
                "Beta"  => assert_eq!(recovered, vec![4, 5]),
                "Gamma" => assert_eq!(recovered, vec![6]),
                other   => panic!("Unexpected stream: {}", other),
            }
        }
    }

    #[test]
    fn test_large_stream() {
        // Create a stream >= 4096 bytes to force regular sectors
        let large_data: Vec<u8> = (0..8192).map(|i| (i % 256) as u8).collect();
        let streams = vec![OleStream {
            name: "LargeStream".to_string(),
            data: large_data.clone(),
        }];
        let data = build_ole_file(&streams);
        let h = parse_header(&data);

        let entry = parse_dir_entry(&data, &h, 1);
        assert_eq!(entry.name, "LargeStream");
        assert_eq!(entry.stream_size, 8192);

        // Read back via sector chain (not mini-stream)
        let recovered = read_stream_data(&data, &h, entry.start_sector, entry.stream_size as usize);
        assert_eq!(recovered, large_data);
    }

    #[test]
    fn test_mixed_streams() {
        let small_data = vec![0xAA; 100]; // < 4096 → mini stream
        let large_data = vec![0xBB; 5000]; // >= 4096 → regular sectors
        let streams = vec![
            OleStream { name: "Small".to_string(), data: small_data.clone() },
            OleStream { name: "Large".to_string(), data: large_data.clone() },
        ];
        let data = build_ole_file(&streams);
        let h = parse_header(&data);

        let root_entry = parse_dir_entry(&data, &h, 0);

        for i in 1..=2 {
            let entry = parse_dir_entry(&data, &h, i);
            match entry.name.as_str() {
                "Small" => {
                    assert_eq!(entry.stream_size, 100);
                    let recovered = read_mini_stream_data(
                        &data, &h, entry.start_sector, entry.stream_size as usize,
                        root_entry.start_sector, root_entry.stream_size,
                    );
                    assert_eq!(recovered, small_data);
                }
                "Large" => {
                    assert_eq!(entry.stream_size, 5000);
                    let recovered = read_stream_data(
                        &data, &h, entry.start_sector, entry.stream_size as usize,
                    );
                    assert_eq!(recovered, large_data);
                }
                other => panic!("Unexpected: {}", other),
            }
        }
    }

    #[test]
    fn test_fat_chain_integrity() {
        // Must be >= 4096 bytes (mini stream cutoff) to use regular sectors.
        // 3 regular sectors = 3 × 512 = 1536 bytes, but that's below cutoff.
        // Use 8 sectors (4096 bytes) to test regular sector chain.
        let large_data = vec![0xCC; 4096]; // 8 sectors (8 × 512)
        let streams = vec![OleStream {
            name: "LargeStream".to_string(),
            data: large_data,
        }];
        let data = build_ole_file(&streams);
        let h = parse_header(&data);

        let entry = parse_dir_entry(&data, &h, 1);
        let chain = follow_chain(&data, &h, entry.start_sector);
        assert_eq!(chain.len(), 8, "Should span 8 sectors");

        // Verify chain is sequential (our writer uses contiguous sectors)
        for i in 0..chain.len() - 1 {
            assert_eq!(chain[i + 1], chain[i] + 1, "Chain should be contiguous");
        }

        // Last entry should be ENDOFCHAIN
        let last_fat = read_fat_entry(&data, &h, *chain.last().unwrap());
        assert_eq!(last_fat, ENDOFCHAIN);
    }

    #[test]
    fn test_fat_sector_marked_as_fatsect() {
        let data = build_ole_file(&[]);
        let h = parse_header(&data);

        // The first FAT sector should be marked as FATSECT in the FAT
        let fat_self = read_fat_entry(&data, &h, h.difat_entries[0]);
        assert_eq!(fat_self, FATSECT, "FAT sector must reference itself as FATSECT");
    }

    #[test]
    fn test_directory_sector_marked_as_endofchain() {
        let data = build_ole_file(&[]);
        let h = parse_header(&data);

        let dir_fat = read_fat_entry(&data, &h, h.first_dir_sector);
        assert_eq!(dir_fat, ENDOFCHAIN, "Directory sector chain should end with ENDOFCHAIN");
    }

    #[test]
    fn test_msi_clsid_on_root() {
        let data = build_ole_file(&[]);
        let h = parse_header(&data);
        let base = HEADER_SIZE + h.first_dir_sector as usize * SECTOR_SIZE;

        // CLSID is at offset 80 in the root directory entry.
        // Must be set to the Windows Installer CLSID so msiexec recognizes the package.
        let clsid = &data[base + 80..base + 96];
        assert_eq!(clsid, &MSI_CLSID, "Root entry CLSID must be the MSI CLSID");
    }

    #[test]
    fn test_file_size_is_sector_aligned() {
        let data = build_ole_file(&[
            OleStream { name: "A".to_string(), data: vec![1; 50] },
        ]);
        // File = header + sectors, total is always a multiple of sector size
        assert!(data.len() >= HEADER_SIZE);
        assert_eq!(data.len() % SECTOR_SIZE, 0, "File size must be a multiple of sector size");
    }

    #[test]
    fn test_stream_name_encoding() {
        // Verify UTF-16LE encoding in directory entry
        let streams = vec![OleStream {
            name: "AB".to_string(),
            data: vec![0x01],
        }];
        let data = build_ole_file(&streams);
        let h = parse_header(&data);
        let base = HEADER_SIZE + h.first_dir_sector as usize * SECTOR_SIZE + DIR_ENTRY_SIZE;

        // Name "AB" in UTF-16LE: 0x41 0x00 0x42 0x00
        assert_eq!(data[base], 0x41);
        assert_eq!(data[base + 1], 0x00);
        assert_eq!(data[base + 2], 0x42);
        assert_eq!(data[base + 3], 0x00);
    }

    #[test]
    fn test_empty_stream() {
        let streams = vec![OleStream {
            name: "Empty".to_string(),
            data: vec![],
        }];
        let data = build_ole_file(&streams);
        let h = parse_header(&data);

        let entry = parse_dir_entry(&data, &h, 1);
        assert_eq!(entry.name, "Empty");
        assert_eq!(entry.stream_size, 0);
    }

    #[test]
    fn test_large_file_with_difat() {
        // Test with a stream that requires DIFAT sectors (>109 FAT sectors).
        // 109 FAT sectors * 128 entries * 512 bytes = ~6.83 MB
        // So we need >6.83 MB to trigger DIFAT. Use 8 MB to be safe.
        let large_data = vec![0xAB; 8 * 1024 * 1024]; // 8 MB
        let streams = vec![OleStream {
            name: "LargeFile".to_string(),
            data: large_data.clone(),
        }];
        let data = build_ole_file(&streams);
        let h = parse_header(&data);

        // Verify header
        assert_eq!(h.major_version, 3);
        assert!(h.num_fat_sectors > DIFAT_IN_HEADER as u32, "Should need >109 FAT sectors");
        assert!(h.num_fat_sectors >= 128, "8MB needs at least 128 FAT sectors");

        // Verify DIFAT is present
        let difat_count = read_u32(&data, 68); // DIFAT count at offset 68
        assert!(difat_count > 0, "Should have DIFAT sectors");

        let first_difat = read_u32(&data, 64); // First DIFAT sector at offset 64
        assert_ne!(first_difat, FREE_SECT, "First DIFAT sector should be set");

        // Verify the stream can be read back
        let entry = parse_dir_entry(&data, &h, 1);
        assert_eq!(entry.name, "LargeFile");
        assert_eq!(entry.stream_size, 8 * 1024 * 1024);

        let recovered = read_stream_data(&data, &h, entry.start_sector, entry.stream_size as usize);
        assert_eq!(recovered.len(), large_data.len());
        assert_eq!(recovered, large_data);
    }

    #[test]
    fn test_very_large_file_50mb() {
        // Test with 50 MB to verify DIFAT works for large payloads
        let large_data = vec![0xCD; 50 * 1024 * 1024]; // 50 MB
        let streams = vec![OleStream {
            name: "VeryLargeFile".to_string(),
            data: large_data.clone(),
        }];
        let data = build_ole_file(&streams);
        let h = parse_header(&data);

        // Verify header
        assert_eq!(h.major_version, 3);
        assert!(h.num_fat_sectors > DIFAT_IN_HEADER as u32, "50MB needs DIFAT");

        // Verify DIFAT
        let difat_count = read_u32(&data, 68);
        assert!(difat_count > 0, "Should have DIFAT sectors");

        // Verify the stream
        let entry = parse_dir_entry(&data, &h, 1);
        assert_eq!(entry.name, "VeryLargeFile");
        assert_eq!(entry.stream_size, 50 * 1024 * 1024);

        let recovered = read_stream_data(&data, &h, entry.start_sector, entry.stream_size as usize);
        assert_eq!(recovered.len(), large_data.len());
        assert_eq!(recovered, large_data);
    }
}
