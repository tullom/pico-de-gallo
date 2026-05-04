# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **hardware**: v1.1 landing board — single keyed 2×12 (0.1″ pitch)
  shrouded header replacing the seven individual connectors of v1.0.
  Routes all 20 firmware signals (UART, SPI CS, 1-Wire, ADC now
  connected). On-board passives: 4.7 kΩ I²C pull-ups (R1/R2), 100 Ω ADC
  series resistors (R3–R5), 100 nF decoupling capacitor (C1). VREF pin
  hardwired to 3.3 V. Uses `hw-rev2` firmware.

- **internal**: `GetDeviceInfo` endpoint (`"device/info"`), `DeviceInfo` struct,
  `Capabilities` bitflag newtype (`u64`) with named constants (`I2C`, `SPI`,
  `UART`, `GPIO`, `PWM`, `ADC`, `ONEWIRE`). Schema version constants
  auto-generated from `Cargo.toml` via `build.rs`.
- **internal**: `Unsupported` variant added to `UartError`, `AdcError`, and
  `OneWireError` for endpoints not available on the current hardware revision.
- **firmware**: `hw-rev1` and `hw-rev2` Cargo feature flags (mutually exclusive).
  `hw-rev1` is the default and matches the current v1 landing board.
  Unsupported peripherals (UART, ADC, 1-Wire on v1) return `Unsupported`
  errors instead of silently failing.
- **firmware**: `device_info_handler` returning firmware version, schema version,
  hardware revision, and capabilities bitfield. Capabilities are gated by
  hardware revision feature flag.
- **lib**: `device_info()` and `validate()` methods, `ValidateError` enum.
  Re-exported `Capabilities` and `DeviceInfo`.
- **ffi**: `gallo_get_device_info()` function, `GalloDeviceInfo` C struct with
  `capabilities` u64 bitfield, `GALLO_CAP_*` constants. 4 new status codes:
  `DeviceInfoFailed` (−62), `SchemaMismatch` (−63), `LegacyFirmware` (−64),
  `Unsupported` (−65).
- **app**: `gallo version` now shows schema version, HW revision, and
  capabilities with graceful fallback for legacy firmware.
- **book**: Updated FFI docs with device info section, C constants, and
  `Unsupported` status code. Added hardware revision notes to getting-started
  pin map, UART, ADC, and 1-Wire pages.
- **ci**: `nostd.yml` now builds and lints firmware for both `hw-rev1` and
  `hw-rev2`. `release-firmware.yml` produces per-revision release assets
  (`firmware-rev1.uf2`, `firmware-rev2.uf2`).
- **ci**: `pyco-de-gallo` is now part of the `check.yml` matrix
  (fmt, clippy, doc, hack, test, msrv) on equal footing with the
  other host crates.

### Changed

- **firmware/release**: Renamed firmware crate package from
  `pico-de-gallo-fw` to `pico-de-gallo-firmware` (matches the
  directory name). The release ELF asset uploaded by
  `release-firmware.yml` is now
  `pico-de-gallo-firmware-{rev1,rev2}` (was
  `pico-de-gallo-fw-{rev1,rev2}`). The `firmware-{rev1,rev2}.uf2`
  artifact name is unchanged.
- **internal**: `UartGetConfigurationResponse` changed from `UartConfigurationInfo`
  to `Result<UartConfigurationInfo, UartError>`. `AdcGetConfigurationResponse`
  changed from `AdcConfigurationInfo` to `Result<AdcConfigurationInfo, AdcError>`.
- **lib**: `uart_get_config()` now returns `PicoDeGalloError<UartError>` (was
  `PicoDeGalloError<Infallible>`). `adc_get_config()` now returns
  `PicoDeGalloError<AdcError>` (was `PicoDeGalloError<Infallible>`).

## [0.8.0] — 2026-04-22

**Crate versions:** internal 0.4.0, lib 0.4.0, hal 0.4.0, ffi 0.5.0, app 0.5.0, firmware 0.8.0

### Breaking Changes

- **firmware/internal**: Reduced GPIO count from 8 (GPIO 8–15) to 4
  (GPIO 8–11). GPIO 12–15 are now reserved for PWM output. All GPIO
  indices are now 0–3 instead of 0–7.
- **internal**: Replaced 12 unit-struct error types (`I2cReadFail`, `SpiWriteFail`,
  etc.) with 3 rich error enums: `I2cError` (7 variants), `SpiError` (2 variants),
  `GpioError` (2 variants). Wire protocol is **not** backward compatible —
  firmware and host must be upgraded together.
- **internal**: `GpioError` now has 4 variants — added `PinMonitored` and
  `PinNotMonitored` for the GPIO event subscription system.
- **firmware**: `gpios` field in `Context` changed from `[Flex<'static>; NUM_GPIOS]`
  to `[Option<Flex<'static>>; NUM_GPIOS]`. GPIO operations on a monitored pin
  return `GpioError::PinMonitored`.
- **lib**: All method return types updated from `PicoDeGalloError<*Fail>` to
  `PicoDeGalloError<I2cError>`, `PicoDeGalloError<SpiError>`, or
  `PicoDeGalloError<GpioError>`.
- **hal**: Single `Error` type replaced with `I2cHalError`, `SpiHalError`, and
  `GpioHalError` — each wraps the endpoint-specific error plus a `Comms` variant.
  I2C `ErrorKind` mapping now returns accurate variants (NoAcknowledge,
  ArbitrationLoss, Bus, Overrun) instead of `Other` for all errors.
- **ffi**: Added 8 new status codes (`I2cNack`, `I2cBusError`,
  `I2cArbitrationLoss`, `I2cOverrun`, `BufferTooLong`, `I2cAddressOutOfRange`,
  `GpioInvalidPin`, `CommsFailed`).
- **firmware**: I2C handlers now map embassy-rp `AbortReason` variants to rich
  error types. SPI `set-config` validates frequency before applying (prevents
  panic on zero frequency).
- **hal**: `I2c::transaction()` and `SpiDevice::transaction()` now use batch
  endpoints under the hood — one USB round-trip per transaction instead of
  one per operation. This is a behavioral change: previously each operation
  in a transaction was an independent USB transfer.

### Added

- **internal**: `GpioEventTopic` (device-to-host topic), `GpioEdge` enum
  (Rising/Falling/Any), `GpioEvent` struct (pin, edge, timestamp_us),
  `GpioSubscribe`/`GpioUnsubscribe` endpoints with request/response types.
  `TOPICS_OUT_LIST` now contains the GPIO event topic.
- **firmware**: GPIO event monitoring via 4 pooled `gpio_monitor_task` instances.
  Subscribe takes ownership of the pin, monitors for edges, and publishes
  `GpioEvent` via `Sender::publish`. Unsubscribe returns the pin to the
  context. Static channels for start/stop/return/armed synchronization.
- **lib**: `gpio_subscribe(pin, edge)`, `gpio_unsubscribe(pin)`, and
  `subscribe_gpio_events(depth)` methods. Re-exported `GpioEdge`, `GpioEvent`,
  `IoClosed`, `MultiSubscription`.
- **hal**: `gpio_subscribe(pin, edge)` and `gpio_unsubscribe(pin)` blocking
  methods. Re-exported `GpioEdge`, `GpioEvent`.
- **ffi**: `gallo_gpio_subscribe(pin, edge)` and `gallo_gpio_unsubscribe(pin)`
  FFI functions. 4 new status codes: `GpioPinMonitored` (-54),
  `GpioPinNotMonitored` (-55), `GpioSubscribeFailed` (-56),
  `GpioUnsubscribeFailed` (-57).
- **app**: `gallo gpio monitor --pin N --edge rising|falling|any` command.
  Subscribes, prints edge events with timestamps, unsubscribes on Ctrl+C.
- **internal**: `I2cBatch` and `SpiBatch` endpoints, `I2cBatchOp`/`SpiBatchOp`
  enums, `I2cBatchRequest`/`SpiBatchRequest`/`I2cBatchError`/`SpiBatchError`
  types, `encode_i2c_batch_ops`/`encode_spi_batch_ops` helpers,
  `i2c_batch_response_len`/`spi_batch_response_len`/`count_i2c_batch_ops`/
  `count_spi_batch_ops` parsing helpers. Constants: `MAX_BATCH_OPS`,
  `BATCH_OP_READ`, `BATCH_OP_WRITE`, `BATCH_OP_TRANSFER`, `BATCH_OP_DELAY_NS`.
- **firmware**: `i2c_batch_handler` and `spi_batch_handler` with pre-validation,
  CS assertion/deassertion for SPI batches. SPI batch executes atomically
  under chip-select.
- **lib**: `i2c_batch(address, ops)` and `spi_batch(cs, ops)` async methods.
  Re-exported `I2cBatchOp`, `SpiBatchOp`, `encode_i2c_batch_ops`,
  `encode_spi_batch_ops`, `I2cBatchError`, `SpiBatchError`.
- **hal**: `I2c::transaction()` and `SpiDevice::transaction()` (blocking and
  async) rewritten to use batch endpoints — 10–50× fewer USB round-trips for
  multi-operation transactions.
- **app**: `gallo i2c batch` and `gallo spi batch` CLI commands for executing
  batched operations (e.g., `--op write:0x00,0x10 --op read:16`).
- **internal**: 6 PWM endpoints (`pwm/set-duty-cycle`, `pwm/get-duty-cycle`,
  `pwm/enable`, `pwm/disable`, `pwm/set-config`, `pwm/get-config`), `PwmError`
  enum (4 variants), request/response types, `PwmDutyCycleInfo` and
  `PwmConfigurationInfo` structs, `NUM_PWM_CHANNELS` constant.
- **firmware**: PWM output on GPIO 12–15 (PWM slices 6–7, 4 channels).
  Frequency/phase-correct configuration with automatic top/divider computation.
  Duty-cycle compare values scaled proportionally when frequency changes.
- **lib**: `pwm_set_duty_cycle`, `pwm_get_duty_cycle`, `pwm_enable`,
  `pwm_disable`, `pwm_set_config`, `pwm_get_config` async methods.
  Re-exported `PwmError`, `PwmDutyCycleInfo`, `PwmConfigurationInfo`.
- **hal**: `PwmChannel` wrapper implementing `embedded_hal::pwm::SetDutyCycle`.
  `PwmHalError` type. `Hal::pwm_channel(n)` accessor. `pwm_set_config` and
  `pwm_get_config` convenience methods on `Hal`.
- **ffi**: 6 PWM FFI functions (`gallo_pwm_set_duty_cycle`,
  `gallo_pwm_get_duty_cycle`, `gallo_pwm_enable`, `gallo_pwm_disable`,
  `gallo_pwm_set_config`, `gallo_pwm_get_config`) and 9 status codes (-41 to -49).
- **app**: `gallo pwm` subcommand group with `set-duty`, `get-duty`, `enable`,
  `disable`, `set-config`, and `get-config` commands.
- **internal**: 2 ADC endpoints (`adc/read`,
  `adc/get-config`), `AdcChannel` enum (4 variants: Adc0–Adc3),
  `AdcError` enum (2 variants), `AdcReadRequest` and `AdcConfigurationInfo`
  types. Constants: `NUM_ADC_GPIO_CHANNELS`, `ADC_RESOLUTION_BITS`,
  `ADC_NOMINAL_REFERENCE_MV`.
- **firmware**: ADC support on GPIO 26–29 (4 GPIO channels).
  Uses `Adc::new_blocking` for single-shot reads.
- **lib**: `adc_read(channel)`, `adc_get_config()`
  methods. Re-exported `AdcChannel`, `AdcError`, `AdcConfigurationInfo`.
- **hal**: `AdcHalError` type. `Hal::adc_read(channel)`,
  `adc_get_config()` convenience methods.
- **ffi**: 2 ADC FFI functions (`gallo_adc_read`,
  `gallo_adc_get_config`) and 4 status codes (-50 to -53).
- **app**: `gallo adc` subcommand group with `read` and
  `info` commands.
- **internal**: 6 1-Wire endpoints (`onewire/reset`, `onewire/read`, `onewire/write`,
  `onewire/write-pullup`, `onewire/search`, `onewire/search-next`), `OneWireError`
  enum (4 variants), `OneWireReadRequest`, `OneWireWriteRequest`,
  `OneWireWritePullupRequest` types. Response type aliases with `use-std` feature
  gating for `onewire/read`.
- **firmware**: 1-Wire support via PIO0/SM0 on GPIO 16 using embassy-rp's
  `PioOneWire` driver. 6 async handlers. ROM search state held in Context.
- **lib**: `onewire_reset()`, `onewire_read(len)`, `onewire_write(data)`,
  `onewire_write_pullup(data, duration_ms)`, `onewire_search()`,
  `onewire_search_next()` methods. Re-exported `OneWireError`.
- **hal**: `OneWire` handle struct with blocking wrappers. `OneWireHalError` type.
  `Hal::onewire()` accessor.
- **ffi**: 5 1-Wire FFI functions (`gallo_onewire_reset`, `gallo_onewire_read`,
  `gallo_onewire_write`, `gallo_onewire_write_pullup`, `gallo_onewire_search`)
  and 5 status codes (-57 to -61).
- **app**: `gallo onewire` subcommand group with `reset`, `read`, `write`,
  `write-pullup`, and `search` commands.
- **book**: New "1-Wire Bus" chapter with DS18B20 temperature sensor examples
  (CLI, Rust, C, HAL).
- **internal**: 5 UART endpoints (`uart/read`, `uart/write`, `uart/flush`,
  `uart/set-config`, `uart/get-config`), `UartError` enum (7 variants),
  `UartReadRequest`, `UartWriteRequest`, `UartSetConfigurationRequest`, and
  `UartConfigurationInfo` types. Response type aliases with `use-std` feature
  gating for owned vs borrowed data.
- **firmware**: UART0 support via `BufferedUart` (interrupt-driven, 1024-byte
  TX/RX buffers). 5 UART handlers with timeout support on reads. Baud rate
  validation (must be > 0). Uses GPIO0 (TX) and GPIO1 (RX).
- **lib**: `uart_read(count, timeout_ms)`, `uart_write(contents)`,
  `uart_flush()`, `uart_set_config(baud_rate)`, `uart_get_config()` methods.
  Re-exported `UartError` and `UartConfigurationInfo`.
- **hal**: `Uart` wrapper struct implementing `embedded_io::Read`,
  `embedded_io::Write`, `embedded_io_async::Read`, and
  `embedded_io_async::Write`. `UartHalError` type with `embedded_io::Error`
  impl. `Hal::uart()` accessor with 1000ms default timeout.
- **ffi**: 5 UART FFI functions (`gallo_uart_read`, `gallo_uart_write`,
  `gallo_uart_flush`, `gallo_uart_set_config`, `gallo_uart_get_config`) and
  10 status codes (-31 to -40).
- **app**: `gallo uart` subcommand group with `read`, `write`, `flush`,
  `set-config`, and `get-config` commands.
- **internal**: `I2cScan` endpoint and `I2cScanRequest` type for firmware-side bus
  scanning. Returns a `Vec<u8>` of responding addresses — a single USB
  round-trip replaces 112 individual reads.
- **firmware**: `i2c_scan_handler` — probes addresses by 1-byte read, collects
  responding addresses. Supports `include_reserved` flag.
- **lib**: `PicoDeGallo::i2c_scan(include_reserved)` method returning `Vec<u8>`.
- **hal**: `Hal::i2c_scan(include_reserved)` method returning `Vec<u8>`.
- **ffi**: `gallo_i2c_scan()` function (writes responding addresses to caller
  buffer) and `I2cScanFailed` status code.
- **app**: `gallo i2c scan` now uses the dedicated scan endpoint (single round-trip)
  instead of 112 individual reads.
- **hal**: `SpiDev` type implementing both `embedded_hal::spi::SpiDevice` and
  `embedded_hal_async::spi::SpiDevice`. Manages chip-select (CS) via a GPIO pin,
  asserting CS low before operations and deasserting high afterward with
  automatic flush. Created via `Hal::spi_device(cs_pin)`.
- **internal**: `GpioDirection` and `GpioPull` enums, `GpioSetConfigurationRequest`,
  `GpioSetConfigurationResponse`, and `GpioSetConfiguration` endpoint for runtime
  GPIO pin direction and pull-resistor configuration.
- **internal**: `GpioError::WrongDirection` variant — returned when a get/wait is
  attempted on a pin configured as output, or a put on a pin configured as input.
- **firmware**: `gpio_set_config_handler` and per-pin `PinMode` tracking. Once a
  pin is configured via `gpio/set-config`, it enters explicit mode and
  get/put/wait respect the configured direction (returns `WrongDirection` on
  mismatch). Legacy auto-switching preserved for unconfigured pins.
- **lib**: `PicoDeGallo::gpio_set_config(pin, direction, pull)` method;
  re-exported `GpioDirection` and `GpioPull`.
- **hal**: `Gpio::set_config(pin, direction, pull)` method.
- **ffi**: `gallo_gpio_set_config()` function and `GpioSetConfigFailed` /
  `GpioWrongDirection` status codes.
- **app**: `gallo gpio set-config`, `gallo gpio get`, and `gallo gpio put`
  subcommands for direct GPIO access from the command line.
- **internal**: `I2cGetConfiguration` and `SpiGetConfiguration` endpoints with
  `SpiConfigurationInfo` struct for querying the active bus configuration without
  relying on local state.
- **firmware**: `i2c_get_config_handler` and `spi_get_config_handler` — return
  the currently active configuration. Firmware now tracks config values set by
  `set-config` endpoints.
- **lib**: `PicoDeGallo::i2c_get_config()` and `spi_get_config()` methods;
  re-exported `SpiConfigurationInfo`.
- **hal**: `Hal::i2c_get_config()` and `spi_get_config()` methods.
- **ffi**: `gallo_i2c_get_config()` and `gallo_spi_get_config()` functions,
  `I2cGetConfigFailed` and `SpiGetConfigFailed` status codes.
- **app**: `gallo i2c get-config` and `gallo spi get-config` subcommands.

### Fixed

- **lib**: Corrected `MAX_TRANSFER_SIZE` references in rustdoc for `i2c_read`,
  `i2c_write_read`, and `spi_read` (was 512, actual value is 4096)

## [0.7.0] — 2025-04-20

### Breaking Changes

- **internal 0.3.0**: Split `SetConfigurationRequest` into `I2cSetConfigurationRequest`
  and `SpiSetConfigurationRequest`
- **internal 0.3.0**: Replaced raw `u32` I2C frequency with `I2cFrequency` enum
  (`Standard`, `Fast`, `FastPlus`)
- **lib 0.3.0**: Split `set_config()` into `i2c_set_config()` and `spi_set_config()`
- **lib 0.3.0**: `PicoDeGalloError` is now generic over the endpoint error type
- **hal 0.3.0**: Split `set_config()` into `i2c_set_config()` and `spi_set_config()`
- **ffi 0.4.0**: Split `gallo_set_config()` into `gallo_i2c_set_config()` and
  `gallo_spi_set_config()`
- **app 0.4.0**: CLI `set-config` command replaced by `i2c set-config` and
  `spi set-config` subcommands
- **firmware 0.7.0**: Wire protocol updated — firmware and host must be upgraded together

### Added

- **spi**: Full-duplex transfer endpoint (`spi/transfer`) using DMA
- **lib**: `list_devices()` function for enumerating connected boards
- **app**: `list` command to show connected devices with serial numbers
- **lib**: `Display` and `std::error::Error` implementations for `PicoDeGalloError`
- **internal**: `From<bool>` / `Into<bool>` conversions for `GpioState`
- **internal**: `MAX_TRANSFER_SIZE` constant (4096 bytes) shared across crates
- **ffi**: Compile-time `Send + Sync` assertion for thread safety
- **hal**: Per-call async context detection (reuses existing tokio runtime if available)
- **docs**: Comprehensive rustdoc documentation across all crates
- **docs**: Repository-level Copilot instructions (`.github/copilot-instructions.md`)
- **ci**: Fixed Windows release asset naming (`.dll` extension)

### Changed

- **firmware**: Handler functions modernized with improved ergonomics
- **firmware**: Buffer increased to `MAX_TRANSFER_SIZE` (4096 bytes)
- **firmware**: `PacketBuffers` sized to `MAX_TRANSFER_SIZE + 1024` per direction
- **lib**: `client` field made private (was accidentally public)
- **app**: `I2cFrequency` exposed as `--frequency standard|fast|fast-plus` CLI arg

## [firmware-v0.6.0] — 2025-03-15

### Added

- Updated all Embassy and postcard-rpc dependencies
- Addressed critical safety issues and improved API ergonomics
- Added more tests and extracted `connect()` helper

## [application-v0.2.1] — 2025-03-15

### Fixed

- Bumped library dependency for latest fixes

## [ffi-v0.3.0] — 2025-03-15

### Changed

- Updated dependencies to match library changes

## [hal-v0.2.0] — 2025-03-15

### Changed

- Updated dependencies and API to match library

---

[Unreleased]: https://github.com/OpenDevicePartnership/pico-de-gallo/compare/firmware-v0.8.0...HEAD
[0.8.0]: https://github.com/OpenDevicePartnership/pico-de-gallo/compare/firmware-v0.7.0...firmware-v0.8.0
[0.7.0]: https://github.com/OpenDevicePartnership/pico-de-gallo/compare/firmware-v0.6.0...firmware-v0.7.0
[firmware-v0.6.0]: https://github.com/OpenDevicePartnership/pico-de-gallo/releases/tag/firmware-v0.6.0
[application-v0.2.1]: https://github.com/OpenDevicePartnership/pico-de-gallo/releases/tag/application-v0.2.1
[ffi-v0.3.0]: https://github.com/OpenDevicePartnership/pico-de-gallo/releases/tag/ffi-v0.3.0
[hal-v0.2.0]: https://github.com/OpenDevicePartnership/pico-de-gallo/releases/tag/hal-v0.2.0
