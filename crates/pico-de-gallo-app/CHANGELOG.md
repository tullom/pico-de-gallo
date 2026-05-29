# Changelog

All notable changes to `gallo` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] — 2026-05-04

### Added

- `gallo version` now shows schema version, HW revision, and
  capabilities with graceful fallback for legacy firmware.

## [0.5.0] — 2026-04-22

### Added

- `gallo gpio monitor --pin N --edge rising|falling|any` command.
  Subscribes, prints edge events with timestamps, unsubscribes on
  Ctrl+C.
- `gallo i2c batch` and `gallo spi batch` CLI commands for
  executing batched operations (e.g.,
  `--op write:0x00,0x10 --op read:16`).
- `gallo pwm` subcommand group with `set-duty`, `get-duty`,
  `enable`, `disable`, `set-config`, and `get-config` commands.
- `gallo adc` subcommand group with `read` and `info` commands.
- `gallo onewire` subcommand group with `reset`, `read`, `write`,
  `write-pullup`, and `search` commands.
- `gallo uart` subcommand group with `read`, `write`, `flush`,
  `set-config`, and `get-config` commands.
- `gallo i2c scan` now uses the dedicated scan endpoint (single
  round-trip) instead of 112 individual reads.
- `gallo gpio set-config`, `gallo gpio get`, and `gallo gpio put`
  subcommands for direct GPIO access from the command line.
- `gallo i2c get-config` and `gallo spi get-config` subcommands.

## [0.4.0] — 2025-04-20

### Breaking Changes

- CLI `set-config` command replaced by `i2c set-config` and
  `spi set-config` subcommands.

### Added

- `list` command to show connected devices with serial numbers.

### Changed

- `I2cFrequency` exposed as `--frequency standard|fast|fast-plus`
  CLI arg.

## [0.2.1] — 2025-03-15

### Fixed

- Bumped library dependency for latest fixes.
