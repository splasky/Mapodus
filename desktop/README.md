# Desktop App

The desktop app uses Tauri 2 and embeds the existing Axum backend inside the
same desktop process. It does not spawn a separate backend executable.

## Development

Install Tauri prerequisites for your OS, then run:

```bash
cargo install tauri-cli --version "^2" --locked
cd desktop/src-tauri
cargo tauri dev
```

The Tauri build command runs the existing Svelte frontend build first, then
starts the desktop app. The backend binds to a random local port on
`127.0.0.1` and the desktop window loads that local URL.

## Desktop Config

Non-sensitive desktop settings are stored in the OS config directory:

- Linux: `~/.config/gmap-to-umap/config.toml`
- macOS: `~/Library/Application Support/gmap-to-umap/config.toml`
- Windows: `%APPDATA%\gmap-to-umap\config.toml`

Example:

```toml
umap_default_url = "https://umap.openstreetmap.fr/en/"
umap_account = "optional-user-name"
locale = "en"
dev_mode = false
```

Do not store Google cookies, uMap passwords, Google Maps API keys, OAuth tokens,
or session cookies in this config file. The desktop app stores sensitive values
with the OS credential vault/keychain:

- macOS: Keychain
- Windows: Credential Manager
- Linux: Secret Service / libsecret-compatible keyring

## Release Artifacts

The release workflow builds these artifacts for GitHub Releases:

- macOS: `.dmg`
- Windows: `.msi`
- Linux: `.AppImage`
- Linux: `.tar.gz` binary archive
- GitHub-provided source code `.zip`
- GitHub-provided source code `.tar.gz`
