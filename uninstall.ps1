# Notes App Uninstaller
$InstallDir = "$env:LOCALAPPDATA\Notes"
$ShortcutPath = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Notes.lnk"

if (Test-Path $ShortcutPath) { Remove-Item $ShortcutPath -Force; Write-Host "Shortcut removed" }
if (Test-Path $InstallDir) { Remove-Item $InstallDir -Recurse -Force; Write-Host "App removed" }
Write-Host "Notes uninstalled. Your vault data in AppData is preserved."
