' Create a minimal MSI with Property + Directory + InstallExecuteSequence via COM
' Then save it so we can compare with our output

Set installer = CreateObject("WindowsInstaller.Installer")
Set db = installer.CreateDatabase("com_ref.msi", 1)

' Create Property table
db.Execute "CREATE TABLE `Property` (`Property` CHAR(72) NOT NULL, `Value` CHAR(255) NULL PRIMARY KEY `Property`)"

' Wait, the correct syntax for MSI SQL:
db.Execute "CREATE TABLE `Property` (`Property` CHAR(72) NOT NULL, `Value` CHAR(255) NULL)"
db.Execute "ALTER TABLE `Property` ADD PRIMARY KEY (`Property`)"

' Insert properties
Dim records
Set records = installer.CreateRecord(2)
records.StringData(1) = "ProductName"
records.StringData(2) = "COM Ref Test"
db.Execute "INSERT INTO `Property` (`Property`, `Value`) VALUES ('ProductName', 'COM Ref Test')", records

' Actually, let me use the simpler SQL syntax
db.Execute "INSERT INTO `Property` (`Property`, `Value`) VALUES ('ProductVersion', '1.0.0')"
db.Execute "INSERT INTO `Property` (`Property`, `Value`) VALUES ('Manufacturer', 'V')"
db.Execute "INSERT INTO `Property` (`Property`, `Value`) VALUES ('ProductCode', '{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}')"
db.Execute "INSERT INTO `Property` (`Property`, `Value`) VALUES ('UpgradeCode', '{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}')"
db.Execute "INSERT INTO `Property` (`Property`, `Value`) VALUES ('ProductLanguage', '1033')"

' Create Directory table
db.Execute "CREATE TABLE `Directory` (`Directory` CHAR(72) NOT NULL, `Directory_Parent` CHAR(72) NULL, `DefaultDir` CHAR(255) NOT NULL)"
db.Execute "ALTER TABLE `Directory` ADD PRIMARY KEY (`Directory`, `DefaultDir`)"
db.Execute "INSERT INTO `Directory` (`Directory`, `Directory_Parent`, `DefaultDir`) VALUES ('TARGETDIR', '', 'SourceDir')"

' Create InstallExecuteSequence table
db.Execute "CREATE TABLE `InstallExecuteSequence` (`Action` CHAR(72) NOT NULL, `Condition` CHAR(255) NULL, `Sequence` SHORT NULL)"
db.Execute "ALTER TABLE `InstallExecuteSequence` ADD PRIMARY KEY (`Action`)"
db.Execute "INSERT INTO `InstallExecuteSequence` (`Action`, `Condition`, `Sequence`) VALUES ('CostInitialize', NULL, 800)"
db.Execute "INSERT INTO `InstallExecuteSequence` (`Action`, `Condition`, `Sequence`) VALUES ('CostFinalize', NULL, 1000)"

' Set summary info properties
Set view = db.OpenView("SELECT * FROM _Tables")
view.Execute

' Commit the database
db.Commit

WScript.Echo "Created com_ref.msi successfully"
