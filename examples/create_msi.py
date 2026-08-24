"""Create a minimal installable MSI using Python msilib - direct API"""
import warnings
warnings.filterwarnings("ignore", category=DeprecationWarning)
import msilib
import os
import subprocess
import time

msi_path = r"c:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\com_minimal.msi"
test_file = r"c:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\velo_test.txt"

# Kill any running msiexec processes first
subprocess.run(["taskkill", "/F", "/IM", "msiexec.exe"], capture_output=True)
time.sleep(2)

# Create test file
with open(test_file, "w") as f:
    f.write("Hello from Velocity Installer!\n")
file_size = os.path.getsize(test_file)

if os.path.exists(msi_path):
    os.remove(msi_path)

product_code = "{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}"
upgrade_code = "{B2C3D4E5-F6A7-8901-BCDE-F12345678901}"

# Create database directly
print("Creating database...")
import sys
sys.stdout.flush()
try:
    db = msilib.OpenDatabase(msi_path, msilib.MSIDBOPEN_CREATEDIRECT)
except Exception as e:
    print(f"ERROR: {e}")
    import traceback; traceback.print_exc()
    sys.exit(1)
print(f"Database created! Type: {type(db)}")

# Create tables using SQL
db.Execute("CREATE TABLE `Property` (`Property` CHAR(72) NOT NULL, `Value` TEXT(255) LOCALIZABLE NULL PRIMARY KEY `Property`)")
db.Execute("CREATE TABLE `Directory` (`Directory` CHAR(72) NOT NULL, `Directory_Parent` CHAR(72) NULL, `DefaultDir` CHAR(255) NOT NULL LOCALIZABLE PRIMARY KEY `Directory`)")
db.Execute("CREATE TABLE `Component` (`Component` CHAR(72) NOT NULL, `ComponentId` CHAR(38) NULL, `Directory_` CHAR(72) NOT NULL, `Attributes` SHORT NOT NULL, `Condition` TEXT(255) NULL, `KeyPath` CHAR(72) NULL PRIMARY KEY `Component`)")
db.Execute("CREATE TABLE `File` (`File` CHAR(72) NOT NULL, `Component_` CHAR(72) NOT NULL, `FileName` CHAR(255) NOT NULL LOCALIZABLE, `FileSize` LONG NOT NULL, `Version` CHAR(72) NULL, `Language` CHAR(20) NULL, `Attributes` SHORT NULL PRIMARY KEY `File`)")
db.Execute("CREATE TABLE `Feature` (`Feature` CHAR(38) NOT NULL, `Feature_Parent` CHAR(38) NULL, `Title` CHAR(64) NULL LOCALIZABLE, `Description` TEXT(255) NULL LOCALIZABLE, `Display` SHORT NULL, `Level` SHORT NOT NULL, `Directory_` CHAR(72) NULL, `Attributes` SHORT NULL PRIMARY KEY `Feature`)")
db.Execute("CREATE TABLE `FeatureComponents` (`Feature_` CHAR(38) NOT NULL, `Component_` CHAR(72) NOT NULL PRIMARY KEY `Feature_`, `Component_`)")
db.Execute("CREATE TABLE `Media` (`DiskId` SHORT NOT NULL, `LastSequence` CHAR(20) NOT NULL, `Cabinet` CHAR(255) NULL LOCALIZABLE, `VolumeLabel` CHAR(32) NULL LOCALIZABLE, `Source` CHAR(72) NULL PRIMARY KEY `DiskId`)")
db.Execute("CREATE TABLE `InstallExecuteSequence` (`Action` CHAR(72) NOT NULL, `Condition` TEXT(255) NULL, `Sequence` SHORT NULL PRIMARY KEY `Action`)")
db.Execute("CREATE TABLE `InstallUISequence` (`Action` CHAR(72) NOT NULL, `Condition` TEXT(255) NULL, `Sequence` SHORT NULL PRIMARY KEY `Action`)")
print("Tables created")

# Insert data using records
def insert_row(db, table, values):
    """Insert a row using the MSI record API"""
    rec = msilib.CreateRecord(len(values))
    for i, val in enumerate(values):
        if val is None:
            rec.SetNull(i + 1)
        elif isinstance(val, int):
            rec.SetInteger(i + 1, val)
        else:
            rec.SetString(i + 1, str(val))
    v = db.OpenView(f"SELECT * FROM `{table}`")
    v.Modify(msilib.MSIMODIFY_INSERT, rec)
    v.Close()

# Properties
for name, value in [
    ("ProductName", "Velocity Test App"),
    ("ProductCode", product_code),
    ("ProductVersion", "1.0.0"),
    ("Manufacturer", "Velocity"),
    ("ProductLanguage", "1033"),
    ("UpgradeCode", upgrade_code),
]:
    insert_row(db, "Property", [name, value])

# Directories
insert_row(db, "Directory", ["TARGETDIR", None, "."])
insert_row(db, "Directory", ["ProgramFilesFolder", "TARGETDIR", "PROGRA~1|Program Files"])
insert_row(db, "Directory", ["INSTALLDIR", "ProgramFilesFolder", "VELOCI~1|Velocity Test"])

# Component
insert_row(db, "Component", ["MainComp", "{12345678-1234-1234-1234-123456789012}", "INSTALLDIR", 0, None, "MainComp"])

# File
insert_row(db, "File", ["velo_test.txt", "MainComp", "VELO_T~1.TXT|velo_test.txt", file_size, None, None, 0])

# Feature
insert_row(db, "Feature", ["MainFeature", None, "Complete", "Install all files", 2, 1, "INSTALLDIR", 0])

# FeatureComponents
insert_row(db, "FeatureComponents", ["MainFeature", "MainComp"])

# Media
insert_row(db, "Media", [1, "1", "#velo.cab", None, None])
print("Data inserted")

# Execute sequences
for action, cond, seq in [
    ("LaunchConditions", "NOT Installed", 100),
    ("ValidateProductID", None, 700),
    ("CostInitialize", None, 800),
    ("FileCost", None, 900),
    ("CostFinalize", None, 1000),
    ("InstallValidate", None, 1400),
    ("InstallInitialize", None, 1500),
    ("ProcessComponents", None, 1600),
    ("UnpublishComponents", None, 1700),
    ("UnpublishFeatures", None, 1800),
    ("RemoveFiles", None, 3500),
    ("InstallFiles", None, 4000),
    ("PublishComponents", None, 6200),
    ("PublishFeatures", None, 6300),
    ("RegisterProduct", None, 6100),
    ("InstallFinalize", None, 6600),
]:
    insert_row(db, "InstallExecuteSequence", [action, cond, seq])

for action, cond, seq in [
    ("LaunchConditions", "NOT Installed", 100),
    ("ValidateProductID", None, 700),
    ("CostInitialize", None, 800),
    ("FileCost", None, 900),
    ("CostFinalize", None, 1000),
    ("ExecuteAction", None, 1300),
]:
    insert_row(db, "InstallUISequence", [action, cond, seq])
print("Sequences inserted")

# SummaryInfo
si = db.GetSummaryInformation(0)
si.SetProperty(msilib.PID_TITLE, "Velocity Test App")
si.SetProperty(msilib.PID_SUBJECT, "Velocity")
si.SetProperty(msilib.PID_AUTHOR, "Velocity")
si.SetProperty(msilib.PID_KEYWORDS, "installer")
si.SetProperty(msilib.PID_TEMPLATE, ";1033")
si.SetProperty(msilib.PID_REVNUMBER, product_code)
si.SetProperty(msilib.PID_WORDCOUNT, 2)
si.SetProperty(msilib.PID_PAGECOUNT, 400)
si.SetProperty(msilib.PID_APPNAME, "Velocity")
si.Persist()

db.Commit()
print(f"MSI committed! Size: {os.path.getsize(msi_path)} bytes")

# Create cabinet
ddf_path = r"c:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\velo.ddf"
cab_path = r"c:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\velo.cab"
if os.path.exists(cab_path):
    os.remove(cab_path)

with open(ddf_path, "w") as f:
    f.write(".OPTION EXPLICIT\n")
    f.write(".Set CabinetNameTemplate=velo.cab\n")
    f.write(f".Set DiskDirectory1={os.path.dirname(msi_path)}\n")
    f.write(".Set Cabinet=on\n")
    f.write(".Set Compress=on\n")
    f.write(f'"{test_file}"\n')

subprocess.run(["makecab.exe", "/f", ddf_path], capture_output=True)
if os.path.exists(cab_path):
    print(f"Cabinet: {os.path.getsize(cab_path)} bytes")
    msilib.add_stream(db, "velo.cab", cab_path)
    db.Commit()
    print(f"MSI with cabinet: {os.path.getsize(msi_path)} bytes")
else:
    print("WARNING: Cabinet not created")

# Test
print("\nTesting install...")
subprocess.run(["msiexec.exe", "/x", product_code, "/qn"], capture_output=True)
time.sleep(1)

result = subprocess.run(
    ["msiexec.exe", "/i", msi_path, "/qn", "/l*v", r"c:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\msi_test.log"],
    capture_output=True, text=True
)
print(f"msiexec exit code: {result.returncode}")

if result.returncode == 0:
    print("SUCCESS!")
    for base in [r"C:\Program Files", r"C:\Program Files (x86)"]:
        for root, dirs, files in os.walk(base):
            if "velo_test.txt" in files:
                print(f"Found: {os.path.join(root, 'velo_test.txt')}")
    r2 = subprocess.run(["msiexec.exe", "/x", product_code, "/qn"], capture_output=True)
    print(f"Uninstall: {r2.returncode}")
else:
    print(f"FAILED: {result.returncode}")
    log_path = r"c:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\msi_test.log"
    if os.path.exists(log_path):
        with open(log_path) as f:
            lines = f.readlines()
        for line in lines[-15:]:
            print(line.rstrip())
