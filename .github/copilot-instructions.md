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

## File Conventions

**All text files use LF (Unix) line endings.** `.gitattributes`
enforces `* text=auto eol=lf`, so git will renormalize on commit even
if your editor wrote CRLF. If you create a file on Windows and
`git diff` shows `^M` markers or whole-file churn, run:

```
dos2unix <file>
```

(or, if `dos2unix` isn't available, re-save with LF endings from your
editor). CRLF in shell `run:` blocks silently breaks `actionlint` /
`shellcheck` and produces noisy diffs.

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
| Host tests (all)  | `cd crates && cargo test --locked`                                   |
| Single crate test | `cargo test -p pico-de-gallo-internal --locked`                      |
| Host format       | `cd crates && cargo fmt --all`                                       |
| Host clippy       | `cd crates && cargo clippy --all-targets --locked -- -D warnings`    |
| Lockfile drift    | `cd crates && cargo check --workspace --locked` (and same in firmware) — must succeed unchanged |
| Dependency policy | `cargo deny --manifest-path crates/Cargo.toml check` (and firmware)  |
| Python wheel (dev)| `cd crates/pyco-de-gallo && maturin develop --release`               |

> **Always pass `--locked`** when validating a dependency change. Cargo
> without `--locked` happily resolves new transitive versions, masking
> the exact regression that broke the embassy-usb-driver 0.2.1 release.
> See "Dependency Change Ritual" below.

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

We use **[Conventional Commits](https://www.conventionalcommits.org/)
with a crate scope** so that
[release-please](.github/RELEASE-PLEASE.md) can drive versioning,
CHANGELOG entries, and tags automatically.

Format: `<type>(<scope>)<!>: <subject>`

- **type:** `feat`, `fix`, `chore`, `docs`, `refactor`, `perf`, `test`,
  `build`, `ci`, `revert`. Use `!` (or a `BREAKING CHANGE:` footer) for
  breaking changes.
- **scope:** `internal`, `lib`, `hal`, `ffi`, `application`, `pyco`,
  `firmware`, or `repo`. Span several with commas.
- **subject:** imperative, no trailing period.

Examples:

```
feat(internal): add device/info endpoint with capability bitfield
fix(firmware): pin embassy-usb-driver to "=0.2.0"
feat(application)!: rename --baud to --baud-rate
```

Other rules:

- Clean history — no squash merges; each commit should build without
  warnings.
- Squash miscellaneous typo/formatting fixes into their parent commit.
- Always include the
  `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`
  trailer when committing as the AI agent.
- Use `git bisect` for regression reports.

## Release Process

Releases are managed by
[release-please](.github/RELEASE-PLEASE.md). Manual `cargo set-version`
+ tag dance is **no longer the path**.

### Normal release flow

1. Land Conventional Commits on `main` as work happens.
2. release-please opens / maintains one **release PR per crate**
   (`chore(internal): release 0.6.0`, etc.). Path-dep version strings in
   dependent crates are bumped automatically by the `cargo-workspace`
   plugin.
3. **Before merging a release PR:** pull the branch, run `cargo update
   --workspace --locked` (host) and `cargo update --locked` (firmware)
   to refresh `Cargo.lock`. CI's `lockfile` job will fail the PR if you
   forget.
4. Merge the release PR. release-please creates the GitHub Release and
   the tag (e.g. `internal-v0.6.0`); the existing
   `release-{firmware,ffi,application,pyco}.yml` workflows fire on
   those tags and produce binary artifacts.

### Wire-protocol invariant — read this before every release

`pico-de-gallo-internal`'s `[package].version` is reflected at runtime
via `SCHEMA_VERSION_{MAJOR,MINOR,PATCH}` (auto-generated by
`build.rs`). When wire types change:

- Bump **`pico-de-gallo-internal`** with `feat!` / `BREAKING CHANGE:`
  (pre-1.0 = minor bump).
- Bump **`pico-de-gallo-firmware`** in the **same release cycle** so
  the firmware encodes the new schema version. release-please does
  **not** know that internal and firmware are wire-coupled — that
  discipline is on you.
- Bump **`lib`**, **`hal`**, **`ffi`**, **`application`**, **`pyco`**
  in lockstep so all downstream surfaces see the new types.

### Tag prefix glossary (canonical)

| Crate                    | release-please component | Tag prefix       |
|--------------------------|--------------------------|------------------|
| `pico-de-gallo-internal` | `internal`               | `internal-v*`    |
| `pico-de-gallo-lib`      | `library`                | `library-v*`     |
| `pico-de-gallo-hal`      | `hal`                    | `hal-v*`         |
| `pico-de-gallo-ffi`      | `ffi`                    | `ffi-v*`         |
| `gallo` (CLI)            | `application`            | `application-v*` |
| `pyco-de-gallo`          | `pyco`                   | `pyco-v*`        |
| `pico-de-gallo-firmware` | `firmware`               | `firmware-v*`    |
| KiCad gerbers            | n/a (manual)             | `hardware-v*`    |

Common typos that have bitten us: `lib-v*` (correct: `library-v*`),
`app-v*` (correct: `application-v*`), `fw-v*` (correct: `firmware-v*`).

### Pre-release checklist

If you must cut a tag manually (e.g. `hardware-v*`, or release-please
is broken):

1. From a clean checkout, run the full preflight:
   - `cd crates && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --locked`
   - `cd crates/pico-de-gallo-firmware && cargo fmt --check && cargo clippy --target thumbv8m.main-none-eabihf -- -D warnings && cargo build --release --locked --target thumbv8m.main-none-eabihf`
2. Confirm `git tag --points-at HEAD` matches expectation **and** that
   the workflow YAML at HEAD is the version you want CI to run
   (see "GitHub Actions trap" below).
3. Push the commit first; wait for CI green; then push tags.

### GitHub Actions trap

> **Tag-triggered workflows use the workflow YAML *as it existed at
> the tagged commit*, not the tip of `main`.**

If you rewrite a release commit (force-push), you **must** delete and
re-create the tags so they point at the new commit. Otherwise CI runs
the old workflow and produces wrong/broken artifacts. Verify with:

```
git --no-pager tag --points-at HEAD
git --no-pager show <tag>:.github/workflows/release-firmware.yml | grep -E 'elf2uf2|picotool'
```

## Dependency Change Ritual

When changing **any** Cargo.toml dependency or adding/removing a pin:

1. From the affected workspace directory, regenerate the lockfile
   from a clean state:
   ```
   rm -f Cargo.lock
   cargo generate-lockfile
   ```
   (Wipe `target/` too if you're suspicious of stale incremental
   artifacts: `cargo clean`.)
2. Run `cargo check --locked` to confirm the build works.
3. Commit the regenerated `Cargo.lock` **alongside** the Cargo.toml
   change. CI's `lockfile` job fails any PR that splits these apart.
4. If you're adding an `=X.Y.Z` exact pin, document **why** in the
   "Pinned dependency rationale" table below so a future contributor
   knows when it can be removed.

> **Why this matters:** the firmware uses two separate cargo workspaces.
> Stale local `Cargo.lock` files pin transitive deps to whatever
> resolved when you last ran cargo, which masks upstream breaks until
> CI runs from a clean checkout. The embassy-usb-driver 0.2.1 break
> shipped because the agent's local check used a stale lockfile.

## Pinned dependency rationale

Every `=X.Y.Z` exact pin in any `Cargo.toml` should be listed here with
the upstream issue/commit and removal criteria.

| Crate                    | Pin                              | Reason                                                                                                  | Remove when                                                                                       |
|--------------------------|----------------------------------|---------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------|
| `pico-de-gallo-firmware` | `embassy-usb-driver = "=0.2.0"`  | `0.2.1` bumped `embedded-io-async` 0.6 → 0.7, which breaks `embassy-usb 0.5.1`'s CDC-ACM `ErrorType` impl (creates two incompatible copies of `embedded-io-async` in the dep graph). `cargo-deny`'s `bans.multiple-versions` will warn. | embassy-usb 0.6 is reachable through postcard-rpc (currently it only ships `embassy-usb-0_5-server`). |

## Known traps & past regressions

When you introduce a new regression, add a one-line entry here so the
next agent doesn't repeat it.

| Date       | Trigger                                     | Symptom                                                                          | Fix                                                                                       |
|------------|---------------------------------------------|----------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------|
| 2026-05-04 | `embassy-usb-driver 0.2.1` (transitive)     | `EndpointError: embedded_io_async::Error` trait bound fails on firmware build.   | Pin `embassy-usb-driver = "=0.2.0"` in firmware Cargo.toml; commit firmware `Cargo.lock`. |
| 2026-05-04 | `elf2uf2-rs 2.2.0` on crates.io is stale    | Release CI fails: `--family` flag does not exist in published binary.            | Install elf2uf2-rs from git (`cargo install --git … --locked`) or revert to picotool.     |
| 2026-05-04 | Tag-triggered workflow uses tagged-commit YAML | After force-pushing the release commit, GitHub still ran the old workflow.    | Always re-tag after rewriting a release commit; verify with `git show <tag>:<workflow>`.  |

## PR Etiquette

- Create a **draft PR** first.
- Ensure **all CI checks pass** (especially `lockfile`, `deny`,
  `semver`, and `actionlint`) before requesting review.
- For dependency bumps, mention which Cargo.toml(s) and Cargo.lock(s)
  are touched — reviewers should see them in the same commit.

## CI Workflows

| Workflow                  | Trigger                            | What it does                                                                                              |
|---------------------------|------------------------------------|-----------------------------------------------------------------------------------------------------------|
| `check.yml`               | Push to `main`, PRs                | fmt, clippy, doc, hack (feature powerset), test, msrv, **lockfile drift**, **actionlint**, **cargo-deny**, **cargo-semver-checks** |
| `nostd.yml`               | Push to `main`, PRs                | Firmware compiles + clippy for `thumbv8m.main-none-eabihf`, both `hw-rev1` and `hw-rev2`                  |
| `gh-pages.yml`            | Push to `main`                     | Builds and deploys the mdBook docs to GitHub Pages                                                        |
| `release-please.yml`      | Push to `main`                     | Opens / maintains one release PR per crate based on Conventional Commits                                  |
| `release-application.yml` | `application-v*` tags              | Builds `gallo` for Linux/Windows/macOS                                                                    |
| `release-ffi.yml`         | `ffi-v*` tags                      | Builds `.so` / `.dll` / `.dylib` + C header                                                               |
| `release-firmware.yml`    | `firmware-v*` tags **and PRs**     | Builds `.uf2` and `.elf`. PR runs are **build-only** (skip-upload) so tooling breakage is caught at PR time |
| `release-hardware.yml`    | `hardware-v*` tags                 | KiCad ERC/DRC, gerbers, schematic PDF                                                                     |
| `release-pyco.yml`        | `pyco-v*` tags                     | Builds Python wheels (CPython 3.8–3.14, Linux/Win/macOS), attaches to GitHub Release                      |

## Commit Messages
- Subject line: capitalized, 50 characters or less, imperative mood (e.g., "Fix bug" not "Fixed bug")
- Separate subject from body with a blank line
- Wrap body text at 72 characters
- Use the body to explain *what* and *why*, not *how*

## AI Attribution
Every commit that includes AI-generated or AI-assisted work **must** contain an `Assisted-by` trailer in the commit message:
```
Assisted-by: AGENT_NAME:MODEL_VERSION [TOOL1] [TOOL2]
```
Where:
- `AGENT_NAME` is the name of the AI tool or framework (e.g., `GitHub Copilot`)
- `MODEL_VERSION` is the specific model version used (e.g., `claude-opus-4.6`)
- `[TOOL1] [TOOL2]` are optional specialized analysis tools used (e.g., `coccinelle`, `sparse`, `smatch`, `clang-tidy`)
Basic development tools (git, cargo, editors) should not be listed.
AI agents **must** verify their own identity (agent name and model version) before composing the `Assisted-by` trailer — do not assume or hard-code a model name from a previous session.
AI agents **MUST NOT** add `Signed-off-by` tags. Only humans can certify the Developer Certificate of Origin.
