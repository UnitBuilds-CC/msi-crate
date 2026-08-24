' Create a reference MSI using Windows Installer COM API via VBScript
Option Explicit

Dim installer, db, view, rec, summary, fso
Dim msiPath, logPath

Set fso = CreateObject("Scripting.FileSystemObject")
Set installer = CreateObject("WindowsInstaller.Installer")

msiPath = Replace(WScript.ScriptFullName, WScript.ScriptName, "") & "vbs_ref.msi"
logPath = Replace(WScript.ScriptFullName, WScript.ScriptName, "") & "vbs_ref.log"

If fso.FileExists(msiPath) Then fso.DeleteFile msiPath
If fso.FileExists(logPath) Then fso.DeleteFile logPath

' Create database (mode 3 = msiOpenDatabaseModeCreate)
Set db = installer.OpenDatabase(msiPath, 3)
WScript.Echo "Database created"

' Property table - use unquoted identifiers
Set view = db.OpenView("CREATE TABLE Property (Property CHAR(72) NOT NULL, Value CHAR(255) NULL)")
view.Execute
view.Close

Set view = db.OpenView("ALTER TABLE Property ADD PRIMARY KEY (Property)")
view.Execute
view.Close

Set view = db.OpenView("INSERT INTO Property (Property, Value) VALUES ('ProductName', 'VBS Ref')")
view.Execute
view.Close

Set view = db.OpenView("INSERT INTO Property (Property, Value) VALUES ('ProductVersion', '1.0.0')")
view.Execute
view.Close

Set view = db.OpenView("INSERT INTO Property (Property, Value) VALUES ('Manufacturer', 'V')")
view.Execute
view.Close

Set view = db.OpenView("INSERT INTO Property (Property, Value) VALUES ('ProductCode', '{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}')")
view.Execute
view.Close

Set view = db.OpenView("INSERT INTO Property (Property, Value) VALUES ('UpgradeCode', '{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}')")
view.Execute
view.Close

Set view = db.OpenView("INSERT INTO Property (Property, Value) VALUES ('ProductLanguage', '1033')")
view.Execute
view.Close
WScript.Echo "Property table done"

' Directory table
Set view = db.OpenView("CREATE TABLE Directory (Directory CHAR(72) NOT NULL, Directory_Parent CHAR(72) NULL, DefaultDir CHAR(255) NOT NULL)")
view.Execute
view.Close

Set view = db.OpenView("ALTER TABLE Directory ADD PRIMARY KEY (Directory, DefaultDir)")
view.Execute
view.Close

Set rec = installer.CreateRecord(3)
rec.StringData(1) = "TARGETDIR"
rec.StringData(3) = "SourceDir"
Set view = db.OpenView("INSERT INTO Directory (Directory, Directory_Parent, DefaultDir) VALUES (?, ?, ?)")
view.Execute rec
view.Close
WScript.Echo "Directory table done"

' InstallExecuteSequence table
Set view = db.OpenView("CREATE TABLE InstallExecuteSequence (Action CHAR(72) NOT NULL, Condition CHAR(255) NULL, Sequence SHORT NULL)")
view.Execute
view.Close

Set view = db.OpenView("ALTER TABLE InstallExecuteSequence ADD PRIMARY KEY (Action)")
view.Execute
view.Close

Set rec = installer.CreateRecord(3)
rec.StringData(1) = "CostInitialize"
rec.IntegerData(3) = 800
Set view = db.OpenView("INSERT INTO InstallExecuteSequence (Action, Condition, Sequence) VALUES (?, ?, ?)")
view.Execute rec
view.Close

Set rec = installer.CreateRecord(3)
rec.StringData(1) = "CostFinalize"
rec.IntegerData(3) = 1000
Set view = db.OpenView("INSERT INTO InstallExecuteSequence (Action, Condition, Sequence) VALUES (?, ?, ?)")
view.Execute rec
view.Close
WScript.Echo "InstallExecuteSequence done"

' Summary Information
Set summary = db.SummaryInformation(1)
summary.Property(1) = 1252
summary.Property(2) = "VBS Ref"
summary.Property(4) = "V"
summary.Property(7) = "Intel;1033"
summary.Property(9) = "{12345678-1234-1234-1234-123456789012}"
summary.Property(14) = 405
summary.Property(15) = 2
summary.Persist
WScript.Echo "SummaryInfo done"

' Commit
db.Commit
WScript.Echo "Committed. Size: " & fso.GetFile(msiPath).Size & " bytes"

' Test with msiexec
Dim shell, proc
Set shell = CreateObject("WScript.Shell")
Set proc = shell.Exec("msiexec.exe /i """ & msiPath & """ /qn /norestart /l*v """ & logPath & """")
Do While proc.Status = 0
    WScript.Sleep 100
Loop

WScript.Echo ""
WScript.Echo "msiexec exit code: " & proc.ExitCode

If proc.ExitCode = 0 Then
    WScript.Echo "SUCCESS!"
    shell.Exec "msiexec.exe /x """ & msiPath & """ /qn /norestart"
Else
    WScript.Echo "FAILED"
End If
