## What's new in v0.4.10

- **Linux installer (.deb)**: a package for Debian, Ubuntu and Linux Mint. Double-click it and cupcraft is installed system-wide, with its icon in the app menu.
- **Updates on Linux ask for permission**: when cupcraft is installed in `/usr/bin`, the in-app updater shows the system password prompt (`pkexec`) to replace the program. If you cancel, nothing changes.
- **Fix: update notice that never went away on Linux**: after updating, the app relaunched the old version, so the "new version available" card kept coming back. It now relaunches the new one.
