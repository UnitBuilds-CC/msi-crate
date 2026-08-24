# Create a minimal installable MSI using Windows Installer COM API
$installer = New-Object -ComObject WindowsInstaller.Installer
$database = $installer.OpenDatabase("C:\temp\com_minimal.msi", 1)
Write-Host "Created database"

function Execute-SQL($db, $sql) {
    $view = $db.OpenView($sql)
    $view.Execute()
    $view.Close()
}

# Create tables
Execute-SQL $database "CREATE TABLE ``Property`` (``Property`` CHAR(72) NOT NULL, ``Value`` TEXT(255) LOCALIZABLE NULL PRIMARY KEY ``Property``)"
Execute-SQL $database "CREATE TABLE ``Directory`` (``Directory`` CHAR(72) NOT NULL, ``Directory_Parent`` CHAR(72) NULL, ``DefaultDir`` CHAR(255) NOT NULL LOCALIZABLE PRIMARY KEY ``Directory``)"
Execute-SQL $database "CREATE TABLE ``Component`` (``Component`` CHAR(72) NOT NULL, ``ComponentId`` CHAR(38) NULL, ``Directory_`` CHAR(72) NOT NULL, ``Attributes`` SHORT NOT NULL, ``Condition`` TEXT(255) NULL, ``KeyPath`` CHAR(72) NULL PRIMARY KEY ``Component``)"
Execute-SQL $database "CREATE TABLE ``File`` (``File`` CHAR(72) NOT NULL, ``Component_`` CHAR(72) NOT NULL, ``FileName`` CHAR(255) NOT NULL LOCALIZABLE, ``FileSize`` LONG NOT NULL, ``Version`` CHAR(72) NULL, ``Language`` CHAR(20) NULL, ``Attributes`` SHORT NULL PRIMARY KEY ``File``)"
Execute-SQL $database "CREATE TABLE ``Feature`` (``Feature`` CHAR(38) NOT NULL, ``Feature_Parent`` CHAR(38) NULL, ``Title`` CHAR(64) NULL LOCALIZABLE, ``Description`` TEXT(255) NULL LOCALIZABLE, ``Display`` SHORT NULL, ``Level`` SHORT NOT NULL, ``Directory_`` CHAR(72) NULL, ``Attributes`` SHORT NULL PRIMARY KEY ``Feature``)"
Execute-SQL $database "CREATE TABLE ``FeatureComponents`` (``Feature_`` CHAR(38) NOT NULL, ``Component_`` CHAR(72) NOT NULL PRIMARY KEY ``Feature_``, ``Component_``)"
Execute-SQL $database "CREATE TABLE ``Media`` (``DiskId`` SHORT NOT NULL, ``LastSequence`` CHAR(20) NOT NULL, ``Cabinet`` CHAR(255) NULL LOCALIZABLE, ``VolumeLabel`` CHAR(32) NULL LOCALIZABLE, ``Source`` CHAR(72) NULL PRIMARY KEY ``DiskId``)"
Execute-SQL $database "CREATE TABLE ``InstallExecuteSequence`` (``Action`` CHAR(72) NOT NULL, ``Condition`` TEXT(255) NULL, ``Sequence`` SHORT NULL PRIMARY KEY ``Action``)"
Execute-SQL $database "CREATE TABLE ``InstallUISequence`` (``Action`` CHAR(72) NOT NULL, ``Condition`` TEXT(255) NULL, ``Sequence`` SHORT NULL PRIMARY KEY ``Action``)"
Write-Host "Tables created"

$productCode = "{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}"
$upgradeCode = "{B2C3D4E5-F6A7-8901-BCDE-F12345678901}"

# Properties
Execute-SQL $database "INSERT INTO ``Property`` VALUES ('ProductName', 'Velocity Test App')"
Execute-SQL $database "INSERT INTO ``Property`` VALUES ('ProductCode', '$productCode')"
Execute-SQL $database "INSERT INTO ``Property`` VALUES ('ProductVersion', '1.0.0')"
Execute-SQL $database "INSERT INTO ``Property`` VALUES ('Manufacturer', 'Velocity')"
Execute-SQL $database "INSERT INTO ``Property`` VALUES ('ProductLanguage', '1033')"
Execute-SQL $database "INSERT INTO ``Property`` VALUES ('UpgradeCode', '$upgradeCode')"
Write-Host "Properties inserted"

# Directories
Execute-SQL $database "INSERT INTO ``Directory`` VALUES ('TARGETDIR', NULL, '.')"
Execute-SQL $database "INSERT INTO ``Directory`` VALUES ('ProgramFilesFolder', 'TARGETDIR', 'PROGRA~1|Program Files')"
Execute-SQL $database "INSERT INTO ``Directory`` VALUES ('INSTALLDIR', 'ProgramFilesFolder', 'VELOCI~1|Velocity Test')"
Write-Host "Directories inserted"

# Create test file
"Hello from Velocity Installer!" | Out-File -FilePath "C:\temp\velo_test.txt" -Encoding ASCII
$fileSize = (Get-Item "C:\temp\velo_test.txt").Length

# Component, File, Feature, etc.
Execute-SQL $database "INSERT INTO ``Component`` VALUES ('MainComp', '{12345678-1234-1234-1234-123456789012}', 'INSTALLDIR', 0, NULL, 'MainComp')"
Execute-SQL $database "INSERT INTO ``File`` VALUES ('velo_test.txt', 'MainComp', 'VELO_T~1.TXT|velo_test.txt', $fileSize, NULL, NULL, 0)"
Execute-SQL $database "INSERT INTO ``Feature`` VALUES ('MainFeature', NULL, 'Complete', 'Install all files', 2, 1, 'INSTALLDIR', 0)"
Execute-SQL $database "INSERT INTO ``FeatureComponents`` VALUES ('MainFeature', 'MainComp')"
Execute-SQL $database "INSERT INTO ``Media`` VALUES (1, '1', '#velo.cab', NULL, NULL)"
Write-Host "Data inserted"

# Execute sequences
Execute-SQL $database "INSERT INTO ``InstallExecuteSequence`` VALUES ('LaunchConditions', 'NOT Installed', 100)"
Execute-SQL $database "INSERT INTO ``InstallExecuteSequence`` VALUES ('ValidateProductID', NULL, 700)"
Execute-SQL $database "INSERT INTO ``InstallExecuteSequence`` VALUES ('CostInitialize', NULL, 800)"
Execute-SQL $database "INSERT INTO ``InstallExecuteSequence`` VALUES ('FileCost', NULL, 900)"
Execute-SQL $database "INSERT INTO ``InstallExecuteSequence`` VALUES ('CostFinalize', NULL, 1000)"
Execute-SQL $database "INSERT INTO ``InstallExecuteSequence`` VALUES ('InstallValidate', NULL, 1400)"
Execute-SQL $database "INSERT INTO ``InstallExecuteSequence`` VALUES ('InstallInitialize', NULL, 1500)"
Execute-SQL $database "INSERT INTO ``InstallExecuteSequence`` VALUES ('ProcessComponents', NULL, 1600)"
Execute-SQL $database "INSERT INTO ``InstallExecuteSequence`` VALUES ('UnpublishComponents', NULL, 1700)"
Execute-SQL $database "INSERT INTO ``InstallExecuteSequence`` VALUES ('UnpublishFeatures', NULL, 1800)"
Execute-SQL $database "INSERT INTO ``InstallExecuteSequence`` VALUES ('RemoveFiles', NULL, 3500)"
Execute-SQL $database "INSERT INTO ``InstallExecuteSequence`` VALUES ('InstallFiles', NULL, 4000)"
Execute-SQL $database "INSERT INTO ``InstallExecuteSequence`` VALUES ('PublishComponents', NULL, 6200)"
Execute-SQL $database "INSERT INTO ``InstallExecuteSequence`` VALUES ('PublishFeatures', NULL, 6300)"
Execute-SQL $database "INSERT INTO ``InstallExecuteSequence`` VALUES ('RegisterProduct', NULL, 6100)"
Execute-SQL $database "INSERT INTO ``InstallExecuteSequence`` VALUES ('InstallFinalize', NULL, 6600)"

Execute-SQL $database "INSERT INTO ``InstallUISequence`` VALUES ('LaunchConditions', 'NOT Installed', 100)"
Execute-SQL $database "INSERT INTO ``InstallUISequence`` VALUES ('ValidateProductID', NULL, 700)"
Execute-SQL $database "INSERT INTO ``InstallUISequence`` VALUES ('CostInitialize', NULL, 800)"
Execute-SQL $database "INSERT INTO ``InstallUISequence`` VALUES ('FileCost', NULL, 900)"
Execute-SQL $database "INSERT INTO ``InstallUISequence`` VALUES ('CostFinalize', NULL, 1000)"
Execute-SQL $database "INSERT INTO ``InstallUISequence`` VALUES ('ExecuteAction', NULL, 1300)"
Write-Host "Sequences inserted"

# SummaryInfo
$si = $database.SummaryInformation(0)
$si.Property(1) = "Velocity Test App"
$si.Property(2) = "Velocity"
$si.Property(3) = "Velocity"
$si.Property(4) = "installer"
$si.Property(7) = ";1033"
$si.Property(9) = $productCode
$si.Property(14) = 2
$si.Property(15) = 400
$si.Property(18) = "Velocity"
$si.Property(19) = ""
$si.Persist()
$database.Commit()
Write-Host "Committed. Size: $((Get-Item 'C:\temp\com_minimal.msi').Length) bytes"

# Create cabinet
$ddfLines = @()
$ddfLines += ".OPTION EXPLICIT"
$ddfLines += ".Set CabinetNameTemplate=velo.cab"
$ddfLines += ".Set DiskDirectory1=C:\temp"
$ddfLines += ".Set Cabinet=on"
$ddfLines += ".Set Compress=on"
$ddfLines += '"C:\temp\velo_test.txt"'
$ddfLines | Out-File -FilePath "C:\temp\velo.ddf" -Encoding ASCII
& makecab.exe /f "C:\temp\velo.ddf" 2>&1 | Out-Null

if (Test-Path "C:\temp\velo.cab") {
    Write-Host "Cabinet: $((Get-Item 'C:\temp\velo.cab').Length) bytes"
    $db2 = $installer.OpenDatabase("C:\temp\com_minimal.msi", 2)
    $cabBytes = [System.IO.File]::ReadAllBytes("C:\temp\velo.cab")
    $db2.AddStream("velo.cab", [System.Text.Encoding]::Default.GetString($cabBytes))
    $db2.Commit()
    Write-Host "Cabinet embedded. MSI: $((Get-Item 'C:\temp\com_minimal.msi').Length) bytes"
} else {
    Write-Host "ERROR: Cabinet not created"
}

# Test
Write-Host "`nTesting install..."
$null = Start-Process -FilePath "msiexec.exe" -ArgumentList "/x", $productCode, "/qn" -Wait -PassThru -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1
$proc = Start-Process -FilePath "msiexec.exe" -ArgumentList "/i", "C:\temp\com_minimal.msi", "/qn", "/l*v", "C:\temp\com_test.log" -Wait -PassThru
Write-Host "Exit code: $($proc.ExitCode)"

if ($proc.ExitCode -eq 0) {
    Write-Host "SUCCESS!"
    $found = Get-ChildItem -Path "C:\Program Files" -Recurse -Filter "velo_test.txt" -ErrorAction SilentlyContinue
    if ($found) { Write-Host "Installed: $($found.FullName)" }
    $proc2 = Start-Process -FilePath "msiexec.exe" -ArgumentList "/x", $productCode, "/qn" -Wait -PassThru
    Write-Host "Uninstall: $($proc2.ExitCode)"
} else {
    Write-Host "FAILED"
    if (Test-Path "C:\temp\com_test.log") { Get-Content "C:\temp\com_test.log" | Select-Object -Last 15 }
}
