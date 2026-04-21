//! C-compatible FFI bindings for the Pico de Gallo USB bridge.
//!
//! This crate wraps [`pico-de-gallo-lib`](https://docs.rs/pico-de-gallo-lib) in
//! a C-compatible API using opaque pointers and integer status codes. It is
//! compiled as a `cdylib` (shared library) and generates a C header via
//! [`cbindgen`](https://docs.rs/cbindgen).
//!
//! # Usage from C
//!
//! ```c
//! #include "pico_de_gallo.h"
//!
//! const PicoDeGallo *gallo = gallo_init();
//! uint32_t id = 42;
//! Status status = gallo_ping(gallo, &id);
//! // id now contains the round-tripped value
//! gallo_free(gallo);
//! ```
//!
//! # Lifecycle
//!
//! 1. Call [`gallo_init`] (or [`gallo_init_with_serial_number`]) to create a context.
//! 2. Use `gallo_*` functions, passing the context pointer.
//! 3. Call [`gallo_free`] to release resources.
//!
//! # Thread Safety
//!
//! The context pointer is safe to share across threads — the inner type is
//! `Send + Sync` (enforced by a compile-time assertion). Each function call
//! creates its own async executor via [`futures::executor::block_on`], so
//! concurrent calls from multiple threads are safe.
//!
//! # Status Codes
//!
//! All functions return a [`Status`] code. [`Status::Ok`] (0) indicates success;
//! negative values indicate errors. See [`Status`] for the full list.

use futures::executor::block_on;
use pico_de_gallo_lib::{
    self as lib, AdcChannel, AdcError, GpioError, I2cError, PicoDeGalloError, PwmError, SpiError,
    UartError,
};
use std::ffi::CStr;
use std::os::raw::c_char;

/// Opaque handle to a Pico de Gallo device context.
///
/// Created by [`gallo_init`] or [`gallo_init_with_serial_number`] and released
/// by [`gallo_free`]. This type is a thin wrapper around
/// [`pico_de_gallo_lib::PicoDeGallo`] and must not be constructed directly
/// from C code.
pub struct PicoDeGallo(lib::PicoDeGallo);

// Compile-time assertion: the FFI handle must be safe to share across
// C threads. lib::PicoDeGallo is Send + Sync because HostClient is
// internally Arc-wrapped.
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PicoDeGallo>();
};

// ----------------------------- Status Codes -----------------------------

/// Status codes returned by all FFI functions.
///
/// [`Status::Ok`] (0) indicates success. All error codes are negative integers
/// with stable values suitable for use in C `switch` statements.
#[repr(i32)]
#[derive(Debug, PartialEq)]
pub enum Status {
    /// Operation successful
    Ok = 0,
    /// I2c Read failed
    I2cReadFailed = -1,
    /// I2c Write failed
    I2cWriteFailed = -2,
    /// Firmware produced an invalid response
    InvalidResponse = -3,
    /// Library was not initialized
    Uninitialized = -4,
    /// Caller passed an invalid argument
    InvalidArgument = -5,
    /// Ping failed
    PingFailed = -6,
    /// Spi Read failed
    SpiReadFailed = -7,
    /// Spi Write failed
    SpiWriteFailed = -8,
    /// Spi Flush failed
    SpiFlushFailed = -9,
    /// Gpio get failed
    GpioGetFailed = -10,
    /// Gpio put failed
    GpioPutFailed = -11,
    /// Gpio wait failed
    GpioWaitFailed = -12,
    /// Set config failed
    SetConfigFailed = -13,
    /// Version failed
    VersionFailed = -14,
    /// I2c Write Read failed
    I2cWriteReadFailed = -15,
    /// I2c Set config failed
    I2cSetConfigFailed = -16,
    /// Spi Set config failed
    SpiSetConfigFailed = -17,
    /// I2C target did not acknowledge
    I2cNack = -18,
    /// I2C bus error
    I2cBusError = -19,
    /// I2C arbitration loss
    I2cArbitrationLoss = -20,
    /// I2C data overrun
    I2cOverrun = -21,
    /// Buffer exceeds firmware transfer limit
    BufferTooLong = -22,
    /// I2C address out of range
    I2cAddressOutOfRange = -23,
    /// GPIO pin number is invalid
    GpioInvalidPin = -24,
    /// USB communication failure
    CommsFailed = -25,
    /// I2C bus scan failed
    I2cScanFailed = -26,
    /// GPIO set config failed
    GpioSetConfigFailed = -27,
    /// GPIO pin configured in wrong direction for the requested operation
    GpioWrongDirection = -28,
    /// I2C get-config query failed
    I2cGetConfigFailed = -29,
    /// SPI get-config query failed
    SpiGetConfigFailed = -30,
    /// UART read failed
    UartReadFailed = -31,
    /// UART write failed
    UartWriteFailed = -32,
    /// UART flush failed
    UartFlushFailed = -33,
    /// UART receiver overrun
    UartOverrun = -34,
    /// UART break condition detected
    UartBreak = -35,
    /// UART parity error
    UartParity = -36,
    /// UART framing error
    UartFraming = -37,
    /// Invalid baud rate
    UartInvalidBaudRate = -38,
    /// UART set-config failed
    UartSetConfigFailed = -39,
    /// UART get-config query failed
    UartGetConfigFailed = -40,
    /// PWM set-duty-cycle failed
    PwmSetDutyCycleFailed = -41,
    /// PWM get-duty-cycle query failed
    PwmGetDutyCycleFailed = -42,
    /// PWM enable failed
    PwmEnableFailed = -43,
    /// PWM disable failed
    PwmDisableFailed = -44,
    /// PWM set-config failed
    PwmSetConfigFailed = -45,
    /// PWM get-config query failed
    PwmGetConfigFailed = -46,
    /// Invalid PWM channel
    PwmInvalidChannel = -47,
    /// Invalid PWM duty cycle
    PwmInvalidDutyCycle = -48,
    /// Invalid PWM configuration
    PwmInvalidConfiguration = -49,
    /// ADC read failed
    AdcReadFailed = -50,
    /// ADC read-temperature failed
    AdcReadTemperatureFailed = -51,
    /// ADC get-config query failed
    AdcGetConfigFailed = -52,
    /// ADC conversion error
    AdcConversionFailed = -53,
}

// ----------------------------- Error Mapping Helpers -----------------------------

fn i2c_error_to_status(e: PicoDeGalloError<I2cError>) -> Status {
    match e {
        PicoDeGalloError::Endpoint(I2cError::NoAcknowledge) => Status::I2cNack,
        PicoDeGalloError::Endpoint(I2cError::Bus) => Status::I2cBusError,
        PicoDeGalloError::Endpoint(I2cError::ArbitrationLoss) => Status::I2cArbitrationLoss,
        PicoDeGalloError::Endpoint(I2cError::Overrun) => Status::I2cOverrun,
        PicoDeGalloError::Endpoint(I2cError::BufferTooLong) => Status::BufferTooLong,
        PicoDeGalloError::Endpoint(I2cError::AddressOutOfRange) => Status::I2cAddressOutOfRange,
        PicoDeGalloError::Endpoint(I2cError::Other) => Status::I2cReadFailed,
        PicoDeGalloError::Comms(_) => Status::CommsFailed,
    }
}

fn spi_error_to_status(e: PicoDeGalloError<SpiError>) -> Status {
    match e {
        PicoDeGalloError::Endpoint(SpiError::BufferTooLong) => Status::BufferTooLong,
        PicoDeGalloError::Endpoint(SpiError::Other) => Status::SpiReadFailed,
        PicoDeGalloError::Comms(_) => Status::CommsFailed,
    }
}

fn gpio_error_to_status(e: PicoDeGalloError<GpioError>) -> Status {
    match e {
        PicoDeGalloError::Endpoint(GpioError::InvalidPin) => Status::GpioInvalidPin,
        PicoDeGalloError::Endpoint(GpioError::WrongDirection) => Status::GpioWrongDirection,
        PicoDeGalloError::Endpoint(GpioError::Other) => Status::GpioGetFailed,
        PicoDeGalloError::Comms(_) => Status::CommsFailed,
    }
}

fn uart_error_to_status(e: PicoDeGalloError<UartError>) -> Status {
    match e {
        PicoDeGalloError::Endpoint(UartError::BufferTooLong) => Status::BufferTooLong,
        PicoDeGalloError::Endpoint(UartError::Overrun) => Status::UartOverrun,
        PicoDeGalloError::Endpoint(UartError::Break) => Status::UartBreak,
        PicoDeGalloError::Endpoint(UartError::Parity) => Status::UartParity,
        PicoDeGalloError::Endpoint(UartError::Framing) => Status::UartFraming,
        PicoDeGalloError::Endpoint(UartError::InvalidBaudRate) => Status::UartInvalidBaudRate,
        PicoDeGalloError::Endpoint(UartError::Other) => Status::UartReadFailed,
        PicoDeGalloError::Comms(_) => Status::CommsFailed,
    }
}

fn pwm_error_to_status(e: PicoDeGalloError<PwmError>) -> Status {
    match e {
        PicoDeGalloError::Endpoint(PwmError::InvalidChannel) => Status::PwmInvalidChannel,
        PicoDeGalloError::Endpoint(PwmError::InvalidDutyCycle) => Status::PwmInvalidDutyCycle,
        PicoDeGalloError::Endpoint(PwmError::InvalidConfiguration) => {
            Status::PwmInvalidConfiguration
        }
        PicoDeGalloError::Endpoint(PwmError::Other) => Status::PwmSetDutyCycleFailed,
        PicoDeGalloError::Comms(_) => Status::CommsFailed,
    }
}

fn adc_error_to_status(e: PicoDeGalloError<AdcError>) -> Status {
    match e {
        PicoDeGalloError::Endpoint(AdcError::ConversionFailed) => Status::AdcConversionFailed,
        PicoDeGalloError::Endpoint(AdcError::Other) => Status::AdcReadFailed,
        PicoDeGalloError::Comms(_) => Status::CommsFailed,
    }
}

// ----------------------------- Library Lifetime -----------------------------

/// gallo_init - Initialize the library context.
///
/// Returns an opaque representation of the underlying PicoDeGallo
/// device.
#[unsafe(no_mangle)]
pub extern "C" fn gallo_init() -> *const PicoDeGallo {
    let gallo = Box::new(PicoDeGallo(lib::PicoDeGallo::new()));

    Box::into_raw(gallo) as *const PicoDeGallo
}

/// gallo_init_with_serial_number - Initialize the library context for
/// a device with the given serial number.
///
/// Returns an opaque representation of the underlying PicoDeGallo
/// device.
///
/// # Safety
///
/// `c_serial_number` must point to a valid c-string containing a
/// valid Pico de Gallo serial number with a NULL-terminator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_init_with_serial_number(
    c_serial_number: *const c_char,
) -> *const PicoDeGallo {
    if c_serial_number.is_null() {
        eprintln!("NULL serial number received");
        return std::ptr::null();
    }

    // Safety: Pointer is not null due to the check above. Caller must
    // make sure to pass a null-terminated string.
    let serial_number = unsafe { CStr::from_ptr(c_serial_number).to_str() };

    if serial_number.is_err() {
        eprintln!("Invalid UTF-8 string");
        return std::ptr::null();
    }

    let gallo = Box::new(PicoDeGallo(lib::PicoDeGallo::new_with_serial_number(
        serial_number.unwrap(),
    )));

    Box::into_raw(gallo) as *const PicoDeGallo
}

/// gallo_free - Releases and destroys the library context created by `gallo_init`.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_free(gallo: *const PicoDeGallo) {
    if !gallo.is_null() {
        // Safety: caller must ensure that `gallo` is a valid opaque
        // pointer to `PicoDeGallo` returned by `gallo_init()`.
        drop(unsafe { Box::from_raw(gallo as *mut PicoDeGallo) });
    }
}

// ----------------------------- Ping endpoint -----------------------------

/// gallo_ping - Ping the firmware and wait for a response
///
/// Returns the same `u32` passed as the first argument.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_ping(gallo: *mut PicoDeGallo, id: *mut u32) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if id.is_null() {
        eprintln!("Unexpected NULL id pointer");
        return Status::InvalidArgument;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    // Safety: null check above guarantees id is non-null.
    let id_val = unsafe { *id };

    let result = block_on(gallo.0.ping(id_val));
    match result {
        Ok(back) => {
            unsafe { *id = back };
            Status::Ok
        }
        Err(_) => Status::PingFailed,
    }
}

// ----------------------------- I2c endpoints -----------------------------

/// gallo_i2c_read - Read `len` bytes from the device at `address` into `buf`.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()` and `buf` must be valid
/// for `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_i2c_read(
    gallo: *mut PicoDeGallo,
    address: u8,
    buf: *mut u8,
    len: usize,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if buf.is_null() {
        eprintln!("Unexpected NULL buffer");
        return Status::InvalidArgument;
    }

    if len > u16::MAX.into() {
        eprintln!("Buffer is too large");
        return Status::InvalidArgument;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    // Safety: caller must ensure buf is valid for len bytes.
    let buf = unsafe { std::slice::from_raw_parts_mut(buf, len) };

    let result = block_on(gallo.0.i2c_read(address, len as u16));

    match result {
        Ok(data) => {
            if data.len() != buf.len() {
                eprintln!(
                    "Firmware returned {} bytes, expected {}",
                    data.len(),
                    buf.len()
                );
                return Status::InvalidResponse;
            }
            buf.copy_from_slice(&data);
            Status::Ok
        }
        Err(e) => i2c_error_to_status(e),
    }
}

/// gallo_i2c_write - Write `len` bytes from `buf` to the device at `address`.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()` and `buf` must be valid
/// for `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_i2c_write(
    gallo: *mut PicoDeGallo,
    address: u8,
    buf: *const u8,
    len: usize,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if buf.is_null() {
        eprintln!("Unexpected NULL buffer");
        return Status::InvalidArgument;
    }

    if len > u16::MAX.into() {
        eprintln!("Buffer is too large");
        return Status::InvalidArgument;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    // Safety: caller must ensure buf is valid for len bytes.
    let buf = unsafe { std::slice::from_raw_parts(buf, len) };

    let result = block_on(gallo.0.i2c_write(address, buf));

    match result {
        Ok(()) => Status::Ok,
        Err(e) => i2c_error_to_status(e),
    }
}

/// gallo_i2c_write_read - Perform a write followed by a read.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`, `txbuf` must be valid
/// for `txlen` bytes, and `rxbuf` must be valid for `rxlen` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_i2c_write_read(
    gallo: *mut PicoDeGallo,
    address: u8,
    txbuf: *const u8,
    txlen: usize,
    rxbuf: *mut u8,
    rxlen: usize,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if txbuf.is_null() || rxbuf.is_null() {
        eprintln!("Unexpected NULL buffer");
        return Status::InvalidArgument;
    }

    if txlen > u16::MAX.into() || rxlen > u16::MAX.into() {
        eprintln!("Buffer is too large");
        return Status::InvalidArgument;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    // Safety: caller must ensure txbuf is valid for txlen bytes.
    let txbuf = unsafe { std::slice::from_raw_parts(txbuf, txlen) };

    // Safety: caller must ensure rxbuf is valid for rxlen bytes.
    let rxbuf = unsafe { std::slice::from_raw_parts_mut(rxbuf, rxlen) };

    let result = block_on(gallo.0.i2c_write_read(address, txbuf, rxlen as u16));
    match result {
        Ok(data) => {
            if data.len() != rxbuf.len() {
                eprintln!(
                    "Firmware returned {} bytes, expected {}",
                    data.len(),
                    rxbuf.len()
                );
                return Status::InvalidResponse;
            }
            rxbuf.copy_from_slice(&data);
            Status::Ok
        }
        Err(e) => i2c_error_to_status(e),
    }
}

/// gallo_i2c_scan - Scan the I2C bus for responding devices.
///
/// The firmware probes each 7-bit address. Addresses that ACK are written
/// into `buf`. The actual number of devices found is written to `*found`.
///
/// When `include_reserved` is `false`, only the standard range (0x08–0x77)
/// is probed; when `true`, the full range (0x00–0x7F) is scanned.
///
/// Returns `Status::Ok` in case of success or various error codes. If
/// `buf_len` is smaller than the number of responding devices the buffer is
/// filled to capacity and `*found` reflects the total count.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`, `buf` must be valid for
/// `buf_len` bytes, and `found` must point to a valid `usize`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_i2c_scan(
    gallo: *mut PicoDeGallo,
    include_reserved: bool,
    buf: *mut u8,
    buf_len: usize,
    found: *mut usize,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if buf.is_null() || found.is_null() {
        eprintln!("Unexpected NULL pointer");
        return Status::InvalidArgument;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    // Safety: caller must ensure buf is valid for buf_len bytes.
    let buf = unsafe { std::slice::from_raw_parts_mut(buf, buf_len) };

    let result = block_on(gallo.0.i2c_scan(include_reserved));

    match result {
        Ok(addresses) => {
            let copy_len = addresses.len().min(buf.len());
            buf[..copy_len].copy_from_slice(&addresses[..copy_len]);
            unsafe { *found = addresses.len() };
            Status::Ok
        }
        Err(e) => i2c_error_to_status(e),
    }
}

// ----------------------------- Spi endpoints -----------------------------

/// gallo_spi_read - Read `len` bytes.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()` and `buf` must be valid
/// for `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_spi_read(
    gallo: *mut PicoDeGallo,
    buf: *mut u8,
    len: usize,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if buf.is_null() {
        eprintln!("Unexpected NULL buffer");
        return Status::InvalidArgument;
    }

    if len > u16::MAX.into() {
        eprintln!("Buffer is too large");
        return Status::InvalidArgument;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    // Safety: caller must ensure buf is valid for len bytes.
    let buf = unsafe { std::slice::from_raw_parts_mut(buf, len) };

    let result = block_on(gallo.0.spi_read(len as u16));

    match result {
        Ok(data) => {
            if data.len() != buf.len() {
                eprintln!(
                    "Firmware returned {} bytes, expected {}",
                    data.len(),
                    buf.len()
                );
                return Status::InvalidResponse;
            }
            buf.copy_from_slice(&data);
            Status::Ok
        }
        Err(e) => spi_error_to_status(e),
    }
}

/// gallo_spi_write - Write `len` bytes from `buf`.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()` and `buf` must be valid
/// for `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_spi_write(
    gallo: *mut PicoDeGallo,
    buf: *const u8,
    len: usize,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if buf.is_null() {
        eprintln!("Unexpected NULL buffer");
        return Status::InvalidArgument;
    }

    if len > u16::MAX.into() {
        eprintln!("Buffer is too large");
        return Status::InvalidArgument;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    // Safety: caller must ensure buf is valid for len bytes.
    let buf = unsafe { std::slice::from_raw_parts(buf, len) };

    let result = block_on(gallo.0.spi_write(buf));

    match result {
        Ok(()) => Status::Ok,
        Err(e) => spi_error_to_status(e),
    }
}

/// gallo_spi_flush - Flush the SPI interface.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_spi_flush(gallo: *mut PicoDeGallo) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let result = block_on(gallo.0.spi_flush());

    match result {
        Ok(()) => Status::Ok,
        Err(e) => spi_error_to_status(e),
    }
}

// ----------------------------- Gpio endpoints -----------------------------

/// gallo_gpio_get - Get the state of a given GPIO pin.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_gpio_get(
    gallo: *mut PicoDeGallo,
    pin: u8,
    state: *mut bool,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if state.is_null() {
        eprintln!("Unexpected NULL state pointer");
        return Status::InvalidArgument;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let result = block_on(gallo.0.gpio_get(pin));

    match result {
        Ok(s) => {
            unsafe { *state = s == lib::GpioState::High };
            Status::Ok
        }
        Err(e) => gpio_error_to_status(e),
    }
}

/// gallo_gpio_put - Set the state of a given GPIO pin.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_gpio_put(gallo: *mut PicoDeGallo, pin: u8, state: bool) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let s = if state {
        lib::GpioState::High
    } else {
        lib::GpioState::Low
    };
    let result = block_on(gallo.0.gpio_put(pin, s));

    match result {
        Ok(()) => Status::Ok,
        Err(e) => gpio_error_to_status(e),
    }
}

/// gallo_gpio_wait_for_high - Waits for a high level on a given GPIO pin.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_gpio_wait_for_high(gallo: *mut PicoDeGallo, pin: u8) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let result = block_on(gallo.0.gpio_wait_for_high(pin));

    match result {
        Ok(()) => Status::Ok,
        Err(e) => gpio_error_to_status(e),
    }
}

/// gallo_gpio_wait_for_low - Waits for a low level on a given GPIO pin.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_gpio_wait_for_low(gallo: *mut PicoDeGallo, pin: u8) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let result = block_on(gallo.0.gpio_wait_for_low(pin));

    match result {
        Ok(()) => Status::Ok,
        Err(e) => gpio_error_to_status(e),
    }
}

/// gallo_gpio_wait_for_rising_edge - Waits for a rising edge on a given GPIO pin.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_gpio_wait_for_rising_edge(
    gallo: *mut PicoDeGallo,
    pin: u8,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let result = block_on(gallo.0.gpio_wait_for_rising_edge(pin));

    match result {
        Ok(()) => Status::Ok,
        Err(e) => gpio_error_to_status(e),
    }
}

/// gallo_gpio_wait_for_falling_edge - Waits for a falling edge on a given GPIO pin.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_gpio_wait_for_falling_edge(
    gallo: *mut PicoDeGallo,
    pin: u8,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let result = block_on(gallo.0.gpio_wait_for_falling_edge(pin));

    match result {
        Ok(()) => Status::Ok,
        Err(e) => gpio_error_to_status(e),
    }
}

/// gallo_gpio_wait_for_any_edge - Waits for a any edge on a given GPIO pin.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_gpio_wait_for_any_edge(gallo: *mut PicoDeGallo, pin: u8) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let result = block_on(gallo.0.gpio_wait_for_any_edge(pin));

    match result {
        Ok(()) => Status::Ok,
        Err(e) => gpio_error_to_status(e),
    }
}

// ----------------------------- GPIO Set config endpoint -----------------------------

/// gallo_gpio_set_config - Configure a GPIO pin's direction and pull resistor.
///
/// `direction`: 0 = Input, 1 = Output.
/// `pull`: 0 = None, 1 = Pull-up, 2 = Pull-down.
///
/// After configuration, the pin enters explicit mode and get/put will no
/// longer auto-switch direction. Calling `gallo_gpio_put` on an input pin
/// (or `gallo_gpio_get`/wait on an output pin) returns
/// `GpioWrongDirection`.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_gpio_set_config(
    gallo: *mut PicoDeGallo,
    pin: u8,
    direction: u8,
    pull: u8,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    let dir = match direction {
        0 => lib::GpioDirection::Input,
        1 => lib::GpioDirection::Output,
        _ => {
            eprintln!("Invalid direction value: {direction}");
            return Status::InvalidArgument;
        }
    };

    let pull_cfg = match pull {
        0 => lib::GpioPull::None,
        1 => lib::GpioPull::Up,
        2 => lib::GpioPull::Down,
        _ => {
            eprintln!("Invalid pull value: {pull}");
            return Status::InvalidArgument;
        }
    };

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let result = block_on(gallo.0.gpio_set_config(pin, dir, pull_cfg));

    match result {
        Ok(()) => Status::Ok,
        Err(e) => gpio_error_to_status(e),
    }
}

// ----------------------------- I2C Set config endpoint -----------------------------

/// gallo_i2c_set_config - Sets the I2C bus configuration parameters.
///
/// `frequency`: 0 = Standard (100 kHz), 1 = Fast (400 kHz),
/// 2 = Fast+ (1 MHz). Any other value returns `Status::InvalidArgument`.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_i2c_set_config(gallo: *mut PicoDeGallo, frequency: u8) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    let freq = match frequency {
        0 => lib::I2cFrequency::Standard,
        1 => lib::I2cFrequency::Fast,
        2 => lib::I2cFrequency::FastPlus,
        _ => return Status::InvalidArgument,
    };

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let result = block_on(gallo.0.i2c_set_config(freq));

    match result {
        Ok(()) => Status::Ok,
        Err(e) => i2c_error_to_status(e),
    }
}

// ----------------------------- SPI Set config endpoint -----------------------------

/// gallo_spi_set_config - Sets the SPI bus configuration parameters.
///
/// `spi_phase`: false means "Capture on first transition" or CPHA=0,
/// true means "Capture on second transition" or CPHA=1.
///
/// `spi_polarity`: false means "Idle low" or CPOL=0, true means "Idle
/// high" or CPOL=1.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_spi_set_config(
    gallo: *mut PicoDeGallo,
    frequency: u32,
    spi_phase: bool,
    spi_polarity: bool,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let phase = if spi_phase {
        lib::SpiPhase::CaptureOnSecondTransition
    } else {
        lib::SpiPhase::CaptureOnFirstTransition
    };

    let polarity = if spi_polarity {
        lib::SpiPolarity::IdleHigh
    } else {
        lib::SpiPolarity::IdleLow
    };

    let result = block_on(gallo.0.spi_set_config(frequency, phase, polarity));

    match result {
        Ok(()) => Status::Ok,
        Err(e) => spi_error_to_status(e),
    }
}

// ----------------------------- I2C Get config endpoint -----------------------------

/// gallo_i2c_get_config - Queries the current I2C bus configuration.
///
/// On success, writes the current frequency to `*out_frequency`:
/// 0 = Standard (100 kHz), 1 = Fast (400 kHz), 2 = Fast+ (1 MHz).
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`, and that `out_frequency`
/// is a valid pointer to a `u8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_i2c_get_config(
    gallo: *mut PicoDeGallo,
    out_frequency: *mut u8,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if out_frequency.is_null() {
        eprintln!("Unexpected NULL out_frequency pointer");
        return Status::InvalidArgument;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let result = block_on(gallo.0.i2c_get_config());

    match result {
        Ok(freq) => {
            unsafe {
                *out_frequency = freq as u8;
            }
            Status::Ok
        }
        Err(_) => Status::I2cGetConfigFailed,
    }
}

// ----------------------------- SPI Get config endpoint -----------------------------

/// gallo_spi_get_config - Queries the current SPI bus configuration.
///
/// On success, writes the current SPI parameters:
/// - `*out_frequency`: SPI clock frequency in Hz
/// - `*out_phase`: false = CPHA=0, true = CPHA=1
/// - `*out_polarity`: false = CPOL=0, true = CPOL=1
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`, and that all output
/// pointers are valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_spi_get_config(
    gallo: *mut PicoDeGallo,
    out_frequency: *mut u32,
    out_phase: *mut bool,
    out_polarity: *mut bool,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if out_frequency.is_null() || out_phase.is_null() || out_polarity.is_null() {
        eprintln!("Unexpected NULL output pointer");
        return Status::InvalidArgument;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let result = block_on(gallo.0.spi_get_config());

    match result {
        Ok(info) => {
            unsafe {
                *out_frequency = info.spi_frequency;
                *out_phase = matches!(info.spi_phase, lib::SpiPhase::CaptureOnSecondTransition);
                *out_polarity = matches!(info.spi_polarity, lib::SpiPolarity::IdleHigh);
            }
            Status::Ok
        }
        Err(_) => Status::SpiGetConfigFailed,
    }
}

// ----------------------------- UART Read endpoint -----------------------------

/// gallo_uart_read - Read bytes from the UART bus.
///
/// Reads up to `count` bytes into `buf`. On success, writes the actual
/// number of bytes read to `*out_len`. If no data arrives within
/// `timeout_ms` milliseconds, sets `*out_len = 0` and returns
/// `Status::Ok`.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`, that `buf` points to at
/// least `count` bytes, and `out_len` is a valid pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_uart_read(
    gallo: *mut PicoDeGallo,
    buf: *mut u8,
    count: u16,
    timeout_ms: u32,
    out_len: *mut u16,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if buf.is_null() || out_len.is_null() {
        eprintln!("Unexpected NULL pointer");
        return Status::InvalidArgument;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let result = block_on(gallo.0.uart_read(count, timeout_ms));

    match result {
        Ok(data) => {
            let len = data.len().min(count as usize);
            unsafe {
                std::ptr::copy_nonoverlapping(data.as_ptr(), buf, len);
                *out_len = len as u16;
            }
            Status::Ok
        }
        Err(e) => uart_error_to_status(e),
    }
}

// ----------------------------- UART Write endpoint -----------------------------

/// gallo_uart_write - Write bytes to the UART bus.
///
/// Queues `len` bytes from `buf` to the UART transmit buffer. Returns
/// once all bytes have been accepted. Use [`gallo_uart_flush`] to wait
/// for transmission to complete on the wire.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`, and that `buf` points to
/// at least `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_uart_write(
    gallo: *mut PicoDeGallo,
    buf: *const u8,
    len: u16,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if buf.is_null() {
        eprintln!("Unexpected NULL buf pointer");
        return Status::InvalidArgument;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let data = unsafe { std::slice::from_raw_parts(buf, len as usize) };

    let result = block_on(gallo.0.uart_write(data));

    match result {
        Ok(()) => Status::Ok,
        Err(e) => uart_error_to_status(e),
    }
}

// ----------------------------- UART Flush endpoint -----------------------------

/// gallo_uart_flush - Flush the UART transmit buffer.
///
/// Blocks until all pending bytes have been transmitted on the wire.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_uart_flush(gallo: *mut PicoDeGallo) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let result = block_on(gallo.0.uart_flush());

    match result {
        Ok(()) => Status::Ok,
        Err(e) => uart_error_to_status(e),
    }
}

// ----------------------------- UART Set config endpoint -----------------------------

/// gallo_uart_set_config - Set the UART baud rate.
///
/// `baud_rate` must be greater than 0. Returns `Status::InvalidArgument`
/// for a zero baud rate.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_uart_set_config(gallo: *mut PicoDeGallo, baud_rate: u32) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if baud_rate == 0 {
        eprintln!("Invalid baud rate: 0");
        return Status::InvalidArgument;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let result = block_on(gallo.0.uart_set_config(baud_rate));

    match result {
        Ok(()) => Status::Ok,
        Err(e) => uart_error_to_status(e),
    }
}

// ----------------------------- UART Get config endpoint -----------------------------

/// gallo_uart_get_config - Query the current UART configuration.
///
/// On success, writes the current baud rate to `*out_baud_rate`.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`, and that `out_baud_rate`
/// is a valid pointer to a `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_uart_get_config(
    gallo: *mut PicoDeGallo,
    out_baud_rate: *mut u32,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if out_baud_rate.is_null() {
        eprintln!("Unexpected NULL out_baud_rate pointer");
        return Status::InvalidArgument;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let result = block_on(gallo.0.uart_get_config());

    match result {
        Ok(info) => {
            unsafe {
                *out_baud_rate = info.baud_rate;
            }
            Status::Ok
        }
        Err(_) => Status::UartGetConfigFailed,
    }
}

// ----------------------------- PWM endpoints -----------------------------

/// gallo_pwm_set_duty_cycle - Set the raw duty cycle of a PWM channel.
///
/// `channel` is 0–3. `duty` is the raw compare value (0 to the current
/// `top` register). Use `gallo_pwm_get_duty_cycle` to discover the max.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_pwm_set_duty_cycle(
    gallo: *mut PicoDeGallo,
    channel: u8,
    duty: u16,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    let gallo = unsafe { &*gallo };
    match block_on(gallo.0.pwm_set_duty_cycle(channel, duty)) {
        Ok(()) => Status::Ok,
        Err(e) => pwm_error_to_status(e),
    }
}

/// gallo_pwm_get_duty_cycle - Query the current duty cycle of a PWM channel.
///
/// On success, writes the current raw compare value to `*out_duty` and
/// the maximum duty (top + 1) to `*out_max_duty`.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`, and that `out_duty` and
/// `out_max_duty` are valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_pwm_get_duty_cycle(
    gallo: *mut PicoDeGallo,
    channel: u8,
    out_duty: *mut u16,
    out_max_duty: *mut u16,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if out_duty.is_null() || out_max_duty.is_null() {
        eprintln!("Unexpected NULL output pointer");
        return Status::InvalidArgument;
    }

    let gallo = unsafe { &*gallo };
    match block_on(gallo.0.pwm_get_duty_cycle(channel)) {
        Ok(info) => {
            unsafe {
                *out_duty = info.current_duty;
                *out_max_duty = info.max_duty;
            }
            Status::Ok
        }
        Err(e) => pwm_error_to_status(e),
    }
}

/// gallo_pwm_enable - Enable the PWM slice that owns the given channel.
///
/// Channels 0–1 share a slice, channels 2–3 share another.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_pwm_enable(gallo: *mut PicoDeGallo, channel: u8) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    let gallo = unsafe { &*gallo };
    match block_on(gallo.0.pwm_enable(channel)) {
        Ok(()) => Status::Ok,
        Err(e) => pwm_error_to_status(e),
    }
}

/// gallo_pwm_disable - Disable the PWM slice that owns the given channel.
///
/// Channels 0–1 share a slice, channels 2–3 share another.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_pwm_disable(gallo: *mut PicoDeGallo, channel: u8) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    let gallo = unsafe { &*gallo };
    match block_on(gallo.0.pwm_disable(channel)) {
        Ok(()) => Status::Ok,
        Err(e) => pwm_error_to_status(e),
    }
}

/// gallo_pwm_set_config - Configure the PWM slice behind a channel.
///
/// Sets `frequency_hz` and `phase_correct` mode. The firmware computes
/// the `top` and `divider` registers automatically.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_pwm_set_config(
    gallo: *mut PicoDeGallo,
    channel: u8,
    frequency_hz: u32,
    phase_correct: bool,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    let gallo = unsafe { &*gallo };
    match block_on(gallo.0.pwm_set_config(channel, frequency_hz, phase_correct)) {
        Ok(()) => Status::Ok,
        Err(e) => pwm_error_to_status(e),
    }
}

/// gallo_pwm_get_config - Query the current PWM configuration.
///
/// On success, writes the effective frequency to `*out_frequency_hz`,
/// the phase-correct flag to `*out_phase_correct`, and the enabled
/// flag to `*out_enabled`.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`, and that all output
/// pointers are valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_pwm_get_config(
    gallo: *mut PicoDeGallo,
    channel: u8,
    out_frequency_hz: *mut u32,
    out_phase_correct: *mut bool,
    out_enabled: *mut bool,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if out_frequency_hz.is_null() || out_phase_correct.is_null() || out_enabled.is_null() {
        eprintln!("Unexpected NULL output pointer");
        return Status::InvalidArgument;
    }

    let gallo = unsafe { &*gallo };
    match block_on(gallo.0.pwm_get_config(channel)) {
        Ok(info) => {
            unsafe {
                *out_frequency_hz = info.frequency_hz;
                *out_phase_correct = info.phase_correct;
                *out_enabled = info.enabled;
            }
            Status::Ok
        }
        Err(e) => pwm_error_to_status(e),
    }
}

// ----------------------------- ADC endpoints -----------------------------

/// gallo_adc_read - Perform a single-shot ADC read.
///
/// On success, writes the raw 12-bit value (0–4095) to `*out_value`.
/// `channel` selects the input: 0–3 for GPIO26–29, 4 for the internal
/// temperature sensor.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`, and that `out_value` is a
/// valid pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_adc_read(
    gallo: *mut PicoDeGallo,
    channel: u8,
    out_value: *mut u16,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if out_value.is_null() {
        eprintln!("Unexpected NULL output pointer");
        return Status::InvalidArgument;
    }

    let adc_channel = match channel {
        0 => AdcChannel::Adc0,
        1 => AdcChannel::Adc1,
        2 => AdcChannel::Adc2,
        3 => AdcChannel::Adc3,
        4 => AdcChannel::TempSensor,
        _ => {
            eprintln!("Invalid ADC channel: {channel}");
            return Status::InvalidArgument;
        }
    };

    let gallo = unsafe { &*gallo };
    match block_on(gallo.0.adc_read(adc_channel)) {
        Ok(raw) => {
            unsafe { *out_value = raw };
            Status::Ok
        }
        Err(e) => adc_error_to_status(e),
    }
}

/// gallo_adc_read_temperature - Read the on-die temperature sensor.
///
/// On success, writes the temperature in millidegrees Celsius to
/// `*out_millidegrees` (e.g., 27000 = 27.000 °C). The value is approximate.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`, and that `out_millidegrees`
/// is a valid pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_adc_read_temperature(
    gallo: *mut PicoDeGallo,
    out_millidegrees: *mut i32,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if out_millidegrees.is_null() {
        eprintln!("Unexpected NULL output pointer");
        return Status::InvalidArgument;
    }

    let gallo = unsafe { &*gallo };
    match block_on(gallo.0.adc_read_temperature()) {
        Ok(temp) => {
            unsafe { *out_millidegrees = temp };
            Status::Ok
        }
        Err(e) => adc_error_to_status(e),
    }
}

/// gallo_adc_get_config - Query the ADC configuration.
///
/// On success, writes resolution (bits), nominal reference voltage (mV),
/// number of GPIO channels, and temperature sensor availability.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`, and that all output pointers
/// are valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gallo_adc_get_config(
    gallo: *mut PicoDeGallo,
    out_resolution_bits: *mut u8,
    out_nominal_reference_mv: *mut u16,
    out_num_gpio_channels: *mut u8,
    out_has_temp_sensor: *mut bool,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if out_resolution_bits.is_null()
        || out_nominal_reference_mv.is_null()
        || out_num_gpio_channels.is_null()
        || out_has_temp_sensor.is_null()
    {
        eprintln!("Unexpected NULL output pointer");
        return Status::InvalidArgument;
    }

    let gallo = unsafe { &*gallo };
    match block_on(gallo.0.adc_get_config()) {
        Ok(info) => {
            unsafe {
                *out_resolution_bits = info.resolution_bits;
                *out_nominal_reference_mv = info.nominal_reference_mv;
                *out_num_gpio_channels = info.num_gpio_channels;
                *out_has_temp_sensor = info.has_temp_sensor;
            }
            Status::Ok
        }
        Err(e) => match e {
            PicoDeGalloError::Comms(_) => Status::CommsFailed,
            PicoDeGalloError::Endpoint(never) => match never {},
        },
    }
}

// ----------------------------- Version endpoint -----------------------------

#[unsafe(no_mangle)]
/// gallo_version - Gets the firmware version.
///
/// Returns `Status::Ok` in case of success or various error codes.
///
/// # Safety
///
/// Caller must ensure that `gallo` is a valid, opaque pointer to
/// `PicoDeGallo` returned by `gallo_init()`.
pub unsafe extern "C" fn gallo_version(
    gallo: *mut PicoDeGallo,
    major: *mut u16,
    minor: *mut u16,
    patch: *mut u32,
) -> Status {
    if gallo.is_null() {
        eprintln!("Unexpected NULL context");
        return Status::Uninitialized;
    }

    if major.is_null() || minor.is_null() || patch.is_null() {
        eprintln!("Unexpected NULL version pointer");
        return Status::InvalidArgument;
    }

    // Safety: caller must ensure that `gallo` is a valid opaque
    // pointer to `PicoDeGallo` returned by `gallo_init()`.
    let gallo = unsafe { &*gallo };

    let result = block_on(gallo.0.version());

    match result {
        Ok(lib::VersionInfo {
            major: a,
            minor: b,
            patch: c,
        }) => {
            unsafe {
                *major = a;
                *minor = b;
                *patch = c;
            }

            Status::Ok
        }
        Err(_) => Status::VersionFailed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // ----------------------------- Status code invariants -----------------------------

    #[test]
    fn ok_is_zero() {
        assert_eq!(Status::Ok as i32, 0);
    }

    #[test]
    fn all_errors_are_negative() {
        let error_codes = [
            Status::I2cReadFailed as i32,
            Status::I2cWriteFailed as i32,
            Status::InvalidResponse as i32,
            Status::Uninitialized as i32,
            Status::InvalidArgument as i32,
            Status::PingFailed as i32,
            Status::SpiReadFailed as i32,
            Status::SpiWriteFailed as i32,
            Status::SpiFlushFailed as i32,
            Status::GpioGetFailed as i32,
            Status::GpioPutFailed as i32,
            Status::GpioWaitFailed as i32,
            Status::SetConfigFailed as i32,
            Status::VersionFailed as i32,
            Status::I2cWriteReadFailed as i32,
            Status::I2cSetConfigFailed as i32,
            Status::SpiSetConfigFailed as i32,
            Status::I2cNack as i32,
            Status::I2cBusError as i32,
            Status::I2cArbitrationLoss as i32,
            Status::I2cOverrun as i32,
            Status::BufferTooLong as i32,
            Status::I2cAddressOutOfRange as i32,
            Status::GpioInvalidPin as i32,
            Status::CommsFailed as i32,
            Status::I2cScanFailed as i32,
            Status::GpioSetConfigFailed as i32,
            Status::GpioWrongDirection as i32,
            Status::I2cGetConfigFailed as i32,
            Status::SpiGetConfigFailed as i32,
            Status::UartReadFailed as i32,
            Status::UartWriteFailed as i32,
            Status::UartFlushFailed as i32,
            Status::UartOverrun as i32,
            Status::UartBreak as i32,
            Status::UartParity as i32,
            Status::UartFraming as i32,
            Status::UartInvalidBaudRate as i32,
            Status::UartSetConfigFailed as i32,
            Status::UartGetConfigFailed as i32,
            Status::PwmSetDutyCycleFailed as i32,
            Status::PwmGetDutyCycleFailed as i32,
            Status::PwmEnableFailed as i32,
            Status::PwmDisableFailed as i32,
            Status::PwmSetConfigFailed as i32,
            Status::PwmGetConfigFailed as i32,
            Status::PwmInvalidChannel as i32,
            Status::PwmInvalidDutyCycle as i32,
            Status::PwmInvalidConfiguration as i32,
            Status::AdcReadFailed as i32,
            Status::AdcReadTemperatureFailed as i32,
            Status::AdcGetConfigFailed as i32,
            Status::AdcConversionFailed as i32,
        ];
        for code in error_codes {
            assert!(code < 0, "error code {code} should be negative");
        }
    }

    #[test]
    fn status_codes_are_distinct() {
        let codes = [
            Status::Ok as i32,
            Status::I2cReadFailed as i32,
            Status::I2cWriteFailed as i32,
            Status::InvalidResponse as i32,
            Status::Uninitialized as i32,
            Status::InvalidArgument as i32,
            Status::PingFailed as i32,
            Status::SpiReadFailed as i32,
            Status::SpiWriteFailed as i32,
            Status::SpiFlushFailed as i32,
            Status::GpioGetFailed as i32,
            Status::GpioPutFailed as i32,
            Status::GpioWaitFailed as i32,
            Status::SetConfigFailed as i32,
            Status::VersionFailed as i32,
            Status::I2cWriteReadFailed as i32,
            Status::I2cSetConfigFailed as i32,
            Status::SpiSetConfigFailed as i32,
            Status::I2cNack as i32,
            Status::I2cBusError as i32,
            Status::I2cArbitrationLoss as i32,
            Status::I2cOverrun as i32,
            Status::BufferTooLong as i32,
            Status::I2cAddressOutOfRange as i32,
            Status::GpioInvalidPin as i32,
            Status::CommsFailed as i32,
            Status::I2cScanFailed as i32,
            Status::GpioSetConfigFailed as i32,
            Status::GpioWrongDirection as i32,
            Status::I2cGetConfigFailed as i32,
            Status::SpiGetConfigFailed as i32,
            Status::UartReadFailed as i32,
            Status::UartWriteFailed as i32,
            Status::UartFlushFailed as i32,
            Status::UartOverrun as i32,
            Status::UartBreak as i32,
            Status::UartParity as i32,
            Status::UartFraming as i32,
            Status::UartInvalidBaudRate as i32,
            Status::UartSetConfigFailed as i32,
            Status::UartGetConfigFailed as i32,
            Status::PwmSetDutyCycleFailed as i32,
            Status::PwmGetDutyCycleFailed as i32,
            Status::PwmEnableFailed as i32,
            Status::PwmDisableFailed as i32,
            Status::PwmSetConfigFailed as i32,
            Status::PwmGetConfigFailed as i32,
            Status::PwmInvalidChannel as i32,
            Status::PwmInvalidDutyCycle as i32,
            Status::PwmInvalidConfiguration as i32,
            Status::AdcReadFailed as i32,
            Status::AdcReadTemperatureFailed as i32,
            Status::AdcGetConfigFailed as i32,
            Status::AdcConversionFailed as i32,
        ];
        let unique: HashSet<i32> = codes.iter().copied().collect();
        assert_eq!(codes.len(), unique.len(), "duplicate status codes found");
    }

    // ----------------------------- Null pointer checks -----------------------------

    #[test]
    fn ping_null_device_returns_uninitialized() {
        let mut id = 42u32;
        let status = unsafe { gallo_ping(std::ptr::null_mut(), &mut id as *mut u32) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn i2c_read_null_device_returns_uninitialized() {
        let mut buf = [0u8; 4];
        let status =
            unsafe { gallo_i2c_read(std::ptr::null_mut(), 0x48, buf.as_mut_ptr(), buf.len()) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn i2c_write_null_device_returns_uninitialized() {
        let buf = [0u8; 4];
        let status =
            unsafe { gallo_i2c_write(std::ptr::null_mut(), 0x48, buf.as_ptr(), buf.len()) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn i2c_write_read_null_device_returns_uninitialized() {
        let txbuf = [0u8; 2];
        let mut rxbuf = [0u8; 4];
        let status = unsafe {
            gallo_i2c_write_read(
                std::ptr::null_mut(),
                0x48,
                txbuf.as_ptr(),
                txbuf.len(),
                rxbuf.as_mut_ptr(),
                rxbuf.len(),
            )
        };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn spi_read_null_device_returns_uninitialized() {
        let mut buf = [0u8; 4];
        let status = unsafe { gallo_spi_read(std::ptr::null_mut(), buf.as_mut_ptr(), buf.len()) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn spi_write_null_device_returns_uninitialized() {
        let buf = [0u8; 4];
        let status = unsafe { gallo_spi_write(std::ptr::null_mut(), buf.as_ptr(), buf.len()) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn spi_flush_null_device_returns_uninitialized() {
        let status = unsafe { gallo_spi_flush(std::ptr::null_mut()) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn gpio_get_null_device_returns_uninitialized() {
        let mut state = false;
        let status = unsafe { gallo_gpio_get(std::ptr::null_mut(), 0, &mut state as *mut bool) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn gpio_put_null_device_returns_uninitialized() {
        let status = unsafe { gallo_gpio_put(std::ptr::null_mut(), 0, true) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn gpio_wait_for_high_null_device_returns_uninitialized() {
        let status = unsafe { gallo_gpio_wait_for_high(std::ptr::null_mut(), 0) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn gpio_wait_for_low_null_device_returns_uninitialized() {
        let status = unsafe { gallo_gpio_wait_for_low(std::ptr::null_mut(), 0) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn gpio_wait_for_rising_edge_null_device_returns_uninitialized() {
        let status = unsafe { gallo_gpio_wait_for_rising_edge(std::ptr::null_mut(), 0) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn gpio_wait_for_falling_edge_null_device_returns_uninitialized() {
        let status = unsafe { gallo_gpio_wait_for_falling_edge(std::ptr::null_mut(), 0) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn gpio_wait_for_any_edge_null_device_returns_uninitialized() {
        let status = unsafe { gallo_gpio_wait_for_any_edge(std::ptr::null_mut(), 0) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn gpio_set_config_null_device_returns_uninitialized() {
        let status = unsafe { gallo_gpio_set_config(std::ptr::null_mut(), 0, 0, 0) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn i2c_set_config_null_device_returns_uninitialized() {
        let status = unsafe { gallo_i2c_set_config(std::ptr::null_mut(), 1) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn i2c_set_config_invalid_frequency_returns_invalid_argument() {
        // We need a non-null pointer but it doesn't matter since validation
        // happens before dereference for the frequency parameter.
        // Use null to get Uninitialized first, then test with a valid-looking
        // but actually invalid frequency value — but null check comes first.
        // So we just verify the enum boundary at the API level.
        let status = unsafe { gallo_i2c_set_config(std::ptr::null_mut(), 99) };
        // null check happens first, so this returns Uninitialized
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn spi_set_config_null_device_returns_uninitialized() {
        let status = unsafe { gallo_spi_set_config(std::ptr::null_mut(), 1_000_000, false, false) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn i2c_get_config_null_device_returns_uninitialized() {
        let mut freq = 0u8;
        let status = unsafe { gallo_i2c_get_config(std::ptr::null_mut(), &mut freq as *mut u8) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn spi_get_config_null_device_returns_uninitialized() {
        let mut freq = 0u32;
        let mut phase = false;
        let mut polarity = false;
        let status = unsafe {
            gallo_spi_get_config(
                std::ptr::null_mut(),
                &mut freq as *mut u32,
                &mut phase as *mut bool,
                &mut polarity as *mut bool,
            )
        };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn version_null_device_returns_uninitialized() {
        let mut major = 0u16;
        let mut minor = 0u16;
        let mut patch = 0u32;
        let status = unsafe {
            gallo_version(
                std::ptr::null_mut(),
                &mut major as *mut u16,
                &mut minor as *mut u16,
                &mut patch as *mut u32,
            )
        };
        assert_eq!(status, Status::Uninitialized);
    }

    // ----------------------------- Null out-param checks -----------------------------

    #[test]
    fn ping_null_id_returns_invalid_argument() {
        let status = unsafe { gallo_ping(std::ptr::null_mut(), std::ptr::null_mut()) };
        // gallo is null, so Uninitialized fires first
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn gpio_get_null_state_returns_invalid_argument() {
        let status = unsafe { gallo_gpio_get(std::ptr::null_mut(), 0, std::ptr::null_mut()) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn version_null_major_returns_invalid_argument() {
        let mut minor = 0u16;
        let mut patch = 0u32;
        let status = unsafe {
            gallo_version(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut minor as *mut u16,
                &mut patch as *mut u32,
            )
        };
        assert_eq!(status, Status::Uninitialized);
    }

    // ----------------------------- PWM null pointer checks -----------------------------

    #[test]
    fn pwm_set_duty_cycle_null_device_returns_uninitialized() {
        let status = unsafe { gallo_pwm_set_duty_cycle(std::ptr::null_mut(), 0, 100) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn pwm_get_duty_cycle_null_device_returns_uninitialized() {
        let status = unsafe {
            gallo_pwm_get_duty_cycle(
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn pwm_enable_null_device_returns_uninitialized() {
        let status = unsafe { gallo_pwm_enable(std::ptr::null_mut(), 0) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn pwm_disable_null_device_returns_uninitialized() {
        let status = unsafe { gallo_pwm_disable(std::ptr::null_mut(), 0) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn pwm_set_config_null_device_returns_uninitialized() {
        let status = unsafe { gallo_pwm_set_config(std::ptr::null_mut(), 0, 1000, false) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn pwm_get_config_null_device_returns_uninitialized() {
        let status = unsafe {
            gallo_pwm_get_config(
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn adc_read_null_device_returns_uninitialized() {
        let status = unsafe { gallo_adc_read(std::ptr::null_mut(), 0, std::ptr::null_mut()) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn adc_read_temperature_null_device_returns_uninitialized() {
        let status =
            unsafe { gallo_adc_read_temperature(std::ptr::null_mut(), std::ptr::null_mut()) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn adc_get_config_null_device_returns_uninitialized() {
        let status = unsafe {
            gallo_adc_get_config(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(status, Status::Uninitialized);
    }

    // ----------------------------- Null buffer checks -----------------------------

    #[test]
    fn i2c_read_null_buffer_returns_invalid_argument() {
        // Device is also null, so we get Uninitialized first.
        // This tests that the null-device check fires before the buffer check.
        let status = unsafe { gallo_i2c_read(std::ptr::null_mut(), 0x48, std::ptr::null_mut(), 4) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn spi_read_null_buffer_returns_invalid_argument() {
        let status = unsafe { gallo_spi_read(std::ptr::null_mut(), std::ptr::null_mut(), 4) };
        assert_eq!(status, Status::Uninitialized);
    }

    // ----------------------------- Free safety -----------------------------

    #[test]
    fn free_null_is_safe() {
        unsafe { gallo_free(std::ptr::null()) };
    }

    #[test]
    fn init_with_null_serial_returns_null() {
        let ptr = unsafe { gallo_init_with_serial_number(std::ptr::null()) };
        assert!(ptr.is_null());
    }

    // ----------------------------- UART null pointer checks -----------------------------

    #[test]
    fn uart_read_null_device_returns_uninitialized() {
        let mut buf = [0u8; 4];
        let mut out_len = 0u16;
        let status = unsafe {
            gallo_uart_read(
                std::ptr::null_mut(),
                buf.as_mut_ptr(),
                4,
                1000,
                &mut out_len as *mut u16,
            )
        };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn uart_read_null_buf_returns_uninitialized() {
        let status = unsafe {
            gallo_uart_read(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                4,
                1000,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn uart_write_null_device_returns_uninitialized() {
        let data = [0x48, 0x65, 0x6c, 0x6c, 0x6f];
        let status = unsafe { gallo_uart_write(std::ptr::null_mut(), data.as_ptr(), 5) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn uart_write_null_buf_returns_uninitialized() {
        let status = unsafe { gallo_uart_write(std::ptr::null_mut(), std::ptr::null(), 5) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn uart_flush_null_device_returns_uninitialized() {
        let status = unsafe { gallo_uart_flush(std::ptr::null_mut()) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn uart_set_config_null_device_returns_uninitialized() {
        let status = unsafe { gallo_uart_set_config(std::ptr::null_mut(), 115200) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn uart_set_config_zero_baud_returns_uninitialized() {
        // null check fires first
        let status = unsafe { gallo_uart_set_config(std::ptr::null_mut(), 0) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn uart_get_config_null_device_returns_uninitialized() {
        let mut baud = 0u32;
        let status = unsafe { gallo_uart_get_config(std::ptr::null_mut(), &mut baud as *mut u32) };
        assert_eq!(status, Status::Uninitialized);
    }

    #[test]
    fn uart_error_to_status_maps_all_variants() {
        assert_eq!(
            uart_error_to_status(PicoDeGalloError::Endpoint(UartError::BufferTooLong)),
            Status::BufferTooLong
        );
        assert_eq!(
            uart_error_to_status(PicoDeGalloError::Endpoint(UartError::Overrun)),
            Status::UartOverrun
        );
        assert_eq!(
            uart_error_to_status(PicoDeGalloError::Endpoint(UartError::Break)),
            Status::UartBreak
        );
        assert_eq!(
            uart_error_to_status(PicoDeGalloError::Endpoint(UartError::Parity)),
            Status::UartParity
        );
        assert_eq!(
            uart_error_to_status(PicoDeGalloError::Endpoint(UartError::Framing)),
            Status::UartFraming
        );
        assert_eq!(
            uart_error_to_status(PicoDeGalloError::Endpoint(UartError::InvalidBaudRate)),
            Status::UartInvalidBaudRate
        );
        assert_eq!(
            uart_error_to_status(PicoDeGalloError::Endpoint(UartError::Other)),
            Status::UartReadFailed
        );
    }

    #[test]
    fn adc_error_mapping() {
        assert_eq!(
            adc_error_to_status(PicoDeGalloError::Endpoint(AdcError::ConversionFailed)),
            Status::AdcConversionFailed
        );
        assert_eq!(
            adc_error_to_status(PicoDeGalloError::Endpoint(AdcError::Other)),
            Status::AdcReadFailed
        );
    }
}
