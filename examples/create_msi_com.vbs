' Create minimal MSI via Windows Installer COM API
Option Explicit
Dim installer, database, fso, si, f, v
Dim productCode, upgradeCode, fileSize, msiPath, testFilePath

productCode = "{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}"
upgradeCode = "{B2C3D4E5-F6A7-8901-BCDE-F12345678901}"
Set fso = CreateObject("Scripting.FileSystemObject")
msiPath = "c:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\com_minimal.msi"
testFilePath = "c:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\velo_test.txt"

If fso.FileExists(msiPath) Then fso.DeleteFile msiPath, True
Set f = fso.CreateTextFile(testFilePath, True)
f.WriteLine "Hello from Velocity Installer!"
f.Close
fileSize = fso.GetFile(testFilePath).Size

Set installer = CreateObject("WindowsInstaller.Installer")
Set database = installer.OpenDatabase(msiPath, 3)
WScript.Echo "Database created"

' Use direct Execute method (works in CreateDirect mode)
' But first we need to commit after creating tables in transact mode
Sub SQL(db, q)
    Dim vw
    Set vw = db.OpenView(q)
    vw.Execute
    vw.Close
End Sub

' Create tables - use backtick chr(96) for MSI identifiers
Dim bt
bt = Chr(96) ' backtick

SQL database, "CREATE TABLE " & bt & "Property" & bt & " (" & bt & "Property" & bt & " CHAR(72) NOT NULL, " & bt & "Value" & bt & " TEXT(255) LOCALIZABLE NULL PRIMARY KEY " & bt & "Property" & bt & ")"
WScript.Echo "Property table created"

SQL database, "CREATE TABLE " & bt & "Directory" & bt & " (" & bt & "Directory" & bt & " CHAR(72) NOT NULL, " & bt & "Directory_Parent" & bt & " CHAR(72) NULL, " & bt & "DefaultDir" & bt & " CHAR(255) NOT NULL LOCALIZABLE PRIMARY KEY " & bt & "Directory" & bt & ")"
SQL database, "CREATE TABLE " & bt & "Component" & bt & " (" & bt & "Component" & bt & " CHAR(72) NOT NULL, " & bt & "ComponentId" & bt & " CHAR(38) NULL, " & bt & "Directory_" & bt & " CHAR(72) NOT NULL, " & bt & "Attributes" & bt & " SHORT NOT NULL, " & bt & "Condition" & bt & " TEXT(255) NULL, " & bt & "KeyPath" & bt & " CHAR(72) NULL PRIMARY KEY " & bt & "Component" & bt & ")"
SQL database, "CREATE TABLE " & bt & "File" & bt & " (" & bt & "File" & bt & " CHAR(72) NOT NULL, " & bt & "Component_" & bt & " CHAR(72) NOT NULL, " & bt & "FileName" & bt & " CHAR(255) NOT NULL LOCALIZABLE, " & bt & "FileSize" & bt & " LONG NOT NULL, " & bt & "Version" & bt & " CHAR(72) NULL, " & bt & "Language" & bt & " CHAR(20) NULL, " & bt & "Attributes" & bt & " SHORT NULL PRIMARY KEY " & bt & "File" & bt & ")"
SQL database, "CREATE TABLE " & bt & "Feature" & bt & " (" & bt & "Feature" & bt & " CHAR(38) NOT NULL, " & bt & "Feature_Parent" & bt & " CHAR(38) NULL, " & bt & "Title" & bt & " CHAR(64) NULL LOCALIZABLE, " & bt & "Description" & bt & " TEXT(255) NULL LOCALIZABLE, " & bt & "Display" & bt & " SHORT NULL, " & bt & "Level" & bt & " SHORT NOT NULL, " & bt & "Directory_" & bt & " CHAR(72) NULL, " & bt & "Attributes" & bt & " SHORT NULL PRIMARY KEY " & bt & "Feature" & bt & ")"
SQL database, "CREATE TABLE " & bt & "FeatureComponents" & bt & " (" & bt & "Feature_" & bt & " CHAR(38) NOT NULL, " & bt & "Component_" & bt & " CHAR(72) NOT NULL PRIMARY KEY " & bt & "Feature_" & bt & ", " & bt & "Component_" & bt & ")"
SQL database, "CREATE TABLE " & bt & "Media" & bt & " (" & bt & "DiskId" & bt & " SHORT NOT NULL, " & bt & "LastSequence" & bt & " CHAR(20) NOT NULL, " & bt & "Cabinet" & bt & " CHAR(255) NULL LOCALIZABLE, " & bt & "VolumeLabel" & bt & " CHAR(32) NULL LOCALIZABLE, " & bt & "Source" & bt & " CHAR(72) NULL PRIMARY KEY " & bt & "DiskId" & bt & ")"
SQL database, "CREATE TABLE " & bt & "InstallExecuteSequence" & bt & " (" & bt & "Action" & bt & " CHAR(72) NOT NULL, " & bt & "Condition" & bt & " TEXT(255) NULL, " & bt & "Sequence" & bt & " SHORT NULL PRIMARY KEY " & bt & "Action" & bt & ")"
SQL database, "CREATE TABLE " & bt & "InstallUISequence" & bt & " (" & bt & "Action" & bt & " CHAR(72) NOT NULL, " & bt & "Condition" & bt & " TEXT(255) NULL, " & bt & "Sequence" & bt & " SHORT NULL PRIMARY KEY " & bt & "Action" & bt & ")"
WScript.Echo "All tables created"

' Insert data
SQL database, "INSERT INTO " & bt & "Property" & bt & " VALUES ('ProductName', 'Velocity Test App')"
SQL database, "INSERT INTO " & bt & "Property" & bt & " VALUES ('ProductCode', '" & productCode & "')"
SQL database, "INSERT INTO " & bt & "Property" & bt & " VALUES ('ProductVersion', '1.0.0')"
SQL database, "INSERT INTO " & bt & "Property" & bt & " VALUES ('Manufacturer', 'Velocity')"
SQL database, "INSERT INTO " & bt & "Property" & bt & " VALUES ('ProductLanguage', '1033')"
SQL database, "INSERT INTO " & bt & "Property" & bt & " VALUES ('UpgradeCode', '" & upgradeCode & "')"

SQL database, "INSERT INTO " & bt & "Directory" & bt & " VALUES ('TARGETDIR', NULL, '.')"
SQL database, "INSERT INTO " & bt & "Directory" & bt & " VALUES ('ProgramFilesFolder', 'TARGETDIR', 'PROGRA~1|Program Files')"
SQL database, "INSERT INTO " & bt & "Directory" & bt & " VALUES ('INSTALLDIR', 'ProgramFilesFolder', 'VELOCI~1|Velocity Test')"

SQL database, "INSERT INTO " & bt & "Component" & bt & " VALUES ('MainComp', '{12345678-1234-1234-1234-123456789012}', 'INSTALLDIR', 0, NULL, 'MainComp')"
SQL database, "INSERT INTO " & bt & "File" & bt & " VALUES ('velo_test.txt', 'MainComp', 'VELO_T~1.TXT|velo_test.txt', " & fileSize & ", NULL, NULL, 0)"
SQL database, "INSERT INTO " & bt & "Feature" & bt & " VALUES ('MainFeature', NULL, 'Complete', 'Install all files', 2, 1, 'INSTALLDIR', 0)"
SQL database, "INSERT INTO " & bt & "FeatureComponents" & bt & " VALUES ('MainFeature', 'MainComp')"
SQL database, "INSERT INTO " & bt & "Media" & bt & " VALUES (1, '1', '#velo.cab', NULL, NULL)"

' Sequences
SQL database, "INSERT INTO " & bt & "InstallExecuteSequence" & bt & " VALUES ('LaunchConditions', 'NOT Installed', 100)"
SQL database, "INSERT INTO " & bt & "InstallExecuteSequence" & bt & " VALUES ('ValidateProductID', NULL, 700)"
SQL database, "INSERT INTO " & bt & "InstallExecuteSequence" & bt & " VALUES ('CostInitialize', NULL, 800)"
SQL database, "INSERT INTO " & bt & "InstallExecuteSequence" & bt & " VALUES ('FileCost', NULL, 900)"
SQL database, "INSERT INTO " & bt & "InstallExecuteSequence" & bt & " VALUES ('CostFinalize', NULL, 1000)"
SQL database, "INSERT INTO " & bt & "InstallExecuteSequence" & bt & " VALUES ('InstallValidate', NULL, 1400)"
SQL database, "INSERT INTO " & bt & "InstallExecuteSequence" & bt & " VALUES ('InstallInitialize', NULL, 1500)"
SQL database, "INSERT INTO " & bt & "InstallExecuteSequence" & bt & " VALUES ('ProcessComponents', NULL, 1600)"
SQL database, "INSERT INTO " & bt & "InstallExecuteSequence" & bt & " VALUES ('UnpublishComponents', NULL, 1700)"
SQL database, "INSERT INTO " & bt & "InstallExecuteSequence" & bt & " VALUES ('UnpublishFeatures', NULL, 1800)"
SQL database, "INSERT INTO " & bt & "InstallExecuteSequence" & bt & " VALUES ('RemoveFiles', NULL, 3500)"
SQL database, "INSERT INTO " & bt & "InstallExecuteSequence" & bt & " VALUES ('InstallFiles', NULL, 4000)"
SQL database, "INSERT INTO " & bt & "InstallExecuteSequence" & bt & " VALUES ('PublishComponents', NULL, 6200)"
SQL database, "INSERT INTO " & bt & "InstallExecuteSequence" & bt & " VALUES ('PublishFeatures', NULL, 6300)"
SQL database, "INSERT INTO " & bt & "InstallExecuteSequence" & bt & " VALUES ('RegisterProduct', NULL, 6100)"
SQL database, "INSERT INTO " & bt & "InstallExecuteSequence" & bt & " VALUES ('InstallFinalize', NULL, 6600)"

SQL database, "INSERT INTO " & bt & "InstallUISequence" & bt & " VALUES ('LaunchConditions', 'NOT Installed', 100)"
SQL database, "INSERT INTO " & bt & "InstallUISequence" & bt & " VALUES ('ValidateProductID', NULL, 700)"
SQL database, "INSERT INTO " & bt & "InstallUISequence" & bt & " VALUES ('CostInitialize', NULL, 800)"
SQL database, "INSERT INTO " & bt & "InstallUISequence" & bt & " VALUES ('FileCost', NULL, 900)"
SQL database, "INSERT INTO " & bt & "InstallUISequence" & bt & " VALUES ('CostFinalize', NULL, 1000)"
SQL database, "INSERT INTO " & bt & "InstallUISequence" & bt & " VALUES ('ExecuteAction', NULL, 1300)"
WScript.Echo "Data inserted"

' SummaryInfo
Set si = database.SummaryInformation(0)
si.Property(1) = "Velocity Test App"
si.Property(2) = "Velocity"
si.Property(3) = "Velocity"
si.Property(4) = "installer"
si.Property(7) = ";1033"
si.Property(9) = productCode
si.Property(14) = 2
si.Property(15) = 400
si.Property(18) = "Velocity"
si.Persist
database.Commit
WScript.Echo "MSI size: " & fso.GetFile(msiPath).Size & " bytes"
WScript.Echo "MSI_PATH=" & msiPath
