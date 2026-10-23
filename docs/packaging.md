# Create Ada3D packages

Run these commands from the repository root, on the operating system and CPU architecture you are packaging for. Install [Rust and the platform build tools](platforms.md) and Python 3.11 or newer first.

## Windows setup wizard

Use Windows x86-64 with Visual Studio C++ Build Tools, the Windows SDK, and [Inno Setup 6.7 or newer](https://jrsoftware.org/isdl.php).

In PowerShell:

```powershell
python -m pip install Pillow
cargo build --release --locked
python tools/collect-licenses.py
python -c "from tools import package, installers; installers.build(package.package())"
```

This creates the portable ZIP and a single setup EXE containing the app, wizard images, shortcuts, project associations and uninstaller.

The builder finds Inno Setup 6 in its default installation directory or `ISCC` on your PATH. For another location, set the compiler path before packaging:

```powershell
$env:ISCC = 'C:\Path\To\Inno Setup\ISCC.exe'
```

## macOS installer

Use a Mac with Xcode command-line tools (`xcode-select --install`). Run on Apple Silicon for an ARM64 package, or an Intel Mac for an Intel package.

```sh
cargo build --release --locked
python3 tools/collect-licenses.py
python3 -c "from tools import package, installers; installers.build(package.package())"
```

This creates a portable ZIP containing Ada3D.app and a PKG that installs it in Applications.

To sign the PKG and its app, import your Developer ID certificates into the build user's keychain and set their identities before running the packaging command:

```sh
export MACOS_APP_SIGN_IDENTITY="Developer ID Application: Your Name (TEAMID)"
export MACOS_INSTALLER_SIGN_IDENTITY="Developer ID Installer: Your Name (TEAMID)"
```

Without these identities, the packages are unsigned. Signing does not notarize the app; notarization must be completed separately before distributing a notarized release.

## Linux DEB and RPM

Use Debian or Ubuntu for the installer builder. It uses Debian's library metadata to build the DEB and also produces an RPM. Build on x86-64 for x86-64 packages, or ARM64 for ARM64 packages. Ubuntu 22.04 is the release build baseline.

```sh
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev dpkg-dev rpm
cargo build --release --locked
python3 tools/collect-licenses.py
python3 -c "from tools import package, installers; installers.build(package.package())"
```

This creates a portable ZIP, a DEB, and an RPM. The native packages include the desktop launcher, icon, software-center metadata and license notices. Open the DEB or RPM with the desktop's package installer to install it.

## Find the packages

Packages are written to `dist/`, with names derived from Cargo.toml and the build machine:

| Platform | Installer |
|---|---|
| Windows x86-64 | `Ada3D-v<version>-windows-x86_64-setup.exe` |
| macOS Intel or Apple Silicon | `Ada3D-v<version>-macos-<architecture>.pkg` |
| Linux x86-64 or ARM64 | `Ada3D-v<version>-linux-<architecture>.deb` and `.rpm` |

Each installer and portable ZIP has a matching `.sha256` file. Keep the ZIP's checksum beside it if you build an installer from an existing ZIP:

```sh
python tools/installers.py "path/to/Ada3D-release.zip"
```

Replace the path with the actual ZIP filename. Its version, operating system and architecture must match the source checkout and build machine. On macOS and Linux, use `python3` if that is your Python command.

## Build with GitHub Actions

1. Open the repository's **Actions** tab and choose **Package Ada3D**.
2. Select **Run workflow**, choose the branch, and start the run.
3. When it succeeds, download the platform you need from **Artifacts** on the run's page. Extract that download to get the installers, portable ZIP and checksums.

Pushes to `main` also build these packages. To publish a GitHub release, push a tag consisting of `v` followed by the version in Cargo.toml. The release workflow publishes the packages and checksums under the names used by the updater. App-store submission and Apple notarization are separate from this workflow.
