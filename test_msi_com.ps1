$ErrorActionPreference = "Continue"
$msiPath = "C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\crates\velocity-msi\test_full.msi"

Write-Host "=== Testing MSI with Windows Installer COM API ==="
Write-Host "File: $msiPath"
Write-Host "Size: $((Get-Item $msiPath).Length) bytes"

try {
    $wi = New-Object -ComObject WindowsInstaller.Installer
    Write-Host "Windows Installer COM object created"
    
    # Open database - use OpenDatabase with proper types
    # 0 = msiOpenDatabaseModeReadOnly
    [int]$mode = 0
    $db = $wi.OpenDatabase($msiPath, $mode)
    Write-Host "Database opened successfully!"
    
    # Read SummaryInfo
    Write-Host "`n=== Summary Information ==="
    try {
        $si = $db.SummaryInformation(0)
        Write-Host "  Title: $($si.Property(2))"
        Write-Host "  Subject: $($si.Property(3))"
        Write-Host "  Author: $($si.Property(4))"
        Write-Host "  Template: $($si.Property(7))"
        Write-Host "  RevNumber: $($si.Property(9))"
        Write-Host "  WordCount: $($si.Property(15))"
        Write-Host "  PageCount: $($si.Property(14))"
        Write-Host "  CreatingApp: $($si.Property(18))"
        Write-Host "  LastSavedBy: $($si.Property(8))"
    } catch {
        Write-Host "  Error reading SummaryInfo: $($_.Exception.Message)"
    }
    
    # Read Property table
    Write-Host "`n=== Property Table ==="
    try {
        $view = $db.OpenView("SELECT Property, Value FROM Property ORDER BY Property")
        $view.Execute()
        while ($true) {
            $rec = $view.Fetch()
            if ($rec -eq $null) { break }
            $name = $rec.StringData(1)
            $val = $rec.StringData(2)
            Write-Host "  $name = $val"
        }
        $view.Close()
    } catch {
        Write-Host "  Error reading Property table: $($_.Exception.Message)"
    }
    
    # List all tables
    Write-Host "`n=== Tables in Database ==="
    try {
        $view2 = $db.OpenView("SELECT Name FROM _Tables ORDER BY Name")
        $view2.Execute()
        while ($true) {
            $rec = $view2.Fetch()
            if ($rec -eq $null) { break }
            $tname = $rec.StringData(1)
            Write-Host "  $tname"
        }
        $view2.Close()
    } catch {
        Write-Host "  Error listing tables: $($_.Exception.Message)"
    }

    # Try to read Directory table
    Write-Host "`n=== Directory Table ==="
    try {
        $view3 = $db.OpenView("SELECT Directory, Directory_Parent, DefaultDir FROM Directory ORDER BY Directory")
        $view3.Execute()
        while ($true) {
            $rec = $view3.Fetch()
            if ($rec -eq $null) { break }
            $d = $rec.StringData(1)
            $p = $rec.StringData(2)
            $dd = $rec.StringData(3)
            Write-Host "  Dir=$d Parent=$p DefaultDir=$dd"
        }
        $view3.Close()
    } catch {
        Write-Host "  Error reading Directory table: $($_.Exception.Message)"
    }

    # Read Component table
    Write-Host "`n=== Component Table ==="
    try {
        $view4 = $db.OpenView("SELECT * FROM Component")
        $view4.Execute()
        while ($true) {
            $rec = $view4.Fetch()
            if ($rec -eq $null) { break }
            Write-Host "  Component=$($rec.StringData(1)) ComponentId=$($rec.StringData(2)) Dir=$($rec.StringData(3)) Attr=$($rec.StringData(4)) Cond=$($rec.StringData(5)) KeyPath=$($rec.StringData(6))"
        }
        $view4.Close()
    } catch {
        Write-Host "  Error reading Component table: $($_.Exception.Message)"
    }

    # Read File table
    Write-Host "`n=== File Table ==="
    try {
        $view5 = $db.OpenView("SELECT * FROM File")
        $view5.Execute()
        while ($true) {
            $rec = $view5.Fetch()
            if ($rec -eq $null) { break }
            Write-Host "  File=$($rec.StringData(1)) Comp=$($rec.StringData(2)) Name=$($rec.StringData(3)) Size=$($rec.StringData(4)) Seq=$($rec.StringData(8))"
        }
        $view5.Close()
    } catch {
        Write-Host "  Error reading File table: $($_.Exception.Message)"
    }
    
    # Read _Columns table (system metadata)
    Write-Host "`n=== _Columns Table (first 10) ==="
    try {
        $view6 = $db.OpenView("SELECT * FROM _Columns ORDER BY Table, Number")
        $view6.Execute()
        $count = 0
        while ($true) {
            $rec = $view6.Fetch()
            if ($rec -eq $null) { break }
            $count++
            if ($count -le 10) {
                Write-Host "  Table=$($rec.StringData(1)) Num=$($rec.StringData(2)) Name=$($rec.StringData(3)) Type=$($rec.StringData(4))"
            }
        }
        Write-Host "  Total _Columns rows: $count"
        $view6.Close()
    } catch {
        Write-Host "  Error reading _Columns: $($_.Exception.Message)"
    }

    # Read InstallExecuteSequence
    Write-Host "`n=== InstallExecuteSequence ==="
    try {
        $view7 = $db.OpenView("SELECT Action, Condition, Sequence FROM InstallExecuteSequence ORDER BY Sequence")
        $view7.Execute()
        while ($true) {
            $rec = $view7.Fetch()
            if ($rec -eq $null) { break }
            Write-Host "  $($rec.StringData(1)) Condition=$($rec.StringData(2)) Seq=$($rec.StringData(3))"
        }
        $view7.Close()
    } catch {
        Write-Host "  Error reading InstallExecuteSequence: $($_.Exception.Message)"
    }

    Write-Host "`n=== DONE ==="
    
} catch {
    Write-Host "FATAL ERROR: $($_.Exception.Message)"
    Write-Host $_.ScriptStackTrace
}
