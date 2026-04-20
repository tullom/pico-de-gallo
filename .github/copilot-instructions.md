# Copilot Instructions for pico-de-gallo

## Project Overview

pico-de-gallo is a USB bridge device built on Embassy-rs (RP2350/Pico
2) that exposes I2C, SPI, and GPIO over USB using postcard-rpc.

## Project Structure

```
crates/
├── Cargo.toml                    # Host workspace (5 crates)
├── pico-de-gallo-internal/       # Shared wire-protocol types (postcard-rpc endpoints)
├── pico-de-gallo-lib/            # Host-side async library (nusb + tokio)
├── pico-de-gallo-hal/            # embedded-hal trait impls wrapping lib
├── pico-de-gallo-ffi/            # C FFI bindings (cdylib + cbindgen)
├── pico-de-gallo-app/            # CLI application — binary name is `gallo`
└── pico-de-gallo-firmware/       # no_std Embassy firmware (SEPARATE workspace)
book/                             # mdBook documentation
hardware/                         # KiCad landing board PCB (not required, but makes wiring easier)
case/                             # 3D-printable enclosure (FreeCAD)
```

**Key facts:**

- Host crates share a Cargo workspace at `crates/Cargo.toml`
- Firmware is in a **separate Cargo workspace**
  (`crates/pico-de-gallo-firmware/`) because it targets
  `thumbv8m.main-none-eabihf`. It cannot share a workspace with host
  crates. **Firmware must still compile cleanly and pass clippy.**
- All crates use **Rust 2024 edition**
- MSRV: **1.90**

## Build & Test Commands

| Task              | Command                                                                                              |
|-------------------|------------------------------------------------------------------------------------------------------|
| Host tests (all)  | `cd crates && cargo test`                                                                            |
| Single crate test | `cargo test -p pico-de-gallo-internal`                                                               |
| Firmware build    | `cd crates/pico-de-gallo-firmware && cargo build --release --target thumbv8m.main-none-eabihf`       |
| Firmware clippy   | `cd crates/pico-de-gallo-firmware && cargo clippy --target thumbv8m.main-none-eabihf -- -D warnings` |
| Host format       | `cd crates && cargo fmt --all`                                                                       |
| Firmware format   | `cd crates/pico-de-gallo-firmware && cargo fmt`                                                      |
| Host clippy       | `cd crates && cargo clippy --all-targets -- -D warnings`                                             |
| Docs (per crate)  | `cargo doc --no-deps --all-features`                                                                 |

**Current test baseline:** 115 tests + 3 doctests across 5 host
crates.

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

### Current Endpoints

| Path                  | Description                                |
|-----------------------|--------------------------------------------|
| `"ping"`              | Echo a u32                                 |
| `"i2c/read"`          | I2C read                                   |
| `"i2c/write"`         | I2C write                                  |
| `"i2c/write-read"`    | I2C write-then-read                        |
| `"i2c/set-config"`    | Configure I2C (I2cFrequency enum)          |
| `"spi/read"`          | SPI read                                   |
| `"spi/write"`         | SPI write                                  |
| `"spi/flush"`         | SPI flush                                  |
| `"spi/transfer"`      | SPI full-duplex transfer                   |
| `"spi/set-config"`    | Configure SPI (frequency, phase, polarity) |
| `"gpio/get"`          | Read GPIO pin                              |
| `"gpio/put"`          | Set GPIO pin                               |
| `"gpio/wait-high"`    | Wait for GPIO high                         |
| `"gpio/wait-low"`     | Wait for GPIO low                          |
| `"gpio/wait-rising"`  | Wait for rising edge                       |
| `"gpio/wait-falling"` | Wait for falling edge                      |
| `"gpio/wait-any"`     | Wait for any edge                          |
| `"version"`           | Get firmware version                       |

## Dependency Constraints

| Constraint                              | Reason                                                           |
|-----------------------------------------|------------------------------------------------------------------|
| `embassy-usb` must be **0.5** (not 0.6) | postcard-rpc 0.12's `embassy-usb-0_5-server` feature requires it |
| `embassy-sync` must stay at **0.7.2**   | Compatibility lock                                               |
| `nusb` must stay at **0.1.x**           | postcard-rpc host-client dependency                              |

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

## Documentation Requirements

- All public items must have **rustdoc documentation**
- Crate-level `//!` docs are required for every crate
- Update `book/` when adding new endpoints or changing CLI behavior
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

| Workflow                  | Trigger               | What it does                                          |
|---------------------------|-----------------------|-------------------------------------------------------|
| `check.yml`               | Push to `main`, PRs   | fmt, clippy, doc, hack (feature powerset), test, msrv |
| `nostd.yml`               | Push to `main`, PRs   | Firmware compiles for `thumbv8m.main-none-eabihf`     |
| `release-application.yml` | `application-v*` tags | Builds `gallo` for Linux/Windows/macOS                |
| `release-ffi.yml`         | `ffi-v*` tags         | Builds .so/.dll/.dylib + C header                     |
| `release-firmware.yml`    | `firmware-v*` tags    | Builds .uf2 and .elf                                  |
| `release-hardware.yml`    | `hardware-v*` tags    | KiCad ERC/DRC, gerbers, schematic PDF                 |
