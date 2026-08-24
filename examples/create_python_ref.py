"""Create a reference MSI using Python's msilib (low-level API).
Gives us a known-good MSI to compare with velocity-msi output."""
import msilib
import os
import subprocess
import uuid

MSI_PATH = "python_ref.msi"
LOG_PATH = "python_ref.log"

# Clean up
for f in [MSI_PATH, LOG_PATH]:
    if os.path.exists(f):
        os.remove(f)

# Generate GUIDs
product_code = "{" + str(uuid.uuid4()).upper() + "}"
upgrade_code = "{" + str(uuid.uuid4()).upper() + "}"
package_code = "{" + str(uuid.uuid4()).upper() + "}"

print(f"ProductCode: {product_code}")
print(f"PackageCode: {package_code}")

# Create database using MSIDBOPEN_CREATE (transacted mode)
db = msilib.OpenDatabase(MSI_PATH, msilib.MSIDBOPEN_CREATE)
print("Database created")

# Create Property table
view = db.OpenView("CREATE TABLE `Property` (`Property` CHAR(72) NOT NULL, `Value` CHAR(255) NULL)")
view.Execute(None)
view.Close()

# Add primary key
view = db.OpenView("ALTER TABLE `Property` ADD PRIMARY KEY `Property`")
view.Execute(None)
view.Close()

# Insert properties using parameterized queries
props = [
    ("ProductName", "Python Ref"),
    ("ProductVersion", "1.0.0"),
    ("Manufacturer", "V"),
    ("ProductCode", product_code),
    ("UpgradeCode", upgrade_code),
    ("ProductLanguage", "1033"),
]

for name, value in props:
    rec = msilib.CreateRecord(2)
    rec.SetString(1, name)
    rec.SetString(2, value)
    view = db.OpenView("INSERT INTO `Property` (`Property`, `Value`) VALUES (?, ?)")
    view.Execute(rec)
    view.Close()
print(f"Property table: {len(props)} rows")

# Create Directory table
view = db.OpenView("CREATE TABLE `Directory` (`Directory` CHAR(72) NOT NULL, `Directory_Parent` CHAR(72) NULL, `DefaultDir` CHAR(255) NOT NULL)")
view.Execute(None)
view.Close()

view = db.OpenView("ALTER TABLE `Directory` ADD PRIMARY KEY `Directory`")
view.Execute(None)
view.Close()

# TARGETDIR with no parent
rec = msilib.CreateRecord(3)
rec.SetString(1, "TARGETDIR")
# Leave Directory_Parent as NULL (don't set it)
rec.SetString(3, "SourceDir")
view = db.OpenView("INSERT INTO `Directory` (`Directory`, `Directory_Parent`, `DefaultDir`) VALUES (?, ?, ?)")
view.Execute(rec)
view.Close()
print("Directory table: 1 row (TARGETDIR)")

# Create InstallExecuteSequence table
view = db.OpenView("CREATE TABLE `InstallExecuteSequence` (`Action` CHAR(72) NOT NULL, `Condition` CHAR(255) NULL, `Sequence` SHORT NULL)")
view.Execute(None)
view.Close()

view = db.OpenView("ALTER TABLE `InstallExecuteSequence` ADD PRIMARY KEY `Action`")
view.Execute(None)
view.Close()

# Add CostInitialize and CostFinalize
for action, seq in [("CostInitialize", 800), ("CostFinalize", 1000)]:
    rec = msilib.CreateRecord(3)
    rec.SetString(1, action)
    # Condition = NULL
    rec.SetInt(3, seq)
    view = db.OpenView("INSERT INTO `InstallExecuteSequence` (`Action`, `Condition`, `Sequence`) VALUES (?, ?, ?)")
    view.Execute(rec)
    view.Close()
print("InstallExecuteSequence table: 2 rows")

# Set Summary Information
si = db.GetSummaryInformation(1)
si.SetProperty(msilib.PID_CODEPAGE, 1252)
si.SetProperty(msilib.PID_TITLE, "Python Ref")
si.SetProperty(msilib.PID_SUBJECT, "Python Ref")
si.SetProperty(msilib.PID_AUTHOR, "V")
si.SetProperty(msilib.PID_TEMPLATE, "Intel;1033")
si.SetProperty(msilib.PID_REVNUMBER, package_code)
si.SetProperty(msilib.PID_SECURITY, 405)
si.SetProperty(msilib.PID_WORDCOUNT, 2)
si.Persist()
si.Close()
print("SummaryInfo set")

# Commit
db.Commit()
db.Close()
print(f"\nMSI created: {os.path.getsize(MSI_PATH)} bytes")

# Test with msiexec
print("\n--- Testing with msiexec ---")
result = subprocess.run(
    ["msiexec.exe", "/i", MSI_PATH, "/qn", "/norestart", "/l*v", LOG_PATH],
    capture_output=True, text=True
)
print(f"msiexec exit code: {result.returncode}")

if result.returncode == 0:
    print("SUCCESS! Reference MSI installs correctly.")
    # Uninstall
    subprocess.run(
        ["msiexec.exe", "/x", MSI_PATH, "/qn", "/norestart"],
        capture_output=True
    )
else:
    print(f"FAILED with error {result.returncode}")
    if os.path.exists(LOG_PATH):
        with open(LOG_PATH, "r", errors="replace") as f:
            for line in f:
                if "Error" in line or "return value 3" in line or "2705" in line or "1620" in line:
                    print(f"  {line.strip()}")

print("\nDone.")
