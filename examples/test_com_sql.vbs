' Test: create MSI, then reopen to add tables
Option Explicit

Dim fso, installer, database, view, rec, summary
Dim msiPath, logPath
msiPath = "C:\temp\com_ref.msi"
logPath = "C:\temp\com_ref.log"

Set fso = CreateObject("Scripting.FileSystemObject")
If fso.FileExists(msiPath) Then fso.DeleteFile msiPath, True
If fso.FileExists(logPath) Then fso.DeleteFile logPath, True

Set installer = CreateObject("WindowsInstaller.Installer")

' Step 1: Create empty database (mode 3 = create)
Set database = installer.OpenDatabase(msiPath, 3)
database.Commit
Set database = Nothing
WScript.Echo "Empty database created: " & fso.GetFile(msiPath).Size & " bytes"

' Step 2: Reopen in transact mode (mode 1) to add tables
Set database = installer.OpenDatabase(msiPath, 1)
WScript.Echo "Reopened in transact mode"

' Test CREATE TABLE
On Error Resume Next
Set view = database.OpenView("CREATE TABLE Property (Property CHAR(72) NOT NULL PRIMARY KEY, Value CHAR(255))")
If Err.Number <> 0 Then
    WScript.Echo "CREATE TABLE failed: " & Err.Description & " (err=" & Err.Number & ")"
    Err.Clear
Else
    view.Execute
    view.Close
    WScript.Echo "Property table created OK"
    
    ' Insert a row
    Set view = database.OpenView("INSERT INTO Property VALUES ('ProductName', 'Test')")
    If Err.Number <> 0 Then
        WScript.Echo "INSERT failed: " & Err.Description
        Err.Clear
    Else
        view.Execute
        view.Close
        WScript.Echo "Row inserted OK"
    End If
End If
On Error GoTo 0

database.Commit
WScript.Echo "Committed. Size: " & fso.GetFile(msiPath).Size & " bytes"
