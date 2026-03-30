# Notes App Installer for Windows
# Run as Administrator: powershell -ExecutionPolicy Bypass -File install.ps1

$AppName = "Notes"
$ExeName = "notes-app.exe"
$InstallDir = "$env:LOCALAPPDATA\Notes"
$SourceExe = "target\release\$ExeName"
$StartMenuDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"
$ShortcutPath = "$StartMenuDir\$AppName.lnk"

Write-Host "Installing $AppName..." -ForegroundColor Green

# Build release
Write-Host "Building release binary..."
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed!" -ForegroundColor Red
    exit 1
}

# Create install directory
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

# Copy exe
Copy-Item $SourceExe "$InstallDir\$ExeName" -Force
Write-Host "Installed to $InstallDir\$ExeName"

# Copy icon
if (Test-Path "assets\logo.png") {
    Copy-Item "assets\logo.png" "$InstallDir\logo.png" -Force
}
if (Test-Path "assets\logo.ico") {
    Copy-Item "assets\logo.ico" "$InstallDir\logo.ico" -Force
}

# Create Start Menu shortcut
$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut($ShortcutPath)
$Shortcut.TargetPath = "$InstallDir\$ExeName"
$Shortcut.WorkingDirectory = $InstallDir
$Shortcut.Description = "Secure Encrypted Notes"
if (Test-Path "$InstallDir\logo.ico") {
    $Shortcut.IconLocation = "$InstallDir\logo.ico"
}
$Shortcut.Save()
Write-Host "Start Menu shortcut created at $ShortcutPath"

Write-Host ""
Write-Host "$AppName installed! You can find it by searching '$AppName' in the Start Menu." -ForegroundColor Green
