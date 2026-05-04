# Copilot Instructions for pico-de-gallo

## Project Overview

pico-de-gallo is a USB bridge device built on Embassy-rs (RP2350/Pico
2) that exposes I2C, SPI, UART, GPIO, PWM, ADC, and 1-Wire over USB
using postcard-rpc.

## Project Structure

```
crates/
├── Cargo.toml                    # Host workspace (6 crates)
├── pico-de-gallo-internal/       # Shared wire-protocol types (postcard-rpc endpoints)
├── pico-de-gallo-lib/            # Host-side async library (nusb + tokio)
├── pico-de-gallo-hal/            # embedded-hal trait impls wrapping lib
├── pico-de-gallo-ffi/            # C FFI bindings (cdylib + cbindgen)
├── pico-de-gallo-app/            # CLI application — binary name is `gallo`
├── pyco-de-gallo/                # Python bindings (PyO3 + maturin, cdylib)
└── pico-de-gallo-firmware/       # no_std Embassy firmware (SEPARATE workspace)
book/                             # mdBook documentation
hardware/                         # KiCad landing board PCB (not required, but makes wiring easier)
case/                             # 3D-printable enclosure (FreeCAD)
```

**Key facts:**

- Host crates share a Cargo workspace at `crates/Cargo.toml` (members:
  `pico-de-gallo-{app,ffi,hal,internal,lib}` and `pyco-de-gallo`;
  `pico-de-gallo-firmware` is excluded).
- Firmware is in a **separate Cargo workspace**
  (`crates/pico-de-gallo-firmware/`) because it targets
  `thumbv8m.main-none-eabihf`. It cannot share a workspace with host
  crates. **Firmware must still compile cleanly and pass clippy.**
- Firmware has mutually exclusive `hw-rev1` (default) / `hw-rev2`
  Cargo features. `nostd.yml` builds and lints both. The release
  workflow produces a separate `.uf2` per revision.
- Firmware uses **`defmt` only** for logging (over RTT). Do not use
  `log`, `println!`, or `eprintln!` in firmware code.
- `pyco-de-gallo` is a **PyO3 cdylib** built with `maturin`. It is
  `publish = false` on crates.io — wheels are published to PyPI via the
  release workflow. It **is** part of `check.yml` (fmt, clippy, doc,
  hack, test, msrv) on equal footing with the other host crates.
- All crates use **Rust 2024 edition**
- MSRV: **1.90**

## Build & Test Commands

CI in `.github/workflows/check.yml` runs each job **per crate**
(`cd crates/<crate>` then `cargo …`). The workspace-level shortcuts
work locally, but per-crate failures are what CI actually gates on,
so reproduce CI by iterating per-crate when something fails.

### Per-crate (matches CI exactly)

| Task                | Command                                                              |
|---------------------|----------------------------------------------------------------------|
| Format check        | `cd crates/<crate> && cargo fmt --check`                             |
| Clippy              | `cd crates/<crate> && cargo clippy --all-targets -- -D warnings`     |
| Tests               | `cd crates/<crate> && cargo test`                                    |
| Docs (nightly)      | `cd crates/<crate> && RUSTDOCFLAGS=--cfg docsrs cargo +nightly doc --no-deps --all-features` |
| Feature powerset    | `cd crates/<crate> && cargo hack --feature-powerset check`           |
| MSRV check          | `cd crates/<crate> && cargo +1.90 check`                             |

The `check.yml` matrix iterates `pico-de-gallo-{app,internal,ffi,hal,lib}`
and `pyco-de-gallo` (plus `pico-de-gallo-firmware` for `fmt`). Run the
appropriate command in each crate to mirror CI.

### Workspace shortcuts (local convenience)

| Task              | Command                                                              |
|-------------------|----------------------------------------------------------------------|
| Host tests (all)  | `cd crates && cargo test`                                            |
| Single crate test | `cargo test -p pico-de-gallo-internal`                               |
| Host format       | `cd crates && cargo fmt --all`                                       |
| Host clippy       | `cd crates && cargo clippy --all-targets -- -D warnings`             |
| Python wheel (dev)| `cd crates/pyco-de-gallo && maturin develop --release`               |

### Firmware (separate workspace, no_std)

| Task               | Command                                                                                                              |
|--------------------|----------------------------------------------------------------------------------------------------------------------|
| Build hw-rev1      | `cd crates/pico-de-gallo-firmware && cargo build --release --target thumbv8m.main-none-eabihf`                       |
| Build hw-rev2      | `cd crates/pico-de-gallo-firmware && cargo build --release --target thumbv8m.main-none-eabihf --no-default-features --features hw-rev2` |
| Clippy hw-rev1     | `cd crates/pico-de-gallo-firmware && cargo clippy --target thumbv8m.main-none-eabihf -- -D warnings`                 |
| Clippy hw-rev2     | `cd crates/pico-de-gallo-firmware && cargo clippy --target thumbv8m.main-none-eabihf --no-default-features --features hw-rev2 -- -D warnings` |
| Format             | `cd crates/pico-de-gallo-firmware && cargo fmt`                                                                      |

The release-mode firmware binary is named `pico-de-gallo-firmware`
(matches the package and directory name).

**Current test baseline:** ~300 unit tests + 3 doctests across the
host workspace (the bulk live in `pico-de-gallo-internal` and
`pico-de-gallo-lib`). `pyco-de-gallo` currently has no Rust-side
tests, but `cargo test` is gated by CI so any future tests run
automatically.

> **Note:** `pico-de-gallo-internal` without the `use-std` feature
> fails on `vec!` macro. Test it via the workspace or with `--features
> use-std`.

## Wire Protocol Rules — CRITICAL

All protocol types live in `pico-de-gallo-internal`.

### Enum Ordering is ABI

postcard serializes enums by **variant index** (0, 1, 2, …), **NOT**
by discriminant value.

- **Never reorder enum variants** — this silently breaks wire
  compatibility between firmware and host.
- All wire enums have `// WARNING: Do not reorder...` comments —
  **preserve these**.
- Adding new variants is safe **only at the end**.

### Endpoint Definitions

Endpoints use the `endpoints!` macro with path strings. Response types
use `#[cfg(feature = "use-std")]` to switch between `Vec<u8>` (host)
and `&[u8]` (firmware).

The schema version is exposed by the `SCHEMA_VERSION_MAJOR`,
`SCHEMA_VERSION_MINOR`, and `SCHEMA_VERSION_PATCH` constants in
`pico-de-gallo-internal`. They are **auto-generated** from the
crate's `[package].version` by `pico-de-gallo-internal/build.rs` —
do not edit a constant; bump the crate version. Pre-1.0 the **minor**
version is the breaking-change axis — bump it whenever the wire
protocol changes (new endpoints, reordered enum variants, changed
request/response types). `PicoDeGallo::validate()` checks this at
runtime.

### Current Endpoints

| Path                     | Description                                           |
|--------------------------|-------------------------------------------------------|
| `"ping"`                 | Echo a u32 (testing)                                  |
| `"version"`              | Get firmware version                                  |
| `"device/info"`          | Get firmware version, schema version, capabilities    |
| `"i2c/read"`             | I2C read                                              |
| `"i2c/write"`            | I2C write                                             |
| `"i2c/write-read"`       | I2C write-then-read                                   |
| `"i2c/scan"`             | Scan I2C bus for responding addresses                 |
| `"i2c/batch"`            | Execute a batch of I2C operations                     |
| `"i2c/set-config"`       | Configure I2C (`I2cFrequency` enum)                   |
| `"i2c/get-config"`       | Query current I2C frequency                           |
| `"spi/read"`             | SPI read                                              |
| `"spi/write"`            | SPI write                                             |
| `"spi/flush"`            | SPI flush                                             |
| `"spi/transfer"`         | SPI full-duplex transfer                              |
| `"spi/batch"`            | SPI batch under chip-select (read/write/transfer/delay) |
| `"spi/set-config"`       | Configure SPI (frequency, phase, polarity)            |
| `"spi/get-config"`       | Query current SPI configuration                       |
| `"uart/read"`            | UART read with timeout                                |
| `"uart/write"`           | UART write                                            |
| `"uart/flush"`           | Flush UART TX buffer                                  |
| `"uart/set-config"`      | Configure UART (baud rate)                            |
| `"uart/get-config"`      | Query current UART configuration                      |
| `"gpio/get"`             | Read GPIO pin                                         |
| `"gpio/put"`             | Set GPIO pin                                          |
| `"gpio/wait-high"`       | Wait for GPIO high                                    |
| `"gpio/wait-low"`        | Wait for GPIO low                                     |
| `"gpio/wait-rising"`     | Wait for rising edge                                  |
| `"gpio/wait-falling"`    | Wait for falling edge                                 |
| `"gpio/wait-any"`        | Wait for any edge                                     |
| `"gpio/set-config"`      | Configure GPIO direction and pull                     |
| `"gpio/subscribe"`       | Subscribe to push-based GPIO edge events              |
| `"gpio/unsubscribe"`     | Unsubscribe from GPIO edge events                     |
| `"pwm/set-duty-cycle"`   | Set raw PWM compare value                             |
| `"pwm/get-duty-cycle"`   | Query current duty cycle and max                      |
| `"pwm/enable"`           | Enable PWM slice owning the channel                   |
| `"pwm/disable"`          | Disable PWM slice owning the channel                  |
| `"pwm/set-config"`       | Configure PWM frequency / phase-correct               |
| `"pwm/get-config"`       | Query PWM configuration                               |
| `"adc/read"`             | Single-shot ADC read                                  |
| `"adc/get-config"`       | Query ADC capabilities                                |
| `"onewire/reset"`        | 1-Wire reset + presence detection                     |
| `"onewire/read"`         | 1-Wire read                                           |
| `"onewire/write"`        | 1-Wire write                                          |
| `"onewire/write-pullup"` | 1-Wire write + strong pullup (parasitic power)        |
| `"onewire/search"`       | Start 1-Wire ROM search                               |
| `"onewire/search-next"`  | Continue 1-Wire ROM search                            |

### Topics

| Path           | Direction         | Message    | Description                  |
|----------------|-------------------|------------|------------------------------|
| `"gpio/event"` | server → client   | `GpioEvent` | Push stream of GPIO edges   |

## Dependency Constraints

| Constraint                              | Reason                                                           |
|-----------------------------------------|------------------------------------------------------------------|
| `embassy-usb` must be **0.5** (not 0.6) | postcard-rpc 0.12's `embassy-usb-0_5-server` feature requires it |
| `embassy-sync` must stay at **0.7.2**   | Compatibility lock                                               |
| `nusb` must stay at **0.1.x**           | postcard-rpc host-client dependency                              |
| `pyo3` is on **0.28.x**                 | Used by `pyco-de-gallo` via `maturin`                            |

## FFI Conventions

- **Opaque pointer pattern:** `PicoDeGallo` is opaque, created by
  `gallo_init`, freed by `gallo_free`
- All functions take `*const PicoDeGallo` as first argument; **check
  for null first**
- Status codes are `#[repr(i32)]` — `Ok = 0`, all errors are
  **negative**
- Status code values are **stable C ABI** — never renumber existing
  codes, only add new ones at the end
- `I2cFrequency` is passed as `u8` (`0 = Standard`, `1 = Fast`, `2 =
  FastPlus`) with validation
- cbindgen generates `pico_de_gallo.h` automatically during build

## Python Bindings (pyco-de-gallo) Conventions

- Built with **PyO3 + maturin**; `pyproject.toml` lives in the crate
  root and declares `requires-python = ">=3.8"`.
- Module name in Python: `pyco_de_gallo`. Public types are exposed
  **without** a `Py` prefix (e.g. `I2cFrequency`, `SpiBatchOp`,
  `DeviceInfo`). The host-side `pico_de_gallo_lib` types are imported
  with a `Lib` prefix internally (`LibI2cFrequency`, etc.) to avoid
  collisions with the wrapper types of the same name.
- The `PycoDeGallo` class owns its own Tokio `Runtime`; all async
  methods are exposed as **synchronous** Python methods that internally
  `block_on` the underlying future.
- For `#[pyclass]` enums passed by value (e.g. as `Vec<T>` arguments),
  derive `Clone` and use `#[pyclass(from_py_object)]` so PyO3 can
  extract them — `Vec<MyEnum>` does **not** work without `Clone`.
- All `#[pyfunction]`, `#[pymethods]`, and `#[pyclass]` items must
  carry rustdoc comments — they become the Python `__doc__` attribute
  surfaced by `help()` and IDE tooltips. Prefer Google-style
  `Args:`/`Returns:`/`Raises:` sections so Sphinx napoleon and Pyright
  render them well.
- Errors from the underlying lib are converted to Python `RuntimeError`
  via `PyRuntimeError::new_err(format!("{e}"))`.

## Testing Conventions

- Tests are organized as `#[cfg(test)] mod tests` inline in each
  crate's `src/lib.rs`
- **Naming:** `type_name_behavior()` (e.g.,
  `i2c_read_request_round_trip`)
- Round-trip serialization tests for every wire type using
  `postcard::{from_bytes, to_allocvec}` (requires `use-std` feature or
  workspace-level test)
- FFI tests check null pointers, status code invariants, argument
  validation
- CLI tests verify clap argument parsing
- `pyco-de-gallo` is exercised in CI via `check.yml` (fmt, clippy,
  doc, hack, test, msrv) like every other host crate. There are no
  Rust-side unit tests yet — behavior is covered transitively by
  `pico-de-gallo-lib` tests and exercised by hand from Python.

## Documentation Requirements

- All public items must have **rustdoc documentation**
- Crate-level `//!` docs are required for every crate
- For `pyco-de-gallo`, doc comments double as Python docstrings — write
  them in a style Python users will actually read.
- Update `book/` when adding new endpoints or changing CLI behavior
- Update `CHANGELOG.md` (Keep a Changelog format) for endpoint
  additions, CLI changes, wire-protocol changes, and any change that
  alters a release artifact name or path.
- `README.md` at repo root should reflect the high-level project
  overview

## Commit Conventions

- Meaningful commit messages (see Tim Pope's blog post on git commit
  messages)
- Clean history — no squash merges; each commit should build without
  warnings
- Squash miscellaneous typo/formatting fixes into their parent commit
- Use `git bisect` for regression reports

## PR Etiquette

- Create a **draft PR** first
- Ensure **all CI checks pass** before requesting review

## CI Workflows

| Workflow                  | Trigger               | What it does                                              |
|---------------------------|-----------------------|-----------------------------------------------------------|
| `check.yml`               | Push to `main`, PRs   | fmt, clippy, doc, hack (feature powerset), test, msrv — runs per-crate over `pico-de-gallo-{app,internal,ffi,hal,lib}` and `pyco-de-gallo` (firmware also covered by `fmt`) |
| `nostd.yml`               | Push to `main`, PRs   | Firmware compiles + clippy for `thumbv8m.main-none-eabihf`, both `hw-rev1` and `hw-rev2` |
| `gh-pages.yml`            | Push to `main`        | Builds and deploys the mdBook docs to GitHub Pages        |
| `release-application.yml` | `application-v*` tags | Builds `gallo` for Linux/Windows/macOS                    |
| `release-ffi.yml`         | `ffi-v*` tags         | Builds .so/.dll/.dylib + C header                         |
| `release-firmware.yml`    | `firmware-v*` tags    | Builds .uf2 and .elf                                      |
| `release-hardware.yml`    | `hardware-v*` tags    | KiCad ERC/DRC, gerbers, schematic PDF                     |
| `release-pyco.yml`        | `pyco-v*` tags        | Builds Python wheels (CPython 3.8–3.14, Linux/Win/macOS), attaches to GitHub Release |

