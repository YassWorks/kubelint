# Getting started

This guide installs kubelint, runs it on a project, and shows how to read and act on
what it reports. It takes about five minutes.

## Install

On Linux or macOS:

```bash
curl -LsSf https://github.com/YassWorks/kubelint/releases/latest/download/kubelint-installer.sh | sh
```

On Windows, in PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/YassWorks/kubelint/releases/latest/download/kubelint-installer.ps1 | iex"
```

Microsoft Defender can block this command as `Trojan:Win32/Commando`: its machine
learning judges the command's shape, download and run, which malware also uses. If it
does, download `kubelint-x86_64-pc-windows-msvc.zip` from the
[releases page](https://github.com/YassWorks/kubelint/releases/latest) and put
`kubelint.exe` in a folder on your `PATH`
([from a release archive](#from-a-release-archive)).

The installer downloads the archive for your platform and puts `kubelint` in the first
of these folders:

1. `$XDG_BIN_HOME`, when it is set.
2. `$XDG_DATA_HOME/../bin`, when it is set.
3. `~/.local/bin` (`%USERPROFILE%\.local\bin` on Windows).

**Why:** `~/.local/bin` is the folder for user programs in the XDG base directory
specification, and many Linux distributions already put it on `PATH`. People who set
`XDG_BIN_HOME` get their own folder.

**Rejected:**

- `~/.cargo/bin`, the installer tool's default. It creates a Rust folder for people who
  never use Rust.
- `~/.kubelint/bin`. It is never on `PATH` by default and adds another folder to the
  home directory.

When the install folder is not on your `PATH`, the installer adds it. Set
`KUBELINT_NO_MODIFY_PATH=1` before running it to skip that. Open a new terminal and
check that it works:

```bash
kubelint --version
```

To install one version instead of the latest, replace `latest/download` in the command
with `download/<tag>`, where `<tag>` is a release tag such as `v1.2.3` whose assets list
`kubelint-installer.sh`.

### What the installer writes

On Linux and macOS:

- `kubelint` in the install folder above, created if missing.
- `~/.config/kubelint/` (`$XDG_CONFIG_HOME/kubelint/` when set), holding
  `kubelint-receipt.json`, which records where and how kubelint was installed. When the
  installer changes your `PATH`, this folder also holds `env.sh` and `env.fish`.
- When it changes your `PATH`: a line `. "$HOME/.config/kubelint/env.sh"` at the end of
  `~/.profile` and of each of `~/.bashrc`, `~/.bash_profile` and `~/.bash_login` that
  exists, and of `~/.zshrc` or `~/.zshenv` (in `$ZDOTDIR` when set). It creates
  `~/.profile` and `~/.zshrc` when they are missing. It also writes
  `~/.config/fish/conf.d/kubelint.env.fish`, creating its folders when missing.

On Windows:

- `kubelint.exe` in the install folder above, created if missing.
- `%LOCALAPPDATA%\kubelint\kubelint-receipt.json` (`%XDG_CONFIG_HOME%\kubelint\` when
  set).
- When it changes your `PATH`: the install folder at the front of your user `Path`.
- A folder with a random name under `%TEMP%`, holding the downloaded `kubelint.zip` and
  the license, README and changelog from it.

### From a release archive

Every release on the [releases page](https://github.com/YassWorks/kubelint/releases)
has an archive for each platform:

| Platform | Archive |
| --- | --- |
| Linux, x86-64 | `kubelint-x86_64-unknown-linux-musl.tar.gz` |
| Linux, ARM64 | `kubelint-aarch64-unknown-linux-musl.tar.gz` |
| macOS, Intel | `kubelint-x86_64-apple-darwin.tar.gz` |
| macOS, Apple silicon | `kubelint-aarch64-apple-darwin.tar.gz` |
| Windows, x86-64 | `kubelint-x86_64-pc-windows-msvc.zip` |

Each archive holds the `kubelint` binary, the license, the README and the changelog: in
a `kubelint-<platform>/` folder in a `.tar.gz`, at the top of a `.zip`. The Linux
binaries are static and run on any distribution, and the Windows binary needs no Visual
C++ runtime. `sha256.sum` lists the checksum of every archive, and each archive also
has its own `<archive>.sha256`. Older releases put the version in archive names, such as
`kubelint-v0.18.15-x86_64-unknown-linux-musl.tar.gz`, and list checksums in `SHA256SUMS`.

On Linux or macOS, set the platform, then download the latest release and unpack it:

```bash
TARGET=x86_64-unknown-linux-musl
curl -fsSL "https://github.com/YassWorks/kubelint/releases/latest/download/kubelint-$TARGET.tar.gz" \
  | tar -xz --strip-components=1 "kubelint-$TARGET/kubelint"
sudo mv kubelint /usr/local/bin/
```

### From source

Install [rustup](https://rustup.rs) and run:

```bash
cargo install --locked --git https://github.com/YassWorks/kubelint kubelint
```

### Update

Installed with the installer:

```bash
kubelint self update
```

It runs the latest release's installer again, into the same folder, and leaves your
shell profiles and `PATH` alone. It needs `curl` or `wget` (Windows 10 and 11 include
`curl`), and PowerShell on Windows. On Windows each update, like the install, leaves a
download folder under `%TEMP%` (step 5 of [Uninstall](#uninstall) deletes them).

Installed another way, `kubelint self update` changes nothing and exits with code 2.
Download the new release archive as above, or run the `cargo install` command again.

### Uninstall

Installed with the installer on Linux or macOS:

1. Delete the binary: `rm "$(command -v kubelint)"`. Delete its folder too if it is now
   empty, and `~/.local` if that is then empty.
2. List the shell profiles the installer changed:

   ```bash
   grep -l 'kubelint/env.sh' ~/.profile ~/.bashrc ~/.bash_profile ~/.bash_login \
     "${ZDOTDIR:-$HOME}/.zshrc" "${ZDOTDIR:-$HOME}/.zshenv" 2>/dev/null
   ```

   In each, delete the line that sources `kubelint/env.sh` and the empty line above it.
   Delete `~/.profile` or `.zshrc` if it is then empty: the installer created it.
3. Delete `~/.config/fish/conf.d/kubelint.env.fish`, and its folders if they are then
   empty.
4. Delete the receipt folder: `rm -r "${XDG_CONFIG_HOME:-$HOME/.config}/kubelint"`. Delete
   `~/.config` too if it is now empty.

Installed with the installer on Windows, in PowerShell:

1. Note the install folder: `Split-Path (Get-Command kubelint).Source`.
2. Delete the binary: `Remove-Item (Get-Command kubelint).Source`.
3. Delete the receipt folder: `Remove-Item -Recurse "$env:LOCALAPPDATA\kubelint"`
   (`"$env:XDG_CONFIG_HOME\kubelint"` when that variable is set).
4. If the install folder is now empty, delete it (and `%USERPROFILE%\.local` if that is
   then empty) and remove it from your user `Path` (search "environment variables" in
   the Start menu). Nothing records whether the installer added it, so keep it when
   other programs live there.
5. Delete the installer's download folder:

   ```powershell
   Get-ChildItem $env:TEMP -Directory | Where-Object { Test-Path "$($_.FullName)\kubelint.zip" } | Remove-Item -Recurse
   ```

Built from source: `cargo uninstall kubelint`.

## Lint a project

From the root of a repository:

```bash
kubelint check .
```

kubelint searches the directories you name for `*.yaml` and `*.yml` files, honoring
`.gitignore`, `.ignore` and your global Git excludes. It skips hidden files and
directories and does not follow symbolic links: name such a file to lint it. If the `.gitignore` and
`.ignore` files above a directory you name (and the repository's `.git/info/exclude`) hide all
of its YAML files, they do not apply to that directory, so `kubelint check gen` lints `gen/`
even when the repository's `.gitignore` lists `gen/*`. The global Git excludes and the ignore
files inside the directory always apply. When some of its YAML files are visible, all the
rules apply and the hidden ones stay skipped. The rules run on Kubernetes objects only. Other YAML (CI files, Helm
values) is only checked for valid syntax. You can also name files, or pass `-` to read standard input.

## Read a finding

```text
$ kubelint check k8s
warning[KL3003]: image `nginx:latest` uses the `latest` tag
  --> k8s/web.yaml:14:16
   |
14 |         image: nginx:latest
   |                ^^^^^^^^^^^^
   |
   = help: pin a version tag, such as `nginx:1.2.3`, or a digest, such as `nginx@sha256:<digest>`
   = note: rule `unpinned-image-tag`

error[KL1009]: container `web` sets env var `PORT` to a value that YAML reads as a number, not a string
  --> k8s/web.yaml:17:18
   |
17 |           value: 8080
   |                  ^^^^
   |
   = help: quote the value: `value: "8080"`
   = note: rule `non-string-env-value`

Found 1 error and 1 warning. Checked 1 file and 1 object in 3ms.
```

Each finding has:

- a **severity** (`error`, `warning` or `info`) and the **rule code**, such as `KL1009`;
- a message saying what is wrong;
- the **file, line and column**, and the offending YAML with the exact part underlined;
- a **help** line saying how to fix it;
- the rule's **name**, which you can use instead of its code anywhere.

To learn why a rule exists and see an example and its fix, ask kubelint:

```bash
kubelint rule KL1009
```

The same text is in the [rule catalog](../rules/README.md), one page per rule.
`kubelint rules` lists every rule and whether it runs by default.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Nothing at or above the `fail-on` severity (`warning` by default) was found. |
| `1` | At least one finding at or above `fail-on`. |
| `2` | kubelint could not run: a missing file, a bad argument or an invalid configuration. |

## Helm charts and Kustomize

kubelint lints rendered YAML. It skips a Helm chart's `templates/` and `Chart.yaml`
with a note, because templates are not valid YAML before rendering. Render first and
pipe the result in:

```bash
helm template charts/web | kubelint check -
```

```bash
kustomize build overlays/prod | kubelint check -
```

## Next steps

- Too many findings at first? Make only errors fail the run (`kubelint check --fail-on
  error .`), or pick rule categories in a [configuration file](configuration.md).
- A finding you have decided to accept? [Suppress it](suppressions.md) with a comment
  or an annotation, and say why.
- Ready to gate pull requests? See [CI integration](ci-integration.md).
