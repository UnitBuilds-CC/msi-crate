# Create a reference MSI using Windows Installer COM API
# This is the GOLD STANDARD - if this MSI doesn't install, nothing will

$ErrorActionPreference = "Continue"
$wi = New-Object -ComObject WindowsInstaller.Installer
$fso = New-Object -ComObject Scripting.FileSystemObject

# Clean up
if ($fso.FileExists("C:\temp\ref_com.msi")) { $fso.DeleteFile("C:\temp\ref_com.msi") }

# Create database - msiOpenDatabaseModeCreateDirect = 6
try {
    $db = $wi.OpenDatabase("C:\temp\ref_com.msi", 6)
    Write-Host "Database created with OpenDatabase mode 6"
} catch {
    Write-Host "OpenDatabase mode 6 failed: $_"
    # Try creating empty file first then opening
    New-Item -Path "C:\temp\ref_com.msi" -ItemType File -Force | Out-Null
    try {
        $db = $wi.OpenDatabase("C:\temp\ref_com.msi", 6)
        Write-Host "Database created after file creation"
    } catch {
        Write-Host "Still failed: $_"
        exit 1
    }
}

# Set Summary Information
$si = $db.SummaryInformation(1)
$si.Property(1) = "1252"
$si.Property(2) = "Reference MSI"
$si.Property(7) = "x64;1033"
$si.Property(9) = "{12345678-1234-1234-1234-123456789012}"
$si.Property(14) = "405"
$si.Property(15) = "2"
$si.Property(18) = "Reference Creator"
$si.Flush()
Write-Host "SummaryInfo set"

# Create Property table - use backtick for SQL identifier quoting
$sql = 'CREATE TABLE `Property` (`Property` CHAR(72) NOT NULL, `Value` CHAR(255) NULL LOCALIZABLE PRIMARY KEY `Property`)'
$db.Execute($sql)
Write-Host "Property table created"

# Insert properties
function Insert-Property($db, $wi, $name, $value) {
    $rec = $wi.CreateRecord(2)
    $rec.StringData(1) = $name
    $rec.StringData(2) = $value
    $sql = 'INSERT INTO `Property` VALUES (?, ?)'
    $db.Execute($sql, $rec)
}

Insert-Property $db $wi "ProductName" "Reference MSI"
Insert-Property $db $wi "ProductCode" "{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}"
Insert-Property $db $wi "ProductVersion" "1.0.0"
Insert-Property $db $wi "Manufacturer" "TestCorp"
Insert-Property $db $wi "UpgradeCode" "{BBBBBBBB-CCCC-DDDD-EEEE-FFFFFFFFFFFF}"
Insert-Property $db $wi "ProductLanguage" "1033"
Insert-Property $db $wi "ALLUSERS" "1"
Write-Host "Property table populated with 7 rows"

# Commit
$db.Commit()
Write-Host "Database committed"

# Verify by re-opening
$db2 = $wi.OpenDatabase("C:\temp\ref_com.msi", 0)
$si2 = $db2.SummaryInformation(0)
Write-Host "`nVerification:"
Write-Host "  Title: $($si2.Property(2))"
Write-Host "  Template: $($si2.Property(7))"
Write-Host "  Codepage: $($si2.Property(1))"

# Query Property table
$view = $db2.OpenView('SELECT * FROM Property')
$view.Execute()
Write-Host "`nProperty table contents:"
while ($true) {
    $rec = $view.Fetch()
    if ($rec -eq $null) { break }
    Write-Host "  $($rec.StringData(1)) = $($rec.StringData(2))"
}
$view.Close()

# Test with msiexec
Write-Host "`n=== Testing with msiexec ==="
$shell = New-Object -ComObject WScript.Shell
$ret = $shell.Run("msiexec /i C:\temp\ref_com.msi /qn /norestart /l*v C:\temp\ref_com.log", 0, $true)
Write-Host "msiexec exit code: $ret"

if ($ret -eq 0) {
    Write-Host "*** REFERENCE MSI INSTALLED SUCCESSFULLY! ***"
} else {
    Write-Host "Reference MSI failed with code $ret"
    if (Test-Path "C:\temp\ref_com.log") {
        Write-Host "`nLast 20 lines of log:"
        Get-Content "C:\temp\ref_com.log" -Tail 20
    }
}
