# Releasing

Releases are built by [`.github/workflows/release.yml`](../.github/workflows/release.yml) and consumed by the in-app updater ([`services/updater.rs`](../rust/crates/cupcraft-app/src/services/updater.rs)).

## Publish a new version

1. Bump `version` in `rust/Cargo.toml` (for example `0.4.7`).
2. Commit and push.
3. Create and push the tag:
   ```sh
   git tag v0.4.7
   git push origin v0.4.7
   ```

GitHub Actions builds Windows, macOS (Apple Silicon and Intel) and Linux, creates the Windows installer, and publishes the release.

> [!IMPORTANT]
> The tag must match the version in `rust/Cargo.toml`. The workflow fails otherwise; if a build reported a lower version than the release, the updater would offer the same update forever.

## Release assets

| File | Used for |
|---|---|
| `cupcraft-setup-windows-x86_64.exe` | Windows installer (per-user, no admin) |
| `cupcraft-windows-x86_64.exe`, `cupcraft-linux-x86_64`, `cupcraft-macos-aarch64`, `cupcraft-macos-x86_64` | Raw executables downloaded by the updater |
| `cupcraft-*.tar.gz` | First manual install on macOS / Linux (keeps the executable bit) |
| `version.json` | Update manifest |

## How the updater works

No GitHub API and no token are involved. The app reads public files through the stable URL:

```
https://github.com/Aitor2010aitor/cupcraft/releases/latest/download/version.json
```

```json
{
  "version": "0.4.7",
  "notes": "",
  "assets": {
    "windows-x86_64": { "file": "cupcraft-windows-x86_64.exe", "sha256": "…" },
    "linux-x86_64":   { "file": "cupcraft-linux-x86_64",       "sha256": "…" },
    "macos-aarch64":  { "file": "cupcraft-macos-aarch64",      "sha256": "…" },
    "macos-x86_64":   { "file": "cupcraft-macos-x86_64",       "sha256": "…" }
  }
}
```

The platform key is `<OS>-<ARCH>` (Rust's `std::env::consts`). If a newer version exists, the app downloads the matching file, verifies its SHA-256, renames the running executable to `cupcraft.exe.old`, moves the new one into place, saves the open project and restarts. The `.old` file is removed on the next start.

> [!NOTE]
> The repository must be public. Installing under a protected folder such as `Program Files` would prevent the replacement; the installer uses `%LOCALAPPDATA%\Programs\cupcraft` for that reason.
