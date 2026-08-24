// Create a reference MSI using Windows Installer COM API via C#
using System;
using System.IO;
using System.Runtime.InteropServices;

class Program
{
    [ComImport]
    [Guid("000C1090-0000-0000-C000-000000000046")]
    [InterfaceType(ComInterfaceType.InterfaceIsIDispatch)]
    interface IInstaller
    {
        object OpenDatabase(string path, int mode);
        object CreateRecord(int count);
    }

    static void Main()
    {
        string msiPath = @"C:\temp\ref_cs.msi";
        if (File.Exists(msiPath)) File.Delete(msiPath);

        // Get the Windows Installer COM object
        Type installerType = Type.GetTypeFromProgID("WindowsInstaller.Installer");
        dynamic installer = Activator.CreateInstance(installerType);

        // Try to create database using different approaches
        Console.WriteLine("Attempting to create MSI via COM API...");

        // Approach 1: OpenDatabase with mode 6 (create direct)
        try
        {
            dynamic db = installer.OpenDatabase(msiPath, 6);
            Console.WriteLine("Created with mode 6");
            CreateMsiContent(db, installer);
            return;
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Mode 6 failed: {ex.Message}");
        }

        // Approach 2: Create empty file, then open with mode 2 (direct write)
        try
        {
            File.WriteAllBytes(msiPath, new byte[0]);
            dynamic db = installer.OpenDatabase(msiPath, 2);
            Console.WriteLine("Created with mode 2 on empty file");
            CreateMsiContent(db, installer);
            return;
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Mode 2 failed: {ex.Message}");
        }

        // Approach 3: Create empty file, then open with mode 1 (transacted)
        try
        {
            File.Delete(msiPath);
            File.WriteAllBytes(msiPath, new byte[0]);
            dynamic db = installer.OpenDatabase(msiPath, 1);
            Console.WriteLine("Created with mode 1 on empty file");
            CreateMsiContent(db, installer);
            return;
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Mode 1 failed: {ex.Message}");
        }

        Console.WriteLine("ALL approaches failed. Cannot create MSI via COM.");
    }

    static void CreateMsiContent(dynamic db, dynamic installer)
    {
        // Set SummaryInfo
        dynamic si = db.SummaryInformation(1);
        si.set_Property(1, "1252");       // Codepage
        si.set_Property(2, "Reference");  // Title
        si.set_Property(7, "x64;1033");   // Template
        si.set_Property(9, "{12345678-1234-1234-1234-123456789012}");
        si.set_Property(14, "405");       // Security
        si.set_Property(15, "2");         // WordCount
        si.set_Property(18, "C# Creator");
        si.Flush();
        Console.WriteLine("SummaryInfo set");

        // Create Property table
        db.Execute("CREATE TABLE `Property` (`Property` CHAR(72) NOT NULL, `Value` CHAR(255) NULL LOCALIZABLE PRIMARY KEY `Property`)");

        // Insert rows
        InsertProp(db, installer, "ProductName", "Reference MSI");
        InsertProp(db, installer, "ProductCode", "{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}");
        InsertProp(db, installer, "ProductVersion", "1.0.0");
        InsertProp(db, installer, "Manufacturer", "TestCorp");
        InsertProp(db, installer, "UpgradeCode", "{BBBBBBBB-CCCC-DDDD-EEEE-FFFFFFFFFFFF}");
        InsertProp(db, installer, "ProductLanguage", "1033");
        InsertProp(db, installer, "ALLUSERS", "1");
        Console.WriteLine("Property table: 7 rows");

        db.Commit();
        Console.WriteLine("Database committed");

        // Test with msiexec
        Console.WriteLine("\n=== Testing with msiexec ===");
        var psi = new System.Diagnostics.ProcessStartInfo("msiexec.exe",
            $"/i \"{msiPath}\" /qn /norestart /l*v C:\\temp\\ref_cs.log");
        psi.UseShellExecute = false;
        var proc = System.Diagnostics.Process.Start(psi);
        proc.WaitForExit();
        Console.WriteLine($"msiexec exit code: {proc.ExitCode}");

        if (proc.ExitCode == 0)
            Console.WriteLine("*** SUCCESS! ***");
        else
        {
            Console.WriteLine($"Failed with code {proc.ExitCode}");
            if (File.Exists(@"C:\temp\ref_cs.log"))
            {
                var lines = File.ReadAllLines(@"C:\temp\ref_cs.log");
                for (int i = Math.Max(0, lines.Length - 20); i < lines.Length; i++)
                    Console.WriteLine(lines[i]);
            }
        }
    }

    static void InsertProp(dynamic db, dynamic installer, string name, string value)
    {
        dynamic rec = installer.CreateRecord(2);
        rec.set_StringData(1, name);
        rec.set_StringData(2, value);
        db.Execute("INSERT INTO `Property` VALUES (?, ?)", rec);
    }
}
