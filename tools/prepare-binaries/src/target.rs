#[derive(Debug, Clone)]
pub struct Target {
    pub target: &'static str,
    pub platform: &'static str,
    pub binary_name: &'static str,
    pub package_dir: &'static str,
}

pub const TARGETS: &[Target] = &[
    Target {
        target: "x86_64-unknown-linux-gnu",
        platform: "linux-x64",
        binary_name: "rari",
        package_dir: "packages/rari-linux-x64",
    },
    Target {
        target: "aarch64-unknown-linux-gnu",
        platform: "linux-arm64",
        binary_name: "rari",
        package_dir: "packages/rari-linux-arm64",
    },
    Target {
        target: "x86_64-apple-darwin",
        platform: "darwin-x64",
        binary_name: "rari",
        package_dir: "packages/rari-darwin-x64",
    },
    Target {
        target: "aarch64-apple-darwin",
        platform: "darwin-arm64",
        binary_name: "rari",
        package_dir: "packages/rari-darwin-arm64",
    },
    Target {
        target: "x86_64-pc-windows-msvc",
        platform: "win32-x64",
        binary_name: "rari.exe",
        package_dir: "packages/rari-win32-x64",
    },
];

pub fn get_current_platform_target() -> Option<&'static Target> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    TARGETS.iter().find(|target| {
        let parts: Vec<&str> = target.platform.split('-').collect();
        if parts.len() != 2 {
            return false;
        }
        let (target_os, target_arch) = (parts[0], parts[1]);

        let os_match = match os {
            "macos" => target_os == "darwin",
            "linux" => target_os == "linux",
            "windows" => target_os == "win32",
            _ => false,
        };

        let arch_match = match arch {
            "x86_64" => target_arch == "x64",
            "aarch64" => target_arch == "arm64",
            _ => false,
        };

        os_match && arch_match
    })
}
