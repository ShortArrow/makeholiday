[ **English** | [日本語](SETUP.jp.md) ]

# Setup

How to get `makeholiday` running on your machine. For day-to-day commands see [USAGE.md](USAGE.md). For development workflow see [CONTRIBUTING.md](CONTRIBUTING.md).

## Prerequisites

- **Rust toolchain** with edition 2024 support. Install via [rustup](https://rustup.rs/):
  ```sh
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
  On Windows, download `rustup-init.exe` from the same site.
- **A C linker.** Already present on Linux (gcc/clang) and macOS (Xcode Command Line Tools). On Windows, install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the "Desktop development with C++" workload.

Verify the toolchain:

```sh
rustc --version
cargo --version
```

## Install

### From local checkout

```sh
git clone https://github.com/ShortArrow/makeholiday.git
cd makeholiday
cargo install --path .
```

This places the `makeholiday` binary into `~/.cargo/bin/` (or `%USERPROFILE%\.cargo\bin\` on Windows). Make sure that directory is on your `PATH`.

### From crates.io

Not yet published. Track [PRD §5.2](PRD.md#52-planned) for status.

### Build without installing

```sh
cargo build --release
./target/release/makeholiday --help
```

## Verify

```sh
makeholiday --help
makeholiday icons
```

A working install prints help text and a preset icon list without errors.

## Platform Notes

- **Windows.** Use PowerShell or any modern terminal. Line endings in generated `.ics` files are CRLF as required by RFC 5545; this is independent of the host OS.
- **macOS.** No extra steps beyond the prerequisites.
- **Linux.** No extra steps. Distribution packages are not provided; install via `cargo install`.

## Updating

From a local checkout:

```sh
git pull
cargo install --path . --force
```

`--force` is required because `cargo install` refuses to overwrite an existing binary by default.

## Uninstall

```sh
cargo uninstall makeholiday
```

## Troubleshooting

- **`error: linker 'cc' not found`** — install your platform's C build tools (see Prerequisites).
- **`makeholiday: command not found`** after install — confirm that `~/.cargo/bin` is on `PATH`.
- **Toolchain too old** — `rustup update stable` to refresh.
