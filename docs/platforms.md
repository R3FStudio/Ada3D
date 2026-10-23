# Build and package Ada3D

Requirements: Rust 1.88 or newer, a native C/C++ toolchain, and Python 3.11 or newer for packaging tools.

```sh
cargo build --release --locked
python tools/collect-licenses.py
python tools/package.py
```

## Windows

Install Visual Studio Build Tools with C++ support and the Windows SDK. Building the setup wizard also requires Inno Setup 6.7 or newer and Pillow (`python -m pip install Pillow`).

## macOS

Install Xcode command-line tools and Rust. Intel and Apple Silicon packages are separate. The app is unsigned unless you add a signing/notarization step to your distribution process.

## Linux

Use a compiler toolchain and the X11, Wayland, xkbcommon and OpenGL development packages.

- Debian/Ubuntu: `build-essential pkg-config libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev`
- Fedora: `gcc gcc-c++ make pkgconf-pkg-config libX11-devel libXi-devel libXcursor-devel libXrandr-devel libxkbcommon-devel wayland-devel mesa-libGL-devel`
- Arch: `base-devel libx11 libxi libxcursor libxrandr libxkbcommon wayland mesa`

Release builds use Ubuntu 22.04 as the Linux baseline. A working OpenGL driver is required. Install `xdg-desktop-portal` and the backend for your desktop to use native file dialogs. Window decoration and material support varies by compositor.

After extracting a Linux release, run `sh install.sh` to install Ada3D and its launcher for your user. A source checkout can use `sh tools/install-linux.sh` after building.

## GitHub releases

Push a tag containing `v` followed by the package version in `Cargo.toml`, to run **Package Ada3D**. It builds Windows x86-64, Linux x86-64/ARM64, and macOS Intel/Apple Silicon packages, then attaches the installers, portable ZIPs and their SHA-256 checksums to the release. Pushes to main and manual workflow runs produce downloadable Actions artifacts without publishing a release.

Updates use [R3FStudio/Ada3D](https://github.com/R3FStudio/Ada3D) by default. Release workflows set `ADA3D_UPDATE_REPOSITORY` from `github.repository`; forks can override it when building.

## Clean build output

```sh
python tools/clean-build.py --dry-run
python tools/clean-build.py
```

The cleaner removes `build/` and old `target/` output. Add `--dist` to remove release packages too.

## Native installers

Run `python tools/installers.py <release-zip>` on the platform that produced the ZIP. Windows builds a single setup EXE with shortcuts, project associations and an uninstaller. macOS builds a PKG for Applications. Linux builds DEB and RPM packages with desktop launchers and software-center metadata; install them with your desktop package installer, `apt install ./<file>.deb`, or `dnf install ./<file>.rpm`. These are direct downloads, not app-store listings.

Linux packaging requires `dpkg-dev` and `rpm`. macOS signing can use `MACOS_APP_SIGN_IDENTITY` and `MACOS_INSTALLER_SIGN_IDENTITY` after importing your signing certificates. Unsigned packages are not notarized.

The updater selects the matching installer for a package-managed installation and the ZIP for a portable copy. Packages are verified before installation. System-wide Linux and macOS updates request the operating system's administrator approval. Update checks run at startup or from About in Settings.
