' Create reference MSI with Windows Installer COM API
' This creates a known-good MSI with a File table for comparison

Set wshShell = CreateObject("WScript.Shell")

' Create temp directory
wshShell.Run "cmd /c mkdir C:\temp\vbs_test 2>nul", 0, True

' Delete existing file if present
Dim fso
Set fso = CreateObject("Scripting.FileSystemObject")
If fso.FileExists("C:\temp\vbs_test\reference.msi") Then fso.DeleteFile("C:\temp\vbs_test\reference.msi")

' Create Windows Installer object
Set installer = CreateObject("WindowsInstaller.Installer")

' Create a new database using OpenDatabase with msiOpenDatabaseModeCreate (1)
Const msiOpenDatabaseModeCreate = 1
Set database = installer.OpenDatabase("C:\temp\vbs_test\reference.msi", msiOpenDatabaseModeCreate)

' Create Property table
database.CreateTable "Property", "Property S72 PRIMARY KEY, Value S255"

' Insert properties
Dim view
Set view = database.OpenView("INSERT INTO Property (Property, Value) VALUES ('ProductName', 'RefTest')")
view.Execute
Set view = database.OpenView("INSERT INTO Property (Property, Value) VALUES ('ProductCode', '{12345678-1234-1234-1234-123456789012}')")
view.Execute
Set view = database.OpenView("INSERT INTO Property (Property, Value) VALUES ('ProductVersion', '1.0.0')")
view.Execute
Set view = database.OpenView("INSERT INTO Property (Property, Value) VALUES ('ProductLanguage', '1033')")
view.Execute
Set view = database.OpenView("INSERT INTO Property (Property, Value) VALUES ('Manufacturer', 'RefCo')")
view.Execute
Set view = database.OpenView("INSERT INTO Property (Property, Value) VALUES ('UpgradeCode', '{87654321-4321-4321-4321-210987654321}')")
view.Execute

' Create File table - this is the key table we need to compare
database.CreateTable "File", "File S72 PRIMARY KEY, Component_ S72, FileName S255, FileSize I4, Attributes I2 NULLABLE, Sequence I4"

' Insert a file row
Set view = database.OpenView("INSERT INTO File (File, Component_, FileName, FileSize, Attributes, Sequence) VALUES ('F1', 'MainComp', 'test.txt', 13, 0, 1)")
view.Execute

' Commit the database
database.Commit

WScript.Echo "Reference MSI created successfully"
