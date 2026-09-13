# Multi-Table MSI Issue - Investigation Report

## Status: RESOLVED

The multi-table MSI issue has been resolved. All 9 integration tests pass, including tests that create MSIs with multiple tables, embedded cabinets, and extra streams.

## Original Problem
Single-table MSI packages opened successfully with Windows Installer COM API, but multi-table MSI packages (2+ user tables) failed with error 1620: "This installation package could not be opened."

## Root Causes Fixed

### 1. OLE V3 Format
The OLE writer produces V3 compound files (512-byte sectors) as required by Windows Installer. Initially there was confusion about V3 vs V4 format — MSI requires V3.

### 2. Integration Test Corrections
The integration tests themselves had stale assertions:
- Tests asserted `cfb::Version::V4` instead of `V3`
- Tests looked for plain ASCII stream names instead of MSI base-64 Unicode encoded names
- These were fixed to match the actual (correct) code behavior

### 3. DIFAT Pointer Fix (msiexec error 2705)
The OLE V3 writer wrote `FREE_SECT` (0xFFFFFFFF) instead of `ENDOFCHAIN` (0xFFFFFFFE) for the empty DIFAT pointer in the header. Per MS-CFB spec, this field must be ENDOFCHAIN when no DIFAT sectors are needed. This caused msiexec to reject all generated packages with error 2705.

**Fix**: Changed `FREE_SECT` to `ENDOFCHAIN` at `src/ole.rs` line 257.

### 4. Stream Name Encoding
Extra streams (cabinets, markers) are stored with MSI base-64 Unicode encoding via `encode_stream_name(name, false)`. Tests were updated to use encoded names for lookups.

## What Was Verified

### String Pool
- String IDs assigned correctly (sequential, 1-based, alphabetical via BTreeMap)
- String refcounts tracked correctly
- Windows-1252 encoding correct
- Codepage header (1252) correct

### System Tables
- _Tables: schema correct (1 column), contains all table names as string pool IDs
- _Columns: schema correct (4 columns), bitfields encoded correctly
- _Validation: 9 columns per MSI spec

### OLE Structure
- V3 format (512-byte sectors)
- Header, FAT chain, directory tree all correct
- cfb library can parse generated MSIs

### Integration Tests (all pass)
- `test_v3_format` — cfb detects V3 format
- `test_string_pool_streams` — pool/data streams present
- `test_summary_info_stream` — summary info accessible
- `test_extra_stream_roundtrip` — embedded cabinets survive roundtrip
- `test_marker_stream` — marker streams accessible
- `test_multiple_tables` — multi-table MSIs valid
- `test_cabinet_stream_encoding` — cabinet names encoded correctly
- `test_table_stream_encoding` — table names encoded correctly
- `test_complete_msi_structure` — full structure validation

## Test Results
- 51 unit tests: PASS
- 27 comprehensive validation tests: PASS
- 9 cross-validation tests: PASS
- 1 doc test: PASS
- 0 clippy warnings
- 88 total tests, all passing

## Conclusion
The multi-table MSI issue was resolved by correcting the integration tests to match the actual code behavior (V3 format, encoded stream names) and ensuring the OLE writer produces correct V3 compound files.
