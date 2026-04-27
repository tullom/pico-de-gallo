//! Pyco de Gallo — Python bindings for [Pico de
//! Gallo](https://github.com/OpenDevicePartnership/pico-de-gallo).
//!
//! This module exposes the host-side `pico_de_gallo_lib` API to Python via
//! [PyO3](https://pyo3.rs). It supports I2C, SPI, UART, GPIO, PWM, ADC, and
//! 1-Wire operations against a Pico de Gallo USB bridge device.
//!
//! All methods are synchronous from Python's perspective: the underlying
//! async calls are driven by an internal Tokio runtime owned by each
//! `PycoDeGallo` instance.
//!
//! Example:
//!
//! ```python
//! import pyco_de_gallo as gallo
//!
//! dev = gallo.open()
//! v = dev.version()
//! print(f"Firmware {v.major}.{v.minor}.{v.patch}")
//!
//! dev.i2c_set_config(gallo.I2cFrequency.Fast)
//! data = dev.i2c_write_read(0x50, [0x00], 16)
//! ```
//!
//! Errors from the underlying library are surfaced as Python `RuntimeError`
//! exceptions.

use pico_de_gallo_lib::{
    AdcChannel, GpioDirection, GpioPull, GpioState,
    I2cBatchOp as LibI2cBatchOp, I2cFrequency as LibI2cFrequency, PicoDeGallo,
    SpiBatchOp as LibSpiBatchOp, SpiPhase as LibSpiPhase, SpiPolarity as LibSpiPolarity,
};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use tokio::runtime::Runtime;

/// List all Pico de Gallo devices currently connected to the system.
///
/// Returns:
///     list[DeviceDescription]: One entry per matching USB device. Use the
///     ``serial_number`` field with :func:`open_with_serial_number` to connect
///     to a specific board.
#[pyfunction]
fn list_devices() -> PyResult<Vec<DeviceDescription>> {
    let list = pico_de_gallo_lib::list_devices()
        .into_iter()
        .map(|dev| DeviceDescription {
            serial_number: dev.serial_number,
            manufacturer: dev.manufacturer,
            product: dev.product,
        })
        .collect::<Vec<_>>();
    Ok(list)
}

/// Open the first Pico de Gallo device available on the system.
///
/// If multiple devices are connected, this picks the first one matched by
/// the OS USB enumeration. Use :func:`open_with_serial_number` for
/// deterministic selection.
///
/// Returns:
///     PycoDeGallo: A connected device handle.
///
/// Raises:
///     RuntimeError: If a Tokio runtime could not be created.
#[pyfunction]
fn open() -> PyResult<PycoDeGallo> {
    let runtime = Runtime::new().map_err(|e| PyRuntimeError::new_err(format!("{e}")))?;
    let gallo = {
        let _guard = runtime.enter();
        PicoDeGallo::new()
    };
    Ok(PycoDeGallo {
        inner: gallo,
        runtime,
    })
}

/// Open the Pico de Gallo device with the given USB serial number.
///
/// Args:
///     serial_number (str): The USB serial number of the target device, as
///         reported by :func:`list_devices`.
///
/// Returns:
///     PycoDeGallo: A connected device handle.
///
/// Raises:
///     RuntimeError: If a Tokio runtime could not be created.
#[pyfunction]
fn open_with_serial_number(serial_number: &str) -> PyResult<PycoDeGallo> {
    let runtime = Runtime::new().map_err(|e| PyRuntimeError::new_err(format!("{e}")))?;
    let gallo = {
        let _guard = runtime.enter();
        PicoDeGallo::new_with_serial_number(serial_number)
    };
    Ok(PycoDeGallo {
        inner: gallo,
        runtime,
    })
}

/// Python bindings for the Pico de Gallo USB bridge.
#[pymodule]
fn pyco_de_gallo(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(list_devices, m)?)?;
    m.add_function(wrap_pyfunction!(open, m)?)?;
    m.add_function(wrap_pyfunction!(open_with_serial_number, m)?)?;

    Ok(())
}

/// SPI clock phase (when the bus samples data relative to the clock edge).
#[pyclass(from_py_object)]
#[derive(Clone)]
enum SpiPhase {
    /// Sample on the first clock transition (CPHA = 0).
    CaptureOnFirstTransition = 0,
    /// Sample on the second clock transition (CPHA = 1).
    CaptureOnSecondTransition = 1,
}

/// SPI clock polarity (the idle level of the SCK line).
#[pyclass(from_py_object)]
#[derive(Clone)]
enum SpiPolarity {
    /// Clock idles low (CPOL = 0).
    IdleLow = 0,
    /// Clock idles high (CPOL = 1).
    IdleHigh = 1,
}

/// Supported I2C bus frequencies. Discriminant values are the bus rate in Hz.
#[pyclass(from_py_object)]
#[derive(Clone)]
enum I2cFrequency {
    /// Standard mode — 100 kHz.
    Standard = 100_000,
    /// Fast mode — 400 kHz.
    Fast = 400_000,
    /// Fast-Plus mode — 1 MHz.
    FastPlus = 1_000_000,
}

/// A single operation in an I2C batch sequence.
///
/// Pass a list of these to :meth:`PycoDeGallo.i2c_batch` to execute multiple
/// reads/writes in a single USB round-trip.
#[pyclass(from_py_object)]
#[derive(Clone)]
enum I2cBatchOp {
    /// Read ``read_count`` bytes from the target address.
    Read { read_count: u16 },
    /// Write ``write_data`` to the target address.
    Write { write_data: Vec<u8> },
}

/// A single operation in an SPI batch sequence.
///
/// Pass a list of these to :meth:`PycoDeGallo.spi_batch` to execute a
/// multi-step SPI transaction atomically under chip-select.
#[pyclass(from_py_object)]
#[derive(Clone)]
enum SpiBatchOp {
    /// Read ``read_count`` bytes from the SPI bus.
    Read { read_count: u16 },
    /// Write ``write_data`` to the SPI bus.
    Write { write_data: Vec<u8> },
    /// Full-duplex transfer: send ``write_data`` and receive the same number of bytes.
    Transfer { write_data: Vec<u8> },
    /// Insert a delay of ``duration_ns`` nanoseconds between operations.
    DelayNs { duration_ns: u32 },
}

/// USB descriptor of a connected Pico de Gallo device. Returned by
/// :func:`list_devices`.
#[pyclass]
struct DeviceDescription {
    /// USB serial number (unique per board, derived from the chip ID).
    #[pyo3(get)]
    serial_number: Option<String>,
    /// USB manufacturer string.
    #[pyo3(get)]
    manufacturer: Option<String>,
    /// USB product string.
    #[pyo3(get)]
    product: Option<String>,
}

/// Firmware version triple reported by :meth:`PycoDeGallo.version`.
#[pyclass]
struct VersionInfo {
    /// Major version.
    #[pyo3(get)]
    major: u16,
    /// Minor version.
    #[pyo3(get)]
    minor: u16,
    /// Patch version.
    #[pyo3(get)]
    patch: u32,
}

/// Extended device information returned by :meth:`PycoDeGallo.device_info`
/// and :meth:`PycoDeGallo.validate`.
#[pyclass]
struct DeviceInfo {
    /// Firmware major version.
    #[pyo3(get)]
    fw_major: u16,
    /// Firmware minor version.
    #[pyo3(get)]
    fw_minor: u16,
    /// Firmware patch version.
    #[pyo3(get)]
    fw_patch: u32,
    /// Wire protocol schema major version.
    #[pyo3(get)]
    schema_major: u16,
    /// Wire protocol schema minor version. Pre-1.0 bumps are breaking.
    #[pyo3(get)]
    schema_minor: u16,
    /// Wire protocol schema patch version.
    #[pyo3(get)]
    schema_patch: u32,
    /// Hardware revision number.
    #[pyo3(get)]
    hw_version: u8,
    /// Bitfield of supported peripheral capabilities.
    #[pyo3(get)]
    capabilities: u64,
}

/// Current UART configuration returned by :meth:`PycoDeGallo.uart_get_config`.
#[pyclass]
struct UartConfigurationInfo {
    /// Active UART baud rate, in bits per second.
    #[pyo3(get)]
    baud_rate: u32,
}

/// Current SPI configuration returned by :meth:`PycoDeGallo.spi_get_config`.
#[pyclass]
struct SpiConfigurationInfo {
    /// SPI clock frequency, in Hz.
    #[pyo3(get)]
    spi_frequency: u32,
    /// SPI clock phase (CPHA).
    #[pyo3(get)]
    spi_phase: SpiPhase,
    /// SPI clock polarity (CPOL).
    #[pyo3(get)]
    spi_polarity: SpiPolarity,
}

/// PWM duty-cycle information returned by :meth:`PycoDeGallo.pwm_get_duty_cycle`.
#[pyclass]
struct PwmDutyCycleInfo {
    /// Full-scale compare value (the slice's ``top`` register + 1).
    #[pyo3(get)]
    max_duty: u16,
    /// Current raw compare value, in ``0..=max_duty``.
    #[pyo3(get)]
    current_duty: u16,
}

/// PWM slice configuration returned by :meth:`PycoDeGallo.pwm_get_config`.
#[pyclass]
struct PwmConfigurationInfo {
    /// Effective PWM frequency, in Hz.
    #[pyo3(get)]
    frequency_hz: u32,
    /// Whether phase-correct mode is enabled.
    #[pyo3(get)]
    phase_correct: bool,
    /// Whether the slice is currently enabled.
    #[pyo3(get)]
    enabled: bool,
}

/// ADC configuration returned by :meth:`PycoDeGallo.adc_get_config`.
#[pyclass]
struct AdcConfigurationInfo {
    /// ADC sample resolution, in bits (12 on RP2350).
    #[pyo3(get)]
    resolution_bits: u8,
    /// Nominal reference voltage, in millivolts.
    #[pyo3(get)]
    nominal_reference_mv: u16,
    /// Number of GPIO-routed ADC channels available.
    #[pyo3(get)]
    num_gpio_channels: u8,
}

/// A connected Pico de Gallo device handle.
///
/// Construct with :func:`open` or :func:`open_with_serial_number`. All methods
/// on this class are synchronous; the underlying async client is driven by an
/// internal Tokio runtime owned by the instance.
#[pyclass]
struct PycoDeGallo {
    inner: PicoDeGallo,
    runtime: Runtime,
}

#[pymethods]
impl PycoDeGallo {
    /// Block until the underlying USB connection is closed.
    fn wait_closed(&self) -> PyResult<()> {
        let gallo = &self.inner;
        self.runtime.block_on(gallo.wait_closed());
        Ok(())
    }

    /// Echo a 32-bit integer back from the firmware.
    ///
    /// Useful for testing connectivity. The firmware returns the same value
    /// that was sent.
    ///
    /// Args:
    ///     id (int): Arbitrary 32-bit unsigned integer.
    ///
    /// Returns:
    ///     int: The same integer the firmware received.
    fn ping(&self, id: u32) -> PyResult<u32> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.ping(id))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Read ``count`` bytes from the I2C device at ``address``.
    ///
    /// The firmware buffer is limited to 4096 bytes; reads exceeding this
    /// limit will be truncated.
    ///
    /// Args:
    ///     address (int): 7-bit I2C target address.
    ///     count (int): Number of bytes to read.
    ///
    /// Returns:
    ///     bytes: The bytes read from the bus.
    fn i2c_read(&self, address: u8, count: u16) -> PyResult<Vec<u8>> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.i2c_read(address, count))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Write ``data`` to the I2C device at ``address``.
    ///
    /// Args:
    ///     address (int): 7-bit I2C target address.
    ///     data (bytes | list[int]): Bytes to write.
    fn i2c_write(&self, address: u8, data: Vec<u8>) -> PyResult<()> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.i2c_write(address, &data))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Write ``write_data`` to the I2C device at ``address``, then read
    /// back ``read_count`` bytes in a single transaction (without releasing
    /// the bus).
    ///
    /// The firmware buffer is limited to 4096 bytes; reads exceeding this
    /// limit will be truncated.
    ///
    /// Args:
    ///     address (int): 7-bit I2C target address.
    ///     write_data (bytes | list[int]): Bytes to write first.
    ///     read_count (int): Number of bytes to read back.
    ///
    /// Returns:
    ///     bytes: The bytes read from the bus.
    fn i2c_write_read(
        &self,
        address: u8,
        write_data: Vec<u8>,
        read_count: u16,
    ) -> PyResult<Vec<u8>> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.i2c_write_read(address, &write_data, read_count))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Scan the I2C bus and return the addresses of all responding devices.
    ///
    /// Probes each 7-bit address with a 1-byte read. Addresses that ACK are
    /// returned in ascending order.
    ///
    /// Args:
    ///     include_reserved (bool): If False (default-equivalent), only the
    ///         standard range ``0x08..=0x77`` is probed. If True, the full
    ///         range ``0x00..=0x7F`` is scanned.
    ///
    /// Returns:
    ///     list[int]: 7-bit addresses that responded, ascending.
    fn i2c_scan(&self, include_reserved: bool) -> PyResult<Vec<u8>> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.i2c_scan(include_reserved))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Execute a batch of I2C operations in a single USB transfer.
    ///
    /// Much faster than issuing individual I2C calls when performing
    /// multi-step sequences (e.g., EEPROM programming).
    ///
    /// Args:
    ///     address (int): 7-bit I2C target address.
    ///     ops (list[I2cBatchOp]): Sequence of read/write operations.
    ///
    /// Returns:
    ///     bytes: Concatenated read data from all ``Read`` operations, in order.
    fn i2c_batch(&self, address: u8, ops: Vec<I2cBatchOp>) -> PyResult<Vec<u8>> {
        let gallo = &self.inner;
        let ops = ops
            .iter()
            .map(|op| match op {
                I2cBatchOp::Read { read_count } => LibI2cBatchOp::Read { len: *read_count },
                I2cBatchOp::Write { write_data } => LibI2cBatchOp::Write { data: write_data },
            })
            .collect::<Vec<_>>();
        self.runtime
            .block_on(gallo.i2c_batch(address, &ops))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Read ``count`` bytes from the SPI bus.
    ///
    /// The firmware buffer is limited to 4096 bytes; reads exceeding this
    /// limit will be truncated.
    fn spi_read(&self, count: u16) -> PyResult<Vec<u8>> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.spi_read(count))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Write ``data`` to the SPI bus.
    fn spi_write(&self, data: Vec<u8>) -> PyResult<()> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.spi_write(&data))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Flush the SPI interface.
    fn spi_flush(&self) -> PyResult<()> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.spi_flush())
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Perform a full-duplex SPI transfer.
    ///
    /// Simultaneously sends ``write_data`` and receives the same number of
    /// bytes. The firmware buffer is limited to 4096 bytes; transfers
    /// exceeding this limit will be rejected.
    ///
    /// Returns:
    ///     bytes: Bytes received during the transfer.
    fn spi_transfer(&self, write_data: Vec<u8>) -> PyResult<Vec<u8>> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.spi_transfer(&write_data))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Execute a batch of SPI operations atomically under chip-select.
    ///
    /// The firmware asserts CS on ``cs_pin`` before the first operation and
    /// deasserts it after the last (or on error).
    ///
    /// Args:
    ///     cs_pin (int): GPIO pin number to use as chip-select.
    ///     ops (list[SpiBatchOp]): Sequence of read/write/transfer/delay ops.
    ///
    /// Returns:
    ///     bytes: Concatenated data from all ``Read`` and ``Transfer`` ops, in order.
    fn spi_batch(&self, cs_pin: u8, ops: Vec<SpiBatchOp>) -> PyResult<Vec<u8>> {
        let gallo = &self.inner;
        let ops = ops
            .iter()
            .map(|op| match op {
                SpiBatchOp::Read { read_count } => LibSpiBatchOp::Read { len: *read_count },
                SpiBatchOp::Write { write_data } => LibSpiBatchOp::Write { data: write_data },
                SpiBatchOp::Transfer { write_data } => LibSpiBatchOp::Transfer { data: write_data },
                SpiBatchOp::DelayNs { duration_ns } => LibSpiBatchOp::DelayNs { ns: *duration_ns },
            })
            .collect::<Vec<_>>();
        self.runtime
            .block_on(gallo.spi_batch(cs_pin, &ops))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Read up to ``count`` bytes from the UART bus.
    ///
    /// Waits up to ``timeout_ms`` milliseconds for at least one byte. Returns
    /// whatever bytes are available (1 to ``count``), or an empty list on timeout.
    ///
    /// Args:
    ///     count (int): Maximum number of bytes to read.
    ///     timeout_ms (int): Timeout in milliseconds.
    ///
    /// Returns:
    ///     bytes: Bytes received, possibly fewer than ``count``.
    fn uart_read(&self, count: u16, timeout_ms: u32) -> PyResult<Vec<u8>> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.uart_read(count, timeout_ms))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Write ``data`` to the UART bus.
    ///
    /// Returns once all bytes have been queued in the firmware's TX buffer
    /// (not necessarily transmitted on the wire). Use :meth:`uart_flush` to
    /// wait for transmission to complete.
    fn uart_write(&self, data: Vec<u8>) -> PyResult<()> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.uart_write(&data))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Flush the UART transmit buffer.
    ///
    /// Blocks until all pending bytes have been transmitted on the wire.
    fn uart_flush(&self) -> PyResult<()> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.uart_flush())
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Get the current state of the GPIO numbered by ``pin``.
    ///
    /// Pico de Gallo offers 4 total GPIOs, numbered 0 through 3.
    ///
    /// Returns:
    ///     bool: True if the pin reads high, False otherwise.
    fn gpio_get(&self, pin: u8) -> PyResult<bool> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.gpio_get(pin))
            .map(|state| matches!(state, GpioState::High))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Drive the GPIO numbered by ``pin`` to ``state``.
    ///
    /// Pico de Gallo offers 4 total GPIOs, numbered 0 through 3.
    ///
    /// Args:
    ///     pin (int): GPIO pin number (0–3).
    ///     state (bool): True for high, False for low.
    fn gpio_put(&self, pin: u8, state: bool) -> PyResult<()> {
        let gallo = &self.inner;
        let state = state.into();
        self.runtime
            .block_on(gallo.gpio_put(pin, state))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Block until the GPIO numbered by ``pin`` reads high.
    fn gpio_wait_for_high(&self, pin: u8) -> PyResult<()> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.gpio_wait_for_high(pin))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Block until the GPIO numbered by ``pin`` reads low.
    fn gpio_wait_for_low(&self, pin: u8) -> PyResult<()> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.gpio_wait_for_low(pin))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Block until a rising edge is detected on the GPIO numbered by ``pin``.
    fn gpio_wait_for_rising_edge(&self, pin: u8) -> PyResult<()> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.gpio_wait_for_rising_edge(pin))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Block until a falling edge is detected on the GPIO numbered by ``pin``.
    fn gpio_wait_for_falling_edge(&self, pin: u8) -> PyResult<()> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.gpio_wait_for_falling_edge(pin))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Block until any (rising or falling) edge is detected on the GPIO
    /// numbered by ``pin``.
    fn gpio_wait_for_any_edge(&self, pin: u8) -> PyResult<()> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.gpio_wait_for_any_edge(pin))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Configure a GPIO pin's direction and internal pull resistor.
    ///
    /// After configuration, the pin enters explicit mode: :meth:`gpio_get`
    /// and :meth:`gpio_put` will no longer auto-switch direction.
    ///
    /// Args:
    ///     pin (int): GPIO pin number (0–3).
    ///     direction (int): ``0`` = input, anything else = output.
    ///     pull (int): ``0`` = none, ``1`` = pull-up, ``2`` = pull-down.
    fn gpio_set_config(&self, pin: u8, direction: u8, pull: u8) -> PyResult<()> {
        let gallo = &self.inner;
        let direction = match direction {
            0 => GpioDirection::Input,
            _ => GpioDirection::Output,
        };
        let pull = match pull {
            1 => GpioPull::Up,
            2 => GpioPull::Down,
            _ => GpioPull::None,
        };
        self.runtime
            .block_on(gallo.gpio_set_config(pin, direction, pull))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Set the I2C bus clock frequency.
    ///
    /// Takes effect immediately before the next I2C operation.
    ///
    /// Args:
    ///     frequency_hz (I2cFrequency): One of ``Standard`` (100 kHz),
    ///         ``Fast`` (400 kHz), or ``FastPlus`` (1 MHz).
    fn i2c_set_config(&self, frequency_hz: I2cFrequency) -> PyResult<()> {
        let gallo = &self.inner;

        let frequency = match frequency_hz {
            I2cFrequency::Standard => LibI2cFrequency::Standard,
            I2cFrequency::Fast => LibI2cFrequency::Fast,
            I2cFrequency::FastPlus => LibI2cFrequency::FastPlus,
        };

        self.runtime
            .block_on(gallo.i2c_set_config(frequency))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Set the SPI bus clock frequency, phase, and polarity.
    ///
    /// Takes effect immediately before the next SPI operation.
    ///
    /// Args:
    ///     frequency_hz (int): SPI clock frequency in Hz.
    ///     spi_phase (SpiPhase): Clock phase (CPHA).
    ///     spi_polarity (SpiPolarity): Clock polarity (CPOL).
    fn spi_set_config(
        &self,
        frequency_hz: u32,
        spi_phase: SpiPhase,
        spi_polarity: SpiPolarity,
    ) -> PyResult<()> {
        let gallo = &self.inner;

        let spi_phase = match spi_phase {
            SpiPhase::CaptureOnFirstTransition => LibSpiPhase::CaptureOnFirstTransition,
            SpiPhase::CaptureOnSecondTransition => LibSpiPhase::CaptureOnSecondTransition,
        };

        let spi_polarity = match spi_polarity {
            SpiPolarity::IdleLow => LibSpiPolarity::IdleLow,
            SpiPolarity::IdleHigh => LibSpiPolarity::IdleHigh,
        };

        self.runtime
            .block_on(gallo.spi_set_config(frequency_hz, spi_phase, spi_polarity))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Get the firmware version from the connected device.
    ///
    /// Returns:
    ///     VersionInfo: Major / minor / patch version triple.
    fn version(&self) -> PyResult<VersionInfo> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.version())
            .map(|v| VersionInfo {
                major: v.major,
                minor: v.minor,
                patch: v.patch,
            })
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Query extended device information, including firmware version, wire
    /// schema version, hardware revision, and peripheral capabilities.
    ///
    /// Returns:
    ///     DeviceInfo: Device identity and capability snapshot.
    fn device_info(&self) -> PyResult<DeviceInfo> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.device_info())
            .map(|info| DeviceInfo {
                fw_major: info.fw_major,
                fw_minor: info.fw_minor,
                fw_patch: info.fw_patch,
                schema_major: info.schema_major,
                schema_minor: info.schema_minor,
                schema_patch: info.schema_patch,
                hw_version: info.hw_version,
                capabilities: info.capabilities.0,
            })
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Validate that the connected firmware is wire-compatible with this
    /// host library, and return the device info on success.
    ///
    /// Raises:
    ///     RuntimeError: If the firmware is too old (no ``device/info``
    ///         endpoint), if the schema versions disagree, or on a
    ///         communication failure.
    fn validate(&self) -> PyResult<DeviceInfo> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.validate())
            .map(|info| DeviceInfo {
                fw_major: info.fw_major,
                fw_minor: info.fw_minor,
                fw_patch: info.fw_patch,
                schema_major: info.schema_major,
                schema_minor: info.schema_minor,
                schema_patch: info.schema_patch,
                hw_version: info.hw_version,
                capabilities: info.capabilities.0,
            })
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Query the current I2C bus frequency.
    ///
    /// Returns:
    ///     I2cFrequency: The active I2C frequency. Defaults to ``Standard``
    ///     (100 kHz) on firmware boot.
    fn i2c_get_config(&self) -> PyResult<I2cFrequency> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.i2c_get_config())
            .map(|freq| match freq {
                LibI2cFrequency::Standard => I2cFrequency::Standard,
                LibI2cFrequency::Fast => I2cFrequency::Fast,
                LibI2cFrequency::FastPlus => I2cFrequency::FastPlus,
            })
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Query the current SPI bus configuration.
    ///
    /// Returns:
    ///     SpiConfigurationInfo: Active SPI frequency, phase, and polarity.
    ///     Defaults are 1 MHz, ``CaptureOnFirstTransition``, ``IdleLow``.
    fn spi_get_config(&self) -> PyResult<SpiConfigurationInfo> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.spi_get_config())
            .map(|config| SpiConfigurationInfo {
                spi_frequency: config.spi_frequency,
                spi_phase: match config.spi_phase {
                    LibSpiPhase::CaptureOnFirstTransition => SpiPhase::CaptureOnFirstTransition,
                    LibSpiPhase::CaptureOnSecondTransition => SpiPhase::CaptureOnSecondTransition,
                },
                spi_polarity: match config.spi_polarity {
                    LibSpiPolarity::IdleLow => SpiPolarity::IdleLow,
                    LibSpiPolarity::IdleHigh => SpiPolarity::IdleHigh,
                },
            })
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Set the UART baud rate.
    ///
    /// Takes effect immediately before the next UART operation. The default
    /// baud rate is 115200.
    fn uart_set_config(&self, baud_rate: u32) -> PyResult<()> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.uart_set_config(baud_rate))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Query the current UART configuration.
    ///
    /// Returns:
    ///     UartConfigurationInfo: Active baud rate. Default is 115200.
    ///
    /// Raises:
    ///     RuntimeError: If the firmware's hardware revision does not support UART.
    fn uart_get_config(&self) -> PyResult<UartConfigurationInfo> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.uart_get_config())
            .map(|config| UartConfigurationInfo {
                baud_rate: config.baud_rate,
            })
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Set the raw duty cycle of a PWM channel.
    ///
    /// ``duty_cycle`` is a raw compare value in ``0..=max_duty``. Use
    /// :meth:`pwm_get_duty_cycle` to discover the channel's ``max_duty``.
    ///
    /// Channels 0–1 share PWM slice 6, channels 2–3 share PWM slice 7.
    ///
    /// Args:
    ///     channel (int): PWM channel (0–3).
    ///     duty_cycle (int): Raw compare value.
    fn pwm_set_duty_cycle(&self, channel: u8, duty_cycle: u16) -> PyResult<()> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.pwm_set_duty_cycle(channel, duty_cycle))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Query the current duty cycle of a PWM channel.
    ///
    /// Returns:
    ///     PwmDutyCycleInfo: ``current_duty`` (raw compare value) and
    ///     ``max_duty`` (full-scale value, equal to ``top + 1``).
    fn pwm_get_duty_cycle(&self, channel: u8) -> PyResult<PwmDutyCycleInfo> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.pwm_get_duty_cycle(channel))
            .map(|info| PwmDutyCycleInfo {
                max_duty: info.max_duty,
                current_duty: info.current_duty,
            })
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Enable the PWM slice that owns ``channel`` (0–3).
    ///
    /// Because PWM slices drive two channels, enabling channel 0 also enables
    /// channel 1 (and vice versa). Same for channels 2 and 3.
    fn pwm_enable(&self, channel: u8) -> PyResult<()> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.pwm_enable(channel))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Disable the PWM slice that owns ``channel`` (0–3).
    ///
    /// Because PWM slices drive two channels, disabling channel 0 also
    /// disables channel 1 (and vice versa). Same for channels 2 and 3.
    fn pwm_disable(&self, channel: u8) -> PyResult<()> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.pwm_disable(channel))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Configure the PWM slice behind ``channel`` (0–3).
    ///
    /// Sets the output frequency and phase-correct mode. The firmware
    /// computes ``top`` and ``divider`` automatically. Existing duty-cycle
    /// compare values are scaled proportionally to the new ``top``.
    ///
    /// Channels 0–1 share a slice (channels 2–3 likewise), so configuring
    /// one channel also affects its sibling.
    ///
    /// Args:
    ///     channel (int): PWM channel (0–3).
    ///     frequency_hz (int): Desired PWM frequency in Hz.
    ///     phase_correct (bool): Whether to enable phase-correct mode.
    fn pwm_set_config(&self, channel: u8, frequency_hz: u32, phase_correct: bool) -> PyResult<()> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.pwm_set_config(channel, frequency_hz, phase_correct))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Query the current configuration of the PWM slice behind ``channel``.
    ///
    /// Returns:
    ///     PwmConfigurationInfo: Effective frequency, phase-correct flag,
    ///     and enabled state.
    fn pwm_get_config(&self, channel: u8) -> PyResult<PwmConfigurationInfo> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.pwm_get_config(channel))
            .map(|config| PwmConfigurationInfo {
                frequency_hz: config.frequency_hz,
                phase_correct: config.phase_correct,
                enabled: config.enabled,
            })
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Perform a single-shot ADC read on the specified channel.
    ///
    /// Returns a raw 12-bit value (0–4095). Convert to voltage with
    /// ``V ≈ raw * 3.3 / 4096`` (approximate; depends on ADC_AVDD).
    ///
    /// Args:
    ///     channel (int): ADC channel (0–3).
    ///
    /// Raises:
    ///     RuntimeError: If ``channel`` is outside the range 0–3, or if the
    ///         firmware reports an ADC error.
    fn adc_read(&self, channel: u8) -> PyResult<u16> {
        let gallo = &self.inner;
        let channel = match channel {
            0 => AdcChannel::Adc0,
            1 => AdcChannel::Adc1,
            2 => AdcChannel::Adc2,
            3 => AdcChannel::Adc3,
            _ => {
                return Err(PyRuntimeError::new_err(
                    "ADC channel must be between 0 and 3",
                ));
            }
        };
        self.runtime
            .block_on(gallo.adc_read(channel))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Query the ADC configuration (resolution, reference voltage, channel count).
    ///
    /// Returns:
    ///     AdcConfigurationInfo: Fixed values for the RP2350 ADC.
    ///
    /// Raises:
    ///     RuntimeError: If the firmware's hardware revision does not support ADC.
    fn adc_get_config(&self) -> PyResult<AdcConfigurationInfo> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.adc_get_config())
            .map(|config| AdcConfigurationInfo {
                resolution_bits: config.resolution_bits,
                nominal_reference_mv: config.nominal_reference_mv,
                num_gpio_channels: config.num_gpio_channels,
            })
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Perform a 1-Wire bus reset and detect device presence.
    ///
    /// Returns:
    ///     bool: True if one or more devices responded with a presence pulse.
    fn onewire_reset(&self) -> PyResult<bool> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.onewire_reset())
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Read ``len`` bytes from the 1-Wire bus.
    ///
    /// The firmware sends ``0xFF`` read slots and captures the response bits.
    fn onewire_read(&self, len: u16) -> PyResult<Vec<u8>> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.onewire_read(len))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Write raw bytes to the 1-Wire bus.
    fn onewire_write(&self, data: Vec<u8>) -> PyResult<()> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.onewire_write(&data))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Write bytes to the 1-Wire bus, then apply a strong pullup.
    ///
    /// Required for parasitic-power devices like the DS18B20 during
    /// temperature conversion. The bus is held high for
    /// ``pullup_duration_ms`` milliseconds after the last bit is sent.
    fn onewire_write_pullup(&self, data: Vec<u8>, pullup_duration_ms: u16) -> PyResult<()> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.onewire_write_pullup(&data, pullup_duration_ms))
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Start a new 1-Wire ROM search and return the first device address.
    ///
    /// Returns:
    ///     int | None: The first 64-bit ROM ID found, or ``None`` if no
    ///     devices are on the bus. Call :meth:`onewire_search_next` to
    ///     enumerate the rest.
    fn onewire_search(&self) -> PyResult<Option<u64>> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.onewire_search())
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Continue the current 1-Wire ROM search.
    ///
    /// Returns:
    ///     int | None: The next device's 64-bit ROM ID, or ``None`` when
    ///     all devices have been enumerated.
    fn onewire_search_next(&self) -> PyResult<Option<u64>> {
        let gallo = &self.inner;
        self.runtime
            .block_on(gallo.onewire_search_next())
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }
}
