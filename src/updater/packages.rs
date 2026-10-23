//! Native package discovery for installations managed by the operating system.
use std::path::{Path, PathBuf};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Portable,
    Setup,
    Deb,
    Rpm,
    Pkg,
}
impl Kind {
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Portable => ".zip",
            Self::Setup => "-setup.exe",
            Self::Deb => ".deb",
            Self::Rpm => ".rpm",
            Self::Pkg => ".pkg",
        }
    }
    pub fn staged_name(self) -> &'static str {
        match self {
            Self::Portable => "package.zip",
            Self::Setup => "installer.exe",
            Self::Deb => "installer.deb",
            Self::Rpm => "installer.rpm",
            Self::Pkg => "installer.pkg",
        }
    }
    pub fn valid_header(self, bytes: &[u8]) -> bool {
        match self {
            Self::Portable => bytes.starts_with(b"PK"),
            Self::Setup => bytes.starts_with(b"MZ"),
            Self::Deb => bytes.starts_with(b"!<arch>\n"),
            Self::Rpm => bytes.starts_with(&[0xed, 0xab, 0xee, 0xdb]),
            Self::Pkg => bytes.starts_with(b"xar!"),
        }
    }
}
pub(super) fn installed() -> Kind {
    let Ok(executable) = std::env::current_exe() else {
        return Kind::Portable;
    };
    if cfg!(target_os = "windows")
        && executable
            .parent()
            .is_some_and(|p| p.join("unins000.exe").is_file())
    {
        return Kind::Setup;
    }
    if cfg!(target_os = "linux") && executable == Path::new("/usr/bin/Ada3D") {
        for (program, args, kind) in [
            (
                "/usr/bin/dpkg-query",
                vec!["-S", "/usr/bin/Ada3D"],
                Kind::Deb,
            ),
            ("/usr/bin/rpm", vec!["-qf", "/usr/bin/Ada3D"], Kind::Rpm),
        ] {
            if std::process::Command::new(program)
                .args(args)
                .output()
                .is_ok_and(|o| o.status.success())
            {
                return kind;
            }
        }
    }
    if cfg!(target_os = "macos")
        && executable.starts_with("/Applications/Ada3D.app")
        && std::process::Command::new("/usr/sbin/pkgutil")
            .args(["--pkg-info", "studio.r3f.Ada3D"])
            .output()
            .is_ok_and(|o| o.status.success())
    {
        return Kind::Pkg;
    }
    Kind::Portable
}
pub(super) fn staged(stage: &Path) -> Option<(Kind, PathBuf)> {
    [Kind::Setup, Kind::Deb, Kind::Rpm, Kind::Pkg]
        .into_iter()
        .map(|kind| (kind, stage.join(kind.staged_name())))
        .find(|(_, path)| path.is_file())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_names_and_headers_match_native_packages() {
        for (kind, suffix, header) in [
            (Kind::Setup, "-setup.exe", &b"MZ"[..]),
            (Kind::Deb, ".deb", &b"!<arch>\n"[..]),
            (Kind::Rpm, ".rpm", &[0xed, 0xab, 0xee, 0xdb][..]),
            (Kind::Pkg, ".pkg", &b"xar!"[..]),
        ] {
            assert_eq!(kind.suffix(), suffix);
            assert!(kind.valid_header(header));
            assert!(!kind.valid_header(b"<html>error</html>"));
        }
    }
}
