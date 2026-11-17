//! `kubelint self update`: runs the latest release's installer again, into the folder where
//! the installer put this kubelint.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output, Stdio};
use std::{env, fs};

use anyhow::{Context as _, bail};

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

#[derive(Debug, clap::Subcommand)]
pub enum SelfCommand {
    /// Update kubelint to the latest release.
    Update,
}

pub fn run(command: &SelfCommand) -> anyhow::Result<ExitCode> {
    match command {
        SelfCommand::Update => update(),
    }
}

fn update() -> anyhow::Result<ExitCode> {
    let receipt = Receipt::load()?;
    let exe = env::current_exe().context("cannot find this kubelint")?;
    let installed = fs::canonicalize(&exe).is_ok_and(|exe| receipt.is_for(&exe));
    if !installed {
        bail!(
            "this kubelint ({}) was not installed by the installer, which put kubelint in {}. \
             Update this one with the tool you installed it with",
            exe.display(),
            receipt.bin_dir.display()
        );
    }
    let current =
        Version::parse(env!("CARGO_PKG_VERSION")).context("this kubelint has no version")?;
    let latest = latest_version()?;
    if latest <= current {
        println!("kubelint v{current} is already the latest version");
    } else {
        install(&receipt, &exe, latest)?;
        println!("updated kubelint from v{current} to v{latest}");
    }
    Ok(ExitCode::SUCCESS)
}

/// A release version, `major.minor.patch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Version([u64; 3]);

impl Version {
    fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('.').map(|part| part.parse().ok());
        let version = Self([parts.next()??, parts.next()??, parts.next()??]);
        parts.next().is_none().then_some(version)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [major, minor, patch] = self.0;
        write!(f, "{major}.{minor}.{patch}")
    }
}

/// The version of the latest release, read from the manifest dist publishes with it.
fn latest_version() -> anyhow::Result<Version> {
    let manifest = download(&format!(
        "{REPOSITORY}/releases/latest/download/dist-manifest.json"
    ))?;
    manifest_version(&manifest).context("cannot find the version of the latest release")
}

fn manifest_version(manifest: &[u8]) -> Option<Version> {
    let manifest: serde_json::Value = serde_json::from_slice(manifest).ok()?;
    Version::parse(manifest["announcement_tag"].as_str()?.strip_prefix('v')?)
}

fn release_file(version: Version, name: &str) -> String {
    format!("{REPOSITORY}/releases/download/v{version}/{name}")
}

#[derive(Debug, PartialEq)]
struct Receipt {
    /// The folder the installer was told to install into.
    prefix: PathBuf,
    /// `prefix`, or `prefix/bin` when that folder was `CARGO_HOME`.
    bin_dir: PathBuf,
}

impl Receipt {
    fn load() -> anyhow::Result<Self> {
        let Some(path) = receipt_paths().find(|path| path.is_file()) else {
            bail!(
                "kubelint was not installed by its installer, so it cannot update itself. \
                 Update it with the tool you installed it with"
            );
        };
        let unreadable = || {
            format!(
                "cannot read the installer's receipt {}. Run the installer again to rewrite it",
                path.display()
            )
        };
        let text = fs::read_to_string(&path).with_context(unreadable)?;
        Self::parse(&text).with_context(unreadable)
    }

    fn parse(text: &str) -> Option<Self> {
        let receipt: serde_json::Value = serde_json::from_str(text).ok()?;
        let prefix = PathBuf::from(receipt["install_prefix"].as_str()?);
        let bin_dir = if receipt["install_layout"] == "cargo-home" {
            prefix.join("bin")
        } else {
            prefix.clone()
        };
        Some(Self { prefix, bin_dir })
    }

    /// Whether `exe`, a canonical path, is the binary the installer put in `bin_dir`.
    fn is_for(&self, exe: &Path) -> bool {
        fs::canonicalize(&self.bin_dir).is_ok_and(|dir| exe.parent() == Some(dir.as_path()))
    }
}

/// Where the installers write the receipt: `XDG_CONFIG_HOME` when set, else `LOCALAPPDATA`
/// on Windows and `~/.config` elsewhere.
fn receipt_paths() -> impl Iterator<Item = PathBuf> {
    let var = |name: &str| {
        env::var_os(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let default = if cfg!(windows) {
        var("LOCALAPPDATA")
    } else {
        env::home_dir()
            .filter(|home| !home.as_os_str().is_empty())
            .map(|home| home.join(".config"))
    };
    [var("XDG_CONFIG_HOME"), default]
        .into_iter()
        .flatten()
        .map(|dir| dir.join("kubelint").join("kubelint-receipt.json"))
}

/// Runs the release's shell installer the way `curl ... | sh` does.
#[cfg(not(windows))]
fn install(receipt: &Receipt, _exe: &Path, version: Version) -> anyhow::Result<()> {
    use std::io::Write as _;

    let script = download(&release_file(version, "kubelint-installer.sh"))?;
    let mut child = installer(Command::new("sh").arg("-s"), receipt)
        .stdin(Stdio::piped())
        .spawn()
        .context("cannot run sh")?;
    let mut stdin = child.stdin.take().context("cannot run sh")?;
    let output = std::thread::scope(|scope| {
        scope.spawn(move || stdin.write_all(&script));
        child.wait_with_output()
    })
    .context("cannot run the installer")?;
    succeeded(&output)
}

/// Runs the release's PowerShell installer from a file.
#[cfg(windows)]
fn install(receipt: &Receipt, exe: &Path, version: Version) -> anyhow::Result<()> {
    let script = download(&release_file(version, "kubelint-installer.ps1"))?;
    let file = env::temp_dir().join(format!("kubelint-installer-{}.ps1", std::process::id()));
    fs::write(&file, script).with_context(|| format!("cannot write {}", file.display()))?;
    let result = replace(receipt, exe, &file);
    let _ = fs::remove_file(&file);
    result
}

/// Windows cannot replace a running exe but can rename it, so `exe` moves aside and a copy
/// takes its place until the installer replaces it.
#[cfg(windows)]
fn replace(receipt: &Receipt, exe: &Path, installer_file: &Path) -> anyhow::Result<()> {
    let previous = exe.with_extension("previous.exe");
    fs::rename(exe, &previous).with_context(|| {
        format!(
            "cannot move {} aside to replace it. Try again",
            exe.display()
        )
    })?;
    let result = fs::copy(&previous, exe)
        .with_context(|| format!("cannot copy {}", previous.display()))
        .and_then(|_| {
            let mut command = powershell();
            command
                .args(["-ExecutionPolicy", "Bypass", "-File"])
                .arg(installer_file);
            let output = installer(&mut command, receipt)
                .output()
                .context("cannot run PowerShell")?;
            succeeded(&output)
        });
    if let Err(err) = result {
        return Err(match fs::rename(&previous, exe) {
            Ok(()) => err,
            Err(_) => err.context(format!(
                "the previous kubelint is at {}: rename it to {}",
                previous.display(),
                exe.display()
            )),
        });
    }
    delete_after_exit(&previous);
    Ok(())
}

/// Deletes `file`, the exe this process runs from, once this process exits.
#[cfg(windows)]
fn delete_after_exit(file: &Path) {
    use std::os::windows::process::CommandExt as _;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let script = format!(
        "Wait-Process -Id {} -ErrorAction Ignore; Remove-Item -LiteralPath $env:KUBELINT_OLD_EXE",
        std::process::id()
    );
    let spawned = powershell()
        .arg("-Command")
        .arg(&script)
        .env("KUBELINT_OLD_EXE", file)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
    if spawned.is_err() {
        eprintln!(
            "warning: cannot delete {}; delete it yourself",
            file.display()
        );
    }
}

#[cfg(windows)]
fn powershell() -> Command {
    let mut command = Command::new("powershell");
    command
        .args(["-NoProfile", "-NonInteractive"])
        .stdin(Stdio::null())
        // Windows PowerShell cannot load its own modules from the paths pwsh sets here.
        .env_remove("PSModulePath");
    command
}

/// Points the installer at the receipt's folder and keeps it away from the shell profiles
/// and the `PATH`, which the first install already set up.
fn installer<'a>(command: &'a mut Command, receipt: &Receipt) -> &'a mut Command {
    command
        .env("KUBELINT_INSTALL_DIR", &receipt.prefix)
        .env("KUBELINT_NO_MODIFY_PATH", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
}

fn succeeded(output: &Output) -> anyhow::Result<()> {
    if output.status.success() {
        return Ok(());
    }
    let text = String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr);
    bail!(
        "the installer failed ({}). Fix what it reports, then run `kubelint self update` \
         again:\n{}",
        output.status,
        text.trim_end()
    )
}

/// Downloads `url` with curl (part of Windows 10 and 11), or wget when curl is missing.
fn download(url: &str) -> anyhow::Result<Vec<u8>> {
    use std::io::ErrorKind::NotFound;

    let mut curl = Command::new("curl");
    curl.args(["--proto", "=https", "-fsSL"]);
    if cfg!(windows) {
        // Windows' curl fails when it cannot reach a certificate's revocation servers.
        curl.arg("--ssl-revoke-best-effort");
    }
    let output = match curl.arg(url).output() {
        Err(err) if err.kind() == NotFound => Command::new("wget").args(["-O", "-", url]).output(),
        output => output,
    };
    let output = match output {
        Err(err) if err.kind() == NotFound => {
            bail!(
                "cannot download {url}: install curl or wget, then run `kubelint self update` again"
            )
        }
        output => output.with_context(|| format!("cannot download {url}"))?,
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = stderr.lines().map(str::trim).rfind(|line| !line.is_empty());
        bail!(
            "cannot download {url}: {}. Check the network connection and try again",
            reason.map_or_else(
                || output.status.to_string(),
                |line| line.trim_end_matches('.').to_owned()
            )
        );
    }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests;
