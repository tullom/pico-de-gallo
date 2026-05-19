# Releases & Compatibility

Pico de Gallo releases are automated, but compatibility still depends on humans
understanding which pieces move together.

## Tag prefixes

Each published surface has its own release tag prefix:

| Component | Tag |
|-----------|-----|
| `pico-de-gallo-internal` | `internal-v*` |
| `pico-de-gallo-lib` | `library-v*` |
| `pico-de-gallo-hal` | `hal-v*` |
| `pico-de-gallo-ffi` | `ffi-v*` |
| `gallo` CLI | `application-v*` |
| `pyco-de-gallo` | `pyco-v*` |
| `pico-de-gallo-firmware` | `firmware-v*` |
| hardware artifacts | `hardware-v*` |

## What drives a release?

The project uses `release-please`. Day-to-day, contributors land Conventional
Commits with crate scopes such as `feat(internal): ...` or `fix(firmware): ...`.
From that history, release-please opens and maintains one release PR per crate.

> [!TIP]
> The scope is not decoration. It is part of how release automation decides what
> to version and publish.

## Protocol changes are lockstep changes

When the wire protocol changes, compatibility is broader than one crate tag.
The protocol crate, firmware, and every host-facing crate must move in the same
release cycle.

That means coordinating:

- `pico-de-gallo-internal`,
- `pico-de-gallo-firmware`,
- `pico-de-gallo-lib`,
- `pico-de-gallo-hal`,
- `pico-de-gallo-ffi`,
- `pico-de-gallo-app`,
- `pyco-de-gallo`.

> [!IMPORTANT]
> `release-please` does not enforce wire coupling for you. If a protocol change
> lands without its matching host and firmware updates, users will feel it.

## How users check compatibility

There are two main compatibility checks:

- `gallo version` prints firmware version, schema version, hardware revision,
  and capabilities.
- `PicoDeGallo::validate()` checks compatibility programmatically and fails with
  `SchemaMismatch` or `LegacyFirmware` when the pair should not talk.

For most users, `gallo version` is the first stop. For library users,
`validate()` is the guardrail you call before doing real work.

## “I flashed new firmware and now my host is broken”

That usually means the firmware and host were built against different versions
of `pico-de-gallo-internal`.

Typical symptoms include:

- `validate()` returning `SchemaMismatch`,
- a new firmware exposing endpoints an older host does not know about,
- older firmware lacking `device/info`, which shows up as `LegacyFirmware`.

The fix is simple: upgrade the matching host component for the firmware you
flashed, or downgrade the firmware to the host release you are using.

> [!CAUTION]
> The protocol is typed, not best-effort. A mismatched pair is expected to fail
> fast instead of guessing.

## MSRV and release hygiene

The workspace tracks Rust 1.90 as its MSRV, and CI checks it explicitly. That
includes the host workspace and the firmware workspace.

For contributor-only release details, including manual-tag edge cases, see
[`AGENTS.md`](https://github.com/OpenDevicePartnership/pico-de-gallo/blob/main/AGENTS.md)
and the repository's
[`RELEASE-PLEASE.md`](https://github.com/OpenDevicePartnership/pico-de-gallo/blob/main/.github/RELEASE-PLEASE.md).
