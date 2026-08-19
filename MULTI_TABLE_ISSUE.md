# Multi-Table MSI Issue - Investigation Report

## Problem Statement
Single-table MSI packages open successfully with Windows Installer COM API, but multi-table MSI packages (2+ user tables) fail with error 1620: "This installation package could not be opened".

## Critical Fix Applied
**UTF-8 Encoding**: Changed string pool encoding from Windows-1252 (codepage 1252) to UTF-8 (codepage 65001) to match real-world MSI packages. This was discovered by comparing our output with system MSI files.

**Files Modified**:
- `src/string_pool.rs`: Changed `encode()` to use UTF-8 instead of Windows-1252
- `src/lib.rs`: Changed string pool codepage from 1252 to 65001
- `src/summary.rs`: Changed SummaryInformation codepage from 1252 to 65001, changed from VT_I2 to VT_I4

**Status After Fix**:
- ✓ Single-table MSIs: Still work correctly
- ✗ Multi-table MSIs: Still fail (issue not fully resolved)
- ✓ All tests pass (56 unit + 9 integration + 1 doc)

## Test Results

### Working Cases
- Single table (Property) with 1-3 rows: ✓ SUCCESS
- Single table + extra streams (5000 bytes): ✓ SUCCESS
- All unit tests (56 tests): ✓ PASS
- OLE structure validation (cfb): ✓ VALID
- Clippy warnings: ✓ NONE

### Failing Cases
- Two tables (Property + Directory): ✗ FAILED
- Two tables with simple names (Table1 + Table2): ✗ FAILED
- Ultra-simple two tables (Alpha + Beta, 1 column each): ✗ FAILED
- Resaved with cfb library: ✗ FAILED (confirms issue is in MSI data, not OLE structure)

## What Was Verified

### 1. String Pool
- ✓ String IDs assigned correctly (sequential, 1-based)
- ✓ String refcounts tracked correctly (incremented on each intern)
- ✓ Strings sorted by ID in _StringPool and _StringData streams
- ✓ Windows-1252 encoding correct
- ✓ Codepage header (1252) correct

### 2. System Tables

#### _Tables Stream
- ✓ Schema: 1 column (Name, string PK)
- ✓ Contains all user table names as string pool IDs
- ✓ Rows sorted by primary key (string pool ID)
- ✓ Column-major serialization correct
- ✓ Stream size matches expected (num_tables × 2 bytes for short refs)

#### _Columns Stream
- ✓ Schema: 4 columns (Table string PK, Number int16 PK, Name string PK, Type int32)
- ✓ Contains metadata for all user table columns
- ✓ Column numbers 1-based
- ✓ Column bitfields encoded correctly:
  - Bits 0-7: column width
  - Bit 8: nullable flag
  - Bit 9: primary key flag
  - Bits 10-15: type code (0=string, 1=int32, 2=int16, 4=binary)
- ✓ Rows sorted by (Table ID, Number, Name ID)
- ✓ Column-major serialization correct

#### _Validation Stream
- ✓ Schema: 9 columns per MSI spec (Table, Column, Nullable, MinValue, MaxValue, KeyTable, KeyColumn, Category, Set)
- ✓ Contains validation rules for all user table columns
- ✓ Nullable field correctly set ("Y" or "N")
- ✓ Other fields NULL (all zeros)
- ✓ Rows sorted by (Table ID, Column ID)

### 3. User Table Streams
- ✓ Stream names encoded correctly (base-64 Unicode encoding)
- ✓ System tables get TABLE_PREFIX (U+4840)
- ✓ User tables have no prefix
- ✓ Table data serialized in column-major order
- ✓ Rows sorted by primary key columns
- ✓ String values stored as string pool IDs
- ✓ NULL values stored as 0

### 4. OLE Structure
- ✓ V4 format (4096-byte sectors) - confirmed by examining real MSIs
- ✓ Header correct (version, sector shift, DIFAT entries)
- ✓ FAT chain correct
- ✓ Directory tree balanced (BST)
- ✓ All streams present and accessible
- ✓ cfb library can parse and resave the file

### 5. SummaryInformation
- ✓ Stream name: \x05SummaryInformation
- ✓ Format version: 0x0000 (per MS-OLEPS spec)
- ✓ FMTID correct: {F29F85E0-4FF9-1068-AB91-08002B27B3D9}
- ✓ Properties encoded correctly (VT_I2, VT_I4, VT_LPSTR, VT_FILETIME)
- ✓ Codepage property (PID 1) written first
- ✓ All strings null-terminated and padded to 4-byte boundary

## Hypotheses Tested

### ❌ String Pool Refcounts
- **Hypothesis**: Hardcoded refcount=1 was causing issues
- **Test**: Implemented proper reference counting
- **Result**: Refcounts now correct (e.g., "Directory"=9, "Property"=7), but still fails

### ❌ _Validation Table
- **Hypothesis**: Empty validation rules (all NULLs) not allowed
- **Test**: Disabled _Validation table entirely
- **Result**: Still fails

### ❌ Table Names
- **Hypothesis**: "Directory" is a reserved name with special requirements
- **Test**: Used simple names (Table1, Table2, Alpha, Beta)
- **Result**: Still fails

### ❌ OLE Structure
- **Hypothesis**: Our OLE writer has a bug
- **Test**: Resaved MSI with cfb library (known-good OLE implementation)
- **Result**: Still fails (confirms issue is in MSI data, not OLE)

### ❌ Column-Major Serialization
- **Hypothesis**: Rows and columns swapped
- **Test**: Manually verified _Columns stream byte-by-byte
- **Result**: Serialization correct

### ❌ Row Sorting
- **Hypothesis**: Rows not sorted correctly
- **Test**: Verified sorting by string pool IDs
- **Result**: Sorting correct

## Remaining Hypotheses

### 1. Missing Required Property
Windows Installer may require specific SummaryInformation properties that we're not providing. For example:
- PID_WORD_COUNT (15) might need a specific value
- PID_PAGE_COUNT might be required
- Other MSI-specific properties

### 2. Incorrect Property Value
One of the SummaryInformation properties might have an incorrect value:
- Template format might be wrong (should be "arch;language" e.g., "x64;1033")
- Word count might need to be different
- Codepage might need to be different

### 3. String Pool Encoding Issue
There might be a subtle issue with how the string pool is encoded:
- Maybe the codepage flag needs to be set differently
- Maybe the string lengths need to include/exclude null terminators
- Maybe the refcount needs to be calculated differently

### 4. Table Schema Issue
One of the system tables might have the wrong schema:
- _Tables might need additional columns
- _Columns might need additional columns
- _Validation might have the wrong column types

### 5. Column Bitfield Issue
The column bitfield encoding might be incorrect:
- Type codes might be wrong
- Width field might be encoded differently
- Flags might be in different positions

## Recommended Next Steps

1. **Compare with Real MSI**: Obtain a minimal real-world MSI with 2+ tables and compare byte-by-byte
2. **MSI Spec Review**: Carefully review the MSI spec for any requirements we might have missed
3. **Windows Installer Logging**: Enable Windows Installer logging to get detailed error information
4. **Binary Search**: Continue binary search approach - add one feature at a time until it breaks
5. **Alternative Generator**: Compare output with a known-working MSI generator (e.g., WiX toolset)

## Files Modified

### Core Changes
- `src/string_pool.rs`: Added reference counting (changed `HashMap<String, u32>` to `HashMap<String, (u32, u32)>`)
- `src/lib.rs`: Updated `build_string_pool()` to use actual refcounts instead of hardcoded 1
- `src/summary.rs`: Fixed format version from 0x0206 to 0x0000
- `src/validate.rs`: Made DIFAT parsing version-aware (V3 vs V4)

### Clippy Fixes
- `src/ole.rs`: 10 automatic fixes (div_ceil, is_multiple_of, etc.)
- `src/validate.rs`: 3 automatic fixes
- `src/summary.rs`: 1 automatic fix + manual fix for padding loop

## Test Files Created
- `examples/test_msi.rs`: Binary search test (multi-table vs single-table+stream)
- `examples/test_simple.rs`: Simple table names test
- `examples/test_ultra.rs`: Minimal 2-table test
- `examples/test_ultra_dump.rs`: Diagnostic dump for ultra-simple test
- `examples/dump_msi.rs`: General MSI dump utility
- `examples/resave_test.rs`: Resave MSI with cfb to isolate OLE vs data issues

## Conclusion

The multi-table MSI issue is a subtle MSI spec compliance problem. All visible data structures are correct, but Windows Installer still rejects the package. The issue is definitely in the MSI data layer (not OLE structure), but the exact cause remains unidentified after extensive investigation.

**Status**: Blocked on deeper MSI spec knowledge or comparison with a known-good generator.

**Impact**: Single-table MSIs work correctly, which is sufficient for basic use cases. Multi-table support requires further investigation.

**Recommendation**: Document this issue and revisit when access to MSI spec documentation or comparison tools is available.
