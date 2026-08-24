# Create a reference MSI using Windows Installer COM API
$ErrorActionPreference = "Stop"

$msiPath = "$PSScriptRoot\com_ref.msi"
$logPath = "$PSScriptRoot\com_ref.log"
if (Test-Path $msiPath) { Remove-Item $msiPath -Force }
if (Test-Path $logPath) { Remove-Item $logPath -Force }

$installer = New-Object -ComObject WindowsInstaller.Installer

# msiOpenDatabaseModeCreate = 3
$db = $installer.OpenDatabase($msiPath, 3)
Write-Host "Database created"

# Helper: execute SQL with optional record
function Exec-SQL($sql, $rec) {
    $view = $db.OpenView($sql)
    if ($rec) {
        $view.Execute($rec)
    } else {
        $view.Execute()
    }
    $view.Close()
    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($view) | Out-Null
}

# Property table - use backtick-quoted identifiers (single-quoted strings preserve backticks)
Exec-SQL 'CREATE TABLE `Property` (`Property` CHAR(72) NOT NULL, `Value` CHAR(255) NULL)'
Exec-SQL 'ALTER TABLE `Property` ADD PRIMARY KEY (`Property`)'
Exec-SQL 'INSERT INTO `Property` (`Property`, `Value`) VALUES (''ProductName'', ''COM Ref Test'')'
Exec-SQL 'INSERT INTO `Property` (`Property`, `Value`) VALUES (''ProductVersion'', ''1.0.0'')'
Exec-SQL 'INSERT INTO `Property` (`Property`, `Value`) VALUES (''Manufacturer'', ''V'')'
Exec-SQL 'INSERT INTO `Property` (`Property`, `Value`) VALUES (''ProductCode'', ''{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}'')'
Exec-SQL 'INSERT INTO `Property` (`Property`, `Value`) VALUES (''UpgradeCode'', ''{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}'')'
Exec-SQL 'INSERT INTO `Property` (`Property`, `Value`) VALUES (''ProductLanguage'', ''1033'')'
Write-Host "Property table done"

# Directory table
Exec-SQL 'CREATE TABLE `Directory` (`Directory` CHAR(72) NOT NULL, `Directory_Parent` CHAR(72) NULL, `DefaultDir` CHAR(255) NOT NULL)'
Exec-SQL 'ALTER TABLE `Directory` ADD PRIMARY KEY (`Directory`, `DefaultDir`)'
$rec = $installer.CreateRecord(3)
$rec.StringData(1) = "TARGETDIR"
# StringData(2) not set = NULL parent
$rec.StringData(3) = "SourceDir"
Exec-SQL 'INSERT INTO `Directory` (`Directory`, `Directory_Parent`, `DefaultDir`) VALUES (?, ?, ?)' $rec
Write-Host "Directory table done"

# InstallExecuteSequence table
Exec-SQL 'CREATE TABLE `InstallExecuteSequence` (`Action` CHAR(72) NOT NULL, `Condition` CHAR(255) NULL, `Sequence` SHORT NULL)'
Exec-SQL 'ALTER TABLE `InstallExecuteSequence` ADD PRIMARY KEY (`Action`)'
$rec = $installer.CreateRecord(3)
$rec.StringData(1) = "CostInitialize"
# Condition = NULL (not set)
$rec.IntegerData(3) = 800
Exec-SQL 'INSERT INTO `InstallExecuteSequence` (`Action`, `Condition`, `Sequence`) VALUES (?, ?, ?)' $rec

$rec = $installer.CreateRecord(3)
$rec.StringData(1) = "CostFinalize"
$rec.IntegerData(3) = 1000
Exec-SQL 'INSERT INTO `InstallExecuteSequence` (`Action`, `Condition`, `Sequence`) VALUES (?, ?, ?)' $rec
Write-Host "InstallExecuteSequence table done"

# Summary Information
$summary = $db.SummaryInformation(1)
$summary.Property(1) = 1252    # Codepage
$summary.Property(2) = "COM Ref Test"  # Title
$summary.Property(4) = "V"     # Author
$summary.Property(7) = "Intel;1033"  # Template
$summary.Property(9) = "{12345678-1234-1234-1234-123456789012}"  # RevNumber
$summary.Property(14) = 405    # Security
$summary.Property(15) = 2      # WordCount
$summary.Persist()
Write-Host "SummaryInfo done"

# Commit
$db.Commit()
Write-Host "Committed. Size: $((Get-Item $msiPath).Length) bytes"

# Test with msiexec
Write-Host "`nTesting with msiexec..."
$proc = Start-Process -FilePath "msiexec.exe" -ArgumentList "/i `"$msiPath`" /qn /norestart /l*v `"$logPath`"" -Wait -PassThru
Write-Host "Exit code: $($proc.ExitCode)"

if ($proc.ExitCode -eq 0) {
    Write-Host "SUCCESS! COM reference MSI installs correctly."
    Start-Process -FilePath "msiexec.exe" -ArgumentList "/x `"$msiPath`" /qn /norestart" -Wait
} else {
    Write-Host "FAILED."
    if (Test-Path $logPath) {
        Get-Content $logPath | Select-String -Pattern "Error|2705|return value 3" | Select-Object -Last 10 | ForEach-Object { $_.Line.Trim() }
    }
}
