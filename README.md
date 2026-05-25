# axmon

`axmon` is a compact terminal monitor for AX embedded boards, inspired by
`nvitop`.

It is designed for boards where a floating GTK dashboard is inconvenient, and a
terminal UI is easier to run over SSH, serial console, or a local shell.

## Features

- Dynamic terminal dashboard with colored usage bars
- One-shot plain output for scripts and logs
- No Rust toolchain required for normal npm installation on Linux ARM64
- Direct AX system metric readers, no Python or GTK dependency
- Root-friendly behavior for NPU monitor enable and bandwidth module loading

Metrics:

- CPU usage from `/proc/stat`
- DDR OS memory from `/proc/meminfo`
- DDR CMM memory from `/proc/ax_proc/mem_cmm_info`
- NPU usage from `/proc/ax_proc/npu/top`
- SoC temperature from `/sys/class/thermal/thermal_zone0/temp`
- AX bandwidth from `/proc/ax_proc/bw/bw`

## Install

### From GitHub

Replace `YOUR_NAME/axmon` with your repository path:

```bash
npm install -g github:YOUR_NAME/axmon
```

Run:

```bash
axmon
```

### From npm

After publishing this package to npm:

```bash
npm install -g @ax-embedded/axmon
```

Run:

```bash
axmon
```

### From Local Source

Inside this repository:

```bash
npm install -g .
axmon
```

## Usage

Dynamic dashboard:

```bash
axmon
```

One snapshot:

```bash
axmon --once
```

Plain output:

```bash
axmon --plain
```

Set refresh interval:

```bash
axmon --interval-ms 500
```

Quit dynamic mode with `q`, `Esc`, or `Ctrl+C`.

## Permissions

For full AX board metrics, run as root:

```bash
sudo axmon
```

Root is needed when `axmon` writes `1` to:

```text
/proc/ax_proc/npu/enable
```

Bandwidth monitoring may require loading:

```text
/soc/ko/ax_perf_monitor.ko
```

If permissions or AX proc files are unavailable, `axmon` keeps running and shows
`N/A` for the affected metric.

## Platform Support

The npm package currently ships a prebuilt binary for:

```text
linux-arm64
```

That is the target platform for AX ARM64 boards. Rust is not required for this
normal install path.

For other platforms, build from source:

```bash
cargo build --release
./target/release/axmon
```

## Repository Layout

```text
bin/axmon.js                 npm command wrapper
prebuilt/linux-arm64/axmon   bundled Linux ARM64 binary
scripts/postinstall.js       install-time binary check
src/                         Rust source
Cargo.toml                   Rust package manifest
package.json                 npm package manifest
```

## Development

Build:

```bash
cargo build --release
```

Run the Rust binary directly:

```bash
sudo ./target/release/axmon
```

Test the npm wrapper locally:

```bash
node bin/axmon.js --once
```

Check npm package contents:

```bash
npm pack --dry-run
```

## Uninstall

If installed globally with npm:

```bash
npm uninstall -g @ax-embedded/axmon
```

If installed from GitHub or local source, npm still records the package by the
`name` in `package.json`, so the same uninstall command applies.

If your shell still points to an old path after uninstalling, clear bash's
command cache:

```bash
hash -r
```

## Notes

- `axmon` uses the bundled prebuilt binary for npm installation.
- The Rust code itself uses only the standard library.
- The monitor reads AX Linux proc/sys files directly; it does not call
  `ax_dashboard`.
- `insmod` and `stty` may be used at runtime for bandwidth monitoring and
  terminal control.

