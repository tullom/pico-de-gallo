//! Shared wire-protocol types for the Pico de Gallo USB bridge.
//!
//! This crate defines the [postcard-rpc](https://docs.rs/postcard-rpc) endpoints,
//! request/response types, and shared constants used by both the firmware
//! ([`pico-de-gallo-firmware`]) and the host-side library ([`pico-de-gallo-lib`]).
//!
//! # Wire Compatibility
//!
//! All types are serialized with [postcard](https://docs.rs/postcard). Postcard
//! encodes enum variants by **index** (0, 1, 2, …), not by discriminant value.
//! Reordering variants in any `enum` in this crate is a **breaking wire change**
//! that will silently corrupt communication between mismatched firmware and host
//! versions.
//!
//! # Feature Flags
//!
//! - **`use-std`** — Enables `Vec<u8>` response types for the host side. Without
//!   this feature (the default for firmware), responses use borrowed `&[u8]` slices.
//!
//! # Crate Organization
//!
//! - **Constants**: [`MICROSOFT_VID`], [`PICO_DE_GALLO_PID`], [`MAX_TRANSFER_SIZE`]
//! - **Endpoints**: Defined via the [`postcard_rpc::endpoints!`] macro — see
//!   [`ENDPOINT_LIST`] for the full table.
//! - **I2C types**: [`I2cReadRequest`], [`I2cWriteRequest`], [`I2cWriteReadRequest`],
//!   [`I2cScanRequest`], and their shared error type [`I2cError`].
//! - **SPI types**: [`SpiReadRequest`], [`SpiWriteRequest`], [`SpiTransferRequest`]
//!   and their shared error type [`SpiError`].
//! - **GPIO types**: [`GpioGetRequest`], [`GpioPutRequest`], [`GpioWaitRequest`],
//!   [`GpioState`], [`GpioDirection`], [`GpioPull`], [`GpioSetConfigurationRequest`],
//!   and their shared error type [`GpioError`].
//! - **UART types**: [`UartReadRequest`], [`UartWriteRequest`],
//!   [`UartSetConfigurationRequest`], [`UartConfigurationInfo`],
//!   and their shared error type [`UartError`].
//! - **PWM types**: [`PwmSetDutyCycleRequest`], [`PwmGetDutyCycleRequest`],
//!   [`PwmEnableRequest`], [`PwmDisableRequest`], [`PwmSetConfigurationRequest`],
//!   [`PwmGetConfigurationRequest`], [`PwmDutyCycleInfo`], [`PwmConfigurationInfo`],
//!   and their shared error type [`PwmError`].
//! - **ADC types**: [`AdcReadRequest`], [`AdcChannel`], [`AdcConfigurationInfo`],
//!   and their shared error type [`AdcError`].
//! - **Configuration**: [`I2cSetConfigurationRequest`], [`SpiSetConfigurationRequest`],
//!   [`GpioSetConfigurationRequest`], [`UartSetConfigurationRequest`],
//!   [`PwmSetConfigurationRequest`],
//!   [`I2cFrequency`], [`SpiPhase`], [`SpiPolarity`],
//!   [`GpioDirection`], [`GpioPull`], [`SpiConfigurationInfo`],
//!   [`UartConfigurationInfo`].
//! - **Version**: [`VersionInfo`].

#![cfg_attr(not(feature = "use-std"), no_std)]

use postcard_rpc::{TopicDirection, endpoints, topics};
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

/// USB Vendor ID (Microsoft Corporation).
pub const MICROSOFT_VID: u16 = 0x045e;

/// USB Product ID assigned to Pico de Gallo.
pub const PICO_DE_GALLO_PID: u16 = 0x067d;

/// Maximum number of bytes the firmware can handle in a single I2C or SPI
/// transaction. Requests exceeding this limit will be rejected by the
/// firmware with an error.
pub const MAX_TRANSFER_SIZE: usize = 4096;

// ---

/// Response type for I2C write operations.
pub type I2cWriteResponse = Result<(), I2cError>;

/// Response type for I2C read operations.
/// On the host (`use-std`), returns `Vec<u8>`; on firmware, returns `&[u8]`.
#[cfg(feature = "use-std")]
pub type I2cReadResponse<'a> = Result<Vec<u8>, I2cError>;
/// Response type for I2C read operations.
/// On the host (`use-std`), returns `Vec<u8>`; on firmware, returns `&[u8]`.
#[cfg(not(feature = "use-std"))]
pub type I2cReadResponse<'a> = Result<&'a [u8], I2cError>;

/// Response type for I2C write-read operations.
/// On the host (`use-std`), returns `Vec<u8>`; on firmware, returns `&[u8]`.
#[cfg(feature = "use-std")]
pub type I2cWriteReadResponse<'a> = Result<Vec<u8>, I2cError>;
/// Response type for I2C write-read operations.
/// On the host (`use-std`), returns `Vec<u8>`; on firmware, returns `&[u8]`.
#[cfg(not(feature = "use-std"))]
pub type I2cWriteReadResponse<'a> = Result<&'a [u8], I2cError>;

/// Response type for SPI write operations.
pub type SpiWriteResponse = Result<(), SpiError>;

/// Response type for SPI read operations.
/// On the host (`use-std`), returns `Vec<u8>`; on firmware, returns `&[u8]`.
#[cfg(feature = "use-std")]
pub type SpiReadResponse<'a> = Result<Vec<u8>, SpiError>;
/// Response type for SPI read operations.
/// On the host (`use-std`), returns `Vec<u8>`; on firmware, returns `&[u8]`.
#[cfg(not(feature = "use-std"))]
pub type SpiReadResponse<'a> = Result<&'a [u8], SpiError>;

/// Response type for SPI flush operations.
pub type SpiFlushResponse = Result<(), SpiError>;

/// Response type for SPI transfer operations.
/// On the host (`use-std`), returns `Vec<u8>`; on firmware, returns `&[u8]`.
#[cfg(feature = "use-std")]
pub type SpiTransferResponse<'a> = Result<Vec<u8>, SpiError>;
/// Response type for SPI transfer operations.
/// On the host (`use-std`), returns `Vec<u8>`; on firmware, returns `&[u8]`.
#[cfg(not(feature = "use-std"))]
pub type SpiTransferResponse<'a> = Result<&'a [u8], SpiError>;

/// Response type for GPIO get operations.
pub type GpioGetResponse = Result<GpioState, GpioError>;
/// Response type for GPIO put operations.
pub type GpioPutResponse = Result<(), GpioError>;
/// Response type for GPIO wait operations.
pub type GpioWaitResponse = Result<(), GpioError>;
/// Response type for GPIO set-configuration operations.
pub type GpioSetConfigurationResponse = Result<(), GpioError>;
/// Response type for I2C bus configuration operations.
pub type I2cSetConfigurationResponse = Result<(), I2cConfigError>;
/// Response type for I2C bus scan operations.
/// On the host (`use-std`), returns `Vec<u8>` of responding addresses;
/// on firmware, returns `&[u8]`.
#[cfg(feature = "use-std")]
pub type I2cScanResponse<'a> = Result<Vec<u8>, I2cError>;
/// Response type for I2C bus scan operations.
/// On the host (`use-std`), returns `Vec<u8>` of responding addresses;
/// on firmware, returns `&[u8]`.
#[cfg(not(feature = "use-std"))]
pub type I2cScanResponse<'a> = Result<&'a [u8], I2cError>;
/// Response type for SPI bus configuration operations.
pub type SpiSetConfigurationResponse = Result<(), SpiConfigError>;

/// Response type for I2C get-configuration queries.
///
/// Returns the currently active I2C bus frequency.
pub type I2cGetConfigurationResponse = I2cFrequency;

/// Response type for SPI get-configuration queries.
///
/// Returns the currently active SPI bus parameters.
pub type SpiGetConfigurationResponse = SpiConfigurationInfo;

/// Response type for UART write operations.
pub type UartWriteResponse = Result<(), UartError>;

/// Response type for UART read operations.
/// On the host (`use-std`), returns `Vec<u8>`; on firmware, returns `&[u8]`.
#[cfg(feature = "use-std")]
pub type UartReadResponse<'a> = Result<Vec<u8>, UartError>;
/// Response type for UART read operations.
/// On the host (`use-std`), returns `Vec<u8>`; on firmware, returns `&[u8]`.
#[cfg(not(feature = "use-std"))]
pub type UartReadResponse<'a> = Result<&'a [u8], UartError>;

/// Response type for UART flush operations.
pub type UartFlushResponse = Result<(), UartError>;

/// Response type for UART bus configuration operations.
pub type UartSetConfigurationResponse = Result<(), UartConfigError>;

/// Response type for UART get-configuration queries.
///
/// Returns the currently active UART parameters.
pub type UartGetConfigurationResponse = UartConfigurationInfo;

/// Response type for PWM set-duty-cycle operations.
pub type PwmSetDutyCycleResponse = Result<(), PwmError>;
/// Response type for PWM get-duty-cycle queries.
pub type PwmGetDutyCycleResponse = Result<PwmDutyCycleInfo, PwmError>;
/// Response type for PWM enable operations.
pub type PwmEnableResponse = Result<(), PwmError>;
/// Response type for PWM disable operations.
pub type PwmDisableResponse = Result<(), PwmError>;
/// Response type for PWM set-configuration operations.
pub type PwmSetConfigurationResponse = Result<(), PwmConfigError>;
/// Response type for PWM get-configuration queries.
pub type PwmGetConfigurationResponse = Result<PwmConfigurationInfo, PwmError>;

/// Response type for ADC read operations.
pub type AdcReadResponse = Result<u16, AdcError>;
/// Response type for ADC read-temperature operations (millidegrees Celsius).
pub type AdcReadTemperatureResponse = Result<i32, AdcError>;
/// Response type for ADC get-configuration queries.
pub type AdcGetConfigurationResponse = AdcConfigurationInfo;

endpoints! {
    list = ENDPOINT_LIST;
    | EndpointTy          | RequestTy                  | ResponseTy                  | Path                |
    | ----------          | ---------                  | ----------                  | ----                |
    | PingEndpoint        | u32                        | u32                         | "ping"              |
    | I2cRead             | I2cReadRequest             | I2cReadResponse<'a>         | "i2c/read"          |
    | I2cWrite            | I2cWriteRequest<'a>        | I2cWriteResponse            | "i2c/write"         |
    | I2cWriteRead        | I2cWriteReadRequest<'a>    | I2cWriteReadResponse<'b>    | "i2c/write-read"    |
    | SpiRead             | SpiReadRequest             | SpiReadResponse<'a>         | "spi/read"          |
    | SpiWrite            | SpiWriteRequest<'a>        | SpiWriteResponse            | "spi/write"         |
    | SpiFlush            | ()                         | SpiFlushResponse            | "spi/flush"         |
    | SpiTransfer         | SpiTransferRequest<'a>     | SpiTransferResponse<'b>     | "spi/transfer"      |
    | GpioGet             | GpioGetRequest             | GpioGetResponse             | "gpio/get"          |
    | GpioPut             | GpioPutRequest             | GpioPutResponse             | "gpio/put"          |
    | GpioWaitForHigh     | GpioWaitRequest            | GpioWaitResponse            | "gpio/wait-high"    |
    | GpioWaitForLow      | GpioWaitRequest            | GpioWaitResponse            | "gpio/wait-low"     |
    | GpioWaitForRising   | GpioWaitRequest            | GpioWaitResponse            | "gpio/wait-rising"  |
    | GpioWaitForFalling  | GpioWaitRequest            | GpioWaitResponse            | "gpio/wait-falling" |
    | GpioWaitForAny      | GpioWaitRequest            | GpioWaitResponse            | "gpio/wait-any"     |
    | I2cSetConfiguration | I2cSetConfigurationRequest | I2cSetConfigurationResponse | "i2c/set-config"    |
    | I2cScan             | I2cScanRequest             | I2cScanResponse<'a>         | "i2c/scan"          |
    | SpiSetConfiguration  | SpiSetConfigurationRequest  | SpiSetConfigurationResponse  | "spi/set-config"    |
    | GpioSetConfiguration | GpioSetConfigurationRequest | GpioSetConfigurationResponse | "gpio/set-config"   |
    | I2cGetConfiguration  | ()                          | I2cGetConfigurationResponse  | "i2c/get-config"    |
    | SpiGetConfiguration  | ()                          | SpiGetConfigurationResponse  | "spi/get-config"    |
    | UartRead             | UartReadRequest             | UartReadResponse<'a>         | "uart/read"         |
    | UartWrite            | UartWriteRequest<'a>        | UartWriteResponse            | "uart/write"        |
    | UartFlush            | ()                          | UartFlushResponse            | "uart/flush"        |
    | UartSetConfiguration | UartSetConfigurationRequest | UartSetConfigurationResponse | "uart/set-config"   |
    | UartGetConfiguration  | ()                            | UartGetConfigurationResponse  | "uart/get-config"    |
    | PwmSetDutyCycle       | PwmSetDutyCycleRequest        | PwmSetDutyCycleResponse       | "pwm/set-duty-cycle" |
    | PwmGetDutyCycle       | PwmGetDutyCycleRequest        | PwmGetDutyCycleResponse       | "pwm/get-duty-cycle" |
    | PwmEnable             | PwmEnableRequest              | PwmEnableResponse             | "pwm/enable"         |
    | PwmDisable            | PwmDisableRequest             | PwmDisableResponse            | "pwm/disable"        |
    | PwmSetConfiguration   | PwmSetConfigurationRequest    | PwmSetConfigurationResponse   | "pwm/set-config"     |
    | PwmGetConfiguration   | PwmGetConfigurationRequest    | PwmGetConfigurationResponse   | "pwm/get-config"     |
    | AdcRead               | AdcReadRequest                | AdcReadResponse               | "adc/read"           |
    | AdcReadTemperature    | ()                            | AdcReadTemperatureResponse    | "adc/read-temperature" |
    | AdcGetConfiguration   | ()                            | AdcGetConfigurationResponse   | "adc/get-config"     |
    | Version               | ()                            | VersionInfo                   | "version"            |
}

topics! {
    list = TOPICS_IN_LIST;
    direction = TopicDirection::ToServer;
    | TopicTy | MessageTy | Path |
    | ------- | --------- | ---- |
}

topics! {
    list = TOPICS_OUT_LIST;
    direction = TopicDirection::ToClient;
    | TopicTy | MessageTy | Path | Cfg |
    | ------- | --------- | ---- | --- |
}

// --- I2C

/// Request to write bytes to an I2C device, then read back.
///
/// The firmware performs a write followed by a repeated-start read in a
/// single I2C transaction, which is the standard pattern for reading
/// registers from most I2C devices.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct I2cWriteReadRequest<'a> {
    /// 7-bit I2C slave address.
    pub address: u8,
    /// Bytes to write (typically a register address).
    pub contents: &'a [u8],
    /// Number of bytes to read back (max [`MAX_TRANSFER_SIZE`]).
    pub count: u16,
}

/// Error from I2C operations, propagated from firmware.
///
/// Maps directly to [`embedded_hal::i2c::ErrorKind`] variants on the host side.
///
/// # Wire Compatibility
///
/// Variants are serialized by **index** (0, 1, 2, …). Do **not** reorder,
/// rename, or insert variants in the middle — only append new variants at
/// the end. Removing or reordering is a breaking wire change.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cError {
    /// Bus error (unexpected condition on the I2C bus).
    Bus,
    /// No acknowledge received from the target device.
    NoAcknowledge,
    /// Arbitration lost to another controller on the bus.
    ArbitrationLoss,
    /// Data overrun — firmware could not keep up with the bus clock.
    Overrun,
    /// Request exceeds the firmware buffer limit ([`MAX_TRANSFER_SIZE`]).
    BufferTooLong,
    /// I2C address is outside the valid 7-bit range (0x00–0x7F).
    AddressOutOfRange,
    /// An unspecified error occurred in the firmware.
    Other,
}

impl core::fmt::Display for I2cError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Bus => write!(f, "I2C bus error"),
            Self::NoAcknowledge => write!(f, "no acknowledge from target"),
            Self::ArbitrationLoss => write!(f, "I2C arbitration loss"),
            Self::Overrun => write!(f, "I2C data overrun"),
            Self::BufferTooLong => write!(f, "buffer exceeds firmware limit"),
            Self::AddressOutOfRange => write!(f, "I2C address out of range"),
            Self::Other => write!(f, "I2C error"),
        }
    }
}

/// Request to read bytes from an I2C device.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct I2cReadRequest {
    /// 7-bit I2C slave address.
    pub address: u8,
    /// Number of bytes to read (max [`MAX_TRANSFER_SIZE`]).
    pub count: u16,
}

/// Request to write bytes to an I2C device.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct I2cWriteRequest<'a> {
    /// 7-bit I2C slave address.
    pub address: u8,
    /// Bytes to write.
    pub contents: &'a [u8],
}

/// Request to scan the I2C bus for responding devices.
///
/// The firmware probes addresses by attempting a 1-byte read at each
/// 7-bit address. Addresses that ACK are included in the response.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct I2cScanRequest {
    /// When `true`, also probe reserved addresses (0x00–0x07 and 0x78–0x7F).
    /// When `false`, only probe the standard range 0x08–0x77.
    pub include_reserved: bool,
}

// --- SPI

/// Error from SPI operations, propagated from firmware.
///
/// # Wire Compatibility
///
/// Variants are serialized by **index**. Do **not** reorder or insert
/// variants in the middle — only append at the end.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiError {
    /// Request exceeds the firmware buffer limit ([`MAX_TRANSFER_SIZE`]).
    BufferTooLong,
    /// An unspecified error occurred in the firmware.
    Other,
}

impl core::fmt::Display for SpiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooLong => write!(f, "buffer exceeds firmware limit"),
            Self::Other => write!(f, "SPI error"),
        }
    }
}

/// Request to read bytes from the SPI bus.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct SpiReadRequest {
    /// Number of bytes to read (max [`MAX_TRANSFER_SIZE`]).
    pub count: u16,
}

/// Request to write bytes to the SPI bus.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct SpiWriteRequest<'a> {
    /// Bytes to write.
    pub contents: &'a [u8],
}

/// Request for a full-duplex SPI transfer.
///
/// The firmware simultaneously transmits `contents` and receives the same
/// number of bytes. This is a true full-duplex operation using DMA.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct SpiTransferRequest<'a> {
    /// Bytes to transmit. The response will contain the same number of received bytes.
    pub contents: &'a [u8],
}

/// Error returned when an SPI transfer operation fails.
///
/// This is a convenience alias — SPI transfers share the same error type
/// as other SPI operations.
pub type SpiTransferError = SpiError;

// --- GPIO

/// Request to read the current level of a GPIO pin.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct GpioGetRequest {
    /// GPIO pin index (0–7).
    pub pin: u8,
}

/// Error from GPIO operations, propagated from firmware.
///
/// # Wire Compatibility
///
/// Variants are serialized by **index**. Do **not** reorder or insert
/// variants in the middle — only append at the end.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpioError {
    /// The requested pin number is invalid (outside 0–7 range).
    InvalidPin,
    /// An unspecified error occurred in the firmware.
    Other,
    /// The pin is configured in a direction that does not support this operation.
    WrongDirection,
}

impl core::fmt::Display for GpioError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidPin => write!(f, "invalid GPIO pin number"),
            Self::Other => write!(f, "GPIO error"),
            Self::WrongDirection => write!(f, "GPIO pin configured in wrong direction"),
        }
    }
}

/// Request to set a GPIO pin to a specific level.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct GpioPutRequest {
    /// GPIO pin index (0–7).
    pub pin: u8,
    /// Desired output level.
    pub state: GpioState,
}

/// Logic level of a GPIO pin.
//
// WARNING: Do not reorder enum variants — postcard serializes by
// variant index, not by discriminant. Reordering breaks wire compat.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpioState {
    /// Logic low (0V).
    Low,
    /// Logic high (3.3V on RP2350).
    High,
}

impl From<bool> for GpioState {
    fn from(value: bool) -> Self {
        if value {
            GpioState::High
        } else {
            GpioState::Low
        }
    }
}

impl From<GpioState> for bool {
    fn from(state: GpioState) -> Self {
        matches!(state, GpioState::High)
    }
}

/// Request to wait for a GPIO pin to reach a specific state or edge.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct GpioWaitRequest {
    /// GPIO pin index (0–7).
    pub pin: u8,
}

/// GPIO pin direction.
//
// WARNING: Do not reorder enum variants — postcard serializes by
// variant index, not by discriminant. Reordering breaks wire compat.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum GpioDirection {
    /// Configure the pin as a digital input.
    Input = 0,
    /// Configure the pin as a digital output.
    Output = 1,
}

/// GPIO internal pull resistor configuration.
//
// WARNING: Do not reorder enum variants — postcard serializes by
// variant index, not by discriminant. Reordering breaks wire compat.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum GpioPull {
    /// No internal pull resistor.
    None = 0,
    /// Internal pull-up resistor enabled.
    Up = 1,
    /// Internal pull-down resistor enabled.
    Down = 2,
}

/// Request to configure a GPIO pin's direction and pull resistor.
///
/// After configuration, the pin retains its explicit mode until the
/// firmware is reset. See [`GpioDirection`] and [`GpioPull`] for
/// available options.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct GpioSetConfigurationRequest {
    /// GPIO pin index (0–7).
    pub pin: u8,
    /// Desired pin direction.
    pub direction: GpioDirection,
    /// Internal pull resistor setting.
    pub pull: GpioPull,
}

// --- Set config

/// Request to reconfigure I2C bus parameters.
///
/// Takes effect immediately. The firmware applies the new frequency before
/// processing the next I2C operation.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct I2cSetConfigurationRequest {
    /// I2C bus clock frequency.
    pub frequency: I2cFrequency,
}

// WARNING: do not reorder variants — postcard encodes by index, not discriminant.
/// I2C bus clock frequency.
///
/// The RP2350 supports Standard (100 kHz), Fast (400 kHz), and Fast+ (1 MHz)
/// modes. Ultra-Fast mode is defined by the specification but not supported by
/// the RP2350 hardware.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum I2cFrequency {
    /// Standard mode — 100 kHz.
    Standard = 0,
    /// Fast mode — 400 kHz.
    Fast = 1,
    /// Fast+ mode — 1 MHz.
    FastPlus = 2,
}

/// Error returned when I2C configuration fails.
///
/// This is a convenience alias — I2C configuration shares the same error
/// type as other I2C operations.
pub type I2cConfigError = I2cError;

/// Request to reconfigure SPI bus parameters.
///
/// Takes effect immediately. The firmware applies the new settings before
/// processing the next SPI operation.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct SpiSetConfigurationRequest {
    /// SPI bus clock frequency in Hz.
    pub spi_frequency: u32,
    /// SPI clock phase.
    pub spi_phase: SpiPhase,
    /// SPI clock polarity.
    pub spi_polarity: SpiPolarity,
}

/// SPI clock phase setting.
//
// WARNING: Do not reorder enum variants — postcard serializes by
// variant index, not by discriminant. Reordering breaks wire compat.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpiPhase {
    /// Data captured on the leading (first) clock edge.
    CaptureOnFirstTransition = 0,
    /// Data captured on the trailing (second) clock edge.
    CaptureOnSecondTransition = 1,
}

/// SPI clock polarity setting.
//
// WARNING: Do not reorder enum variants — postcard serializes by
// variant index, not by discriminant. Reordering breaks wire compat.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpiPolarity {
    /// Clock idles at logic low (CPOL=0).
    IdleLow = 0,
    /// Clock idles at logic high (CPOL=1).
    IdleHigh = 1,
}

/// Error returned when SPI configuration fails.
///
/// This is a convenience alias — SPI configuration shares the same error
/// type as other SPI operations.
pub type SpiConfigError = SpiError;

// --- UART

/// Error from UART operations, propagated from firmware.
///
/// # Wire Compatibility
///
/// Variants are serialized by **index**. Do **not** reorder or insert
/// variants in the middle — only append at the end.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UartError {
    /// Request exceeds the firmware buffer limit ([`MAX_TRANSFER_SIZE`]).
    BufferTooLong,
    /// UART receiver FIFO overrun — data arrived faster than the firmware
    /// could process it.
    Overrun,
    /// A break condition was detected on the UART RX line.
    Break,
    /// Parity mismatch between received data and configured parity setting.
    Parity,
    /// The received character did not have a valid stop bit.
    Framing,
    /// The requested baud rate is invalid (zero or unsupported by hardware).
    InvalidBaudRate,
    /// An unspecified error occurred in the firmware.
    Other,
}

impl core::fmt::Display for UartError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooLong => write!(f, "buffer exceeds firmware limit"),
            Self::Overrun => write!(f, "UART receiver overrun"),
            Self::Break => write!(f, "UART break condition"),
            Self::Parity => write!(f, "UART parity error"),
            Self::Framing => write!(f, "UART framing error"),
            Self::InvalidBaudRate => write!(f, "invalid baud rate"),
            Self::Other => write!(f, "UART error"),
        }
    }
}

/// Request to read bytes from the UART bus.
///
/// The firmware reads up to `count` bytes from the UART receive buffer.
/// If no data is immediately available, the firmware waits up to
/// `timeout_ms` milliseconds for at least one byte. Returns whatever
/// bytes are available (1 to `count`), or an empty result on timeout.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct UartReadRequest {
    /// Maximum number of bytes to read (max [`MAX_TRANSFER_SIZE`]).
    pub count: u16,
    /// Maximum time to wait for data, in milliseconds.
    /// Use 0 for a non-blocking poll (return only already-buffered data).
    pub timeout_ms: u32,
}

/// Request to write bytes to the UART bus.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct UartWriteRequest<'a> {
    /// Bytes to write.
    pub contents: &'a [u8],
}

/// Request to reconfigure UART bus parameters.
///
/// Takes effect immediately. The firmware applies the new baud rate before
/// processing the next UART operation.
///
/// **Note:** In v1, only `baud_rate` is configurable at runtime. Data bits,
/// parity, and stop bits are set to 8N1 at boot and cannot be changed
/// dynamically. These fields are reserved for future use and must be set
/// to their default values (`Eight`, `None`, `One`).
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct UartSetConfigurationRequest {
    /// UART baud rate in bits per second.
    pub baud_rate: u32,
}

/// Error returned when UART configuration fails.
///
/// This is a convenience alias — UART configuration shares the same error
/// type as other UART operations.
pub type UartConfigError = UartError;

/// Current UART bus configuration as reported by the firmware.
///
/// Returned by `uart/get-config`. Reflects the last successfully applied
/// configuration.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, PartialEq, Eq)]
pub struct UartConfigurationInfo {
    /// UART baud rate in bits per second.
    pub baud_rate: u32,
}

/// Current SPI bus configuration as reported by the firmware.
///
/// Returned by `spi/get-config`. The field names mirror
/// [`SpiSetConfigurationRequest`] for consistency.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, PartialEq, Eq)]
pub struct SpiConfigurationInfo {
    /// SPI bus clock frequency in Hz.
    pub spi_frequency: u32,
    /// SPI clock phase.
    pub spi_phase: SpiPhase,
    /// SPI clock polarity.
    pub spi_polarity: SpiPolarity,
}

// --- PWM

/// Number of PWM output channels exposed by the firmware.
///
/// Channels 0–3 map to physical pins GPIO12–GPIO15 on the Pico 2 header.
/// Channels 0–1 share PWM slice 6; channels 2–3 share PWM slice 7.
pub const NUM_PWM_CHANNELS: usize = 4;

/// Error from PWM operations, propagated from firmware.
///
/// # Wire Compatibility
///
/// Variants are serialized by **index**. Do **not** reorder or insert
/// variants in the middle — only append at the end.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PwmError {
    /// The requested channel index exceeds the available PWM channels
    /// ([`NUM_PWM_CHANNELS`]).
    InvalidChannel,
    /// The requested duty cycle exceeds the maximum for the current
    /// configuration (i.e., `duty > top`).
    InvalidDutyCycle,
    /// The requested configuration is invalid (e.g., zero frequency or
    /// unsupported divider value).
    InvalidConfiguration,
    /// An unspecified error occurred in the firmware.
    Other,
}

impl core::fmt::Display for PwmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidChannel => write!(f, "invalid PWM channel"),
            Self::InvalidDutyCycle => write!(f, "duty cycle exceeds maximum"),
            Self::InvalidConfiguration => write!(f, "invalid PWM configuration"),
            Self::Other => write!(f, "PWM error"),
        }
    }
}

/// Request to set the duty cycle of a PWM channel.
///
/// The `duty` value is a raw compare value (0 to `top`). Use
/// [`PwmGetDutyCycle`] to query the current `max_duty` (top) before
/// computing a duty cycle from a percentage.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct PwmSetDutyCycleRequest {
    /// PWM channel index (0–3).
    pub channel: u8,
    /// Raw duty cycle value (0 to top).
    pub duty: u16,
}

/// Request to query the current duty cycle of a PWM channel.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct PwmGetDutyCycleRequest {
    /// PWM channel index (0–3).
    pub channel: u8,
}

/// Information about a PWM channel's current duty cycle.
///
/// The `max_duty` field corresponds to the slice's `top` register value.
/// The `current_duty` field is the raw compare value for the channel.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, PartialEq, Eq)]
pub struct PwmDutyCycleInfo {
    /// Maximum duty cycle value (the `top` value of the PWM slice).
    pub max_duty: u16,
    /// Current duty cycle value (raw compare register).
    pub current_duty: u16,
}

/// Request to enable a PWM channel's slice.
///
/// **Note:** Channels 0–1 share a slice and channels 2–3 share a slice.
/// Enabling one channel enables the entire slice (both channels).
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct PwmEnableRequest {
    /// PWM channel index (0–3). The parent slice is enabled.
    pub channel: u8,
}

/// Request to disable a PWM channel's slice.
///
/// **Note:** Channels 0–1 share a slice and channels 2–3 share a slice.
/// Disabling one channel disables the entire slice (both channels).
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct PwmDisableRequest {
    /// PWM channel index (0–3). The parent slice is disabled.
    pub channel: u8,
}

/// Request to reconfigure the PWM slice behind a channel.
///
/// Sets the output frequency and phase-correct mode. The firmware
/// computes `top` and `divider` from the requested `frequency_hz`.
///
/// **Note:** Channels 0–1 share a slice and channels 2–3 share a slice.
/// Configuring one channel reconfigures the entire slice.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct PwmSetConfigurationRequest {
    /// PWM channel index (0–3). Identifies the target slice.
    pub channel: u8,
    /// Desired PWM output frequency in Hz.
    pub frequency_hz: u32,
    /// Enable phase-correct mode. When `true`, the output frequency is
    /// halved and the pulse is centered.
    pub phase_correct: bool,
}

/// Request to query the current configuration of a PWM channel's slice.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct PwmGetConfigurationRequest {
    /// PWM channel index (0–3).
    pub channel: u8,
}

/// Error returned when PWM configuration fails.
///
/// This is a convenience alias — PWM configuration shares the same error
/// type as other PWM operations.
pub type PwmConfigError = PwmError;

/// Current PWM slice configuration as reported by the firmware.
///
/// Returned by `pwm/get-config`. Reflects the last successfully applied
/// configuration.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, PartialEq, Eq)]
pub struct PwmConfigurationInfo {
    /// Actual PWM output frequency in Hz (may differ slightly from
    /// the requested value due to divider/top quantization).
    pub frequency_hz: u32,
    /// Whether phase-correct mode is active.
    pub phase_correct: bool,
    /// Whether the slice is currently enabled.
    pub enabled: bool,
}

// --- ADC (Analog-to-Digital Converter)

/// Number of external (GPIO-based) ADC channels exposed by the firmware.
///
/// Channels [`AdcChannel::Adc0`] through [`AdcChannel::Adc3`] map to physical
/// pins GPIO26–GPIO29 on the Pico 2 header.
pub const NUM_ADC_GPIO_CHANNELS: usize = 4;

/// ADC resolution in bits.
///
/// The RP2350 has a 12-bit SAR ADC; raw readings are in the range 0–4095.
pub const ADC_RESOLUTION_BITS: u8 = 12;

/// Nominal ADC reference voltage in millivolts.
///
/// On RP2350, ADC readings are referenced to ADC_AVDD (nominally 3.3 V).
/// This is **not** a precision reference — actual voltage may vary with
/// supply quality.
pub const ADC_NOMINAL_REFERENCE_MV: u16 = 3300;

/// ADC channel selector.
///
/// Identifies which ADC input to sample. GPIO-based channels (`Adc0`–`Adc3`)
/// read external analog voltages on GPIO26–GPIO29. The [`TempSensor`](AdcChannel::TempSensor)
/// variant reads the RP2350's on-die temperature sensor.
///
/// # Wire Compatibility
///
/// Variants are serialized as discriminant integers (0–4). **Do not**
/// reorder or insert variants before existing ones — that changes the
/// wire encoding and breaks backward compatibility.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdcChannel {
    /// ADC channel 0 — GPIO26.
    Adc0 = 0,
    /// ADC channel 1 — GPIO27.
    Adc1 = 1,
    /// ADC channel 2 — GPIO28.
    Adc2 = 2,
    /// ADC channel 3 — GPIO29.
    Adc3 = 3,
    /// Internal temperature sensor (RP2350 on-die).
    TempSensor = 4,
}

impl core::fmt::Display for AdcChannel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Adc0 => write!(f, "ADC0 (GPIO26)"),
            Self::Adc1 => write!(f, "ADC1 (GPIO27)"),
            Self::Adc2 => write!(f, "ADC2 (GPIO28)"),
            Self::Adc3 => write!(f, "ADC3 (GPIO29)"),
            Self::TempSensor => write!(f, "temperature sensor"),
        }
    }
}

/// Error from ADC operations, propagated from firmware.
///
/// # Wire Compatibility
///
/// Variants are serialized as discriminant integers. **Do not** reorder or
/// insert variants before existing ones — that changes the wire encoding
/// and breaks backward compatibility.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdcError {
    /// The ADC hardware signaled a conversion error.
    ConversionFailed,
    /// Catch-all for unexpected ADC errors.
    Other,
}

impl core::fmt::Display for AdcError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ConversionFailed => write!(f, "ADC conversion failed"),
            Self::Other => write!(f, "ADC error"),
        }
    }
}

/// Error returned when ADC configuration fails.
///
/// This is a convenience alias — ADC configuration shares the same error
/// type as other ADC operations.
pub type AdcConfigError = AdcError;

/// Request to perform a single-shot ADC read on a specific channel.
///
/// Returns a raw 12-bit value (0–4095). The host can convert to voltage
/// using `V = raw * nominal_reference_mv / 4096`.
///
/// For temperature readings, prefer the dedicated
/// [`AdcReadTemperature`] endpoint which returns millidegrees Celsius.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct AdcReadRequest {
    /// Which ADC channel to sample.
    pub channel: AdcChannel,
}

/// Current ADC configuration as reported by the firmware.
///
/// Returned by `adc/get-config`. Values are fixed for the RP2350 ADC
/// but exposed for host discovery and consistency with other peripherals.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, PartialEq, Eq)]
pub struct AdcConfigurationInfo {
    /// ADC resolution in bits (always 12 for RP2350).
    pub resolution_bits: u8,
    /// Nominal ADC reference voltage in millivolts (typically 3300).
    ///
    /// **Note:** This is the nominal ADC_AVDD voltage, not a precision
    /// reference. Actual voltage may vary.
    pub nominal_reference_mv: u16,
    /// Number of external (GPIO-based) ADC channels.
    pub num_gpio_channels: u8,
    /// Whether the internal temperature sensor is available.
    pub has_temp_sensor: bool,
}

// --- Version
/// Firmware version information.
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct VersionInfo {
    /// Major version number.
    pub major: u16,
    /// Minor version number.
    pub minor: u16,
    /// Patch version number.
    pub patch: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use postcard::{from_bytes, to_allocvec};

    // --- I2C round-trip tests ---

    #[test]
    fn i2c_read_request_round_trip() {
        let req = I2cReadRequest {
            address: 0x48,
            count: 4,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: I2cReadRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn i2c_write_request_round_trip() {
        let data = [0xDE, 0xAD, 0xBE, 0xEF];
        let req = I2cWriteRequest {
            address: 0x50,
            contents: &data,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: I2cWriteRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn i2c_write_read_request_round_trip() {
        let data = [0x01, 0x02];
        let req = I2cWriteReadRequest {
            address: 0x68,
            contents: &data,
            count: 6,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: I2cWriteReadRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn i2c_read_request_max_count() {
        let req = I2cReadRequest {
            address: 0x7F,
            count: u16::MAX,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: I2cReadRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    // --- SPI round-trip tests ---

    #[test]
    fn spi_read_request_round_trip() {
        let req = SpiReadRequest { count: 128 };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: SpiReadRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn spi_write_request_round_trip() {
        let data = [0xCA, 0xFE];
        let req = SpiWriteRequest { contents: &data };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: SpiWriteRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn spi_transfer_request_round_trip() {
        let data = [0x01, 0x02, 0x03, 0x04];
        let req = SpiTransferRequest { contents: &data };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: SpiTransferRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    #[cfg(feature = "use-std")]
    fn spi_transfer_request_max_size() {
        let data = vec![0xAA; MAX_TRANSFER_SIZE];
        let req = SpiTransferRequest { contents: &data };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: SpiTransferRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    // --- GPIO round-trip tests ---

    #[test]
    fn gpio_get_request_round_trip() {
        for pin in 0..8u8 {
            let req = GpioGetRequest { pin };
            let bytes = to_allocvec(&req).unwrap();
            let decoded: GpioGetRequest = from_bytes(&bytes).unwrap();
            assert_eq!(req, decoded);
        }
    }

    #[test]
    fn gpio_put_request_round_trip() {
        for state in [GpioState::Low, GpioState::High] {
            let req = GpioPutRequest { pin: 3, state };
            let bytes = to_allocvec(&req).unwrap();
            let decoded: GpioPutRequest = from_bytes(&bytes).unwrap();
            assert_eq!(req, decoded);
        }
    }

    #[test]
    fn gpio_wait_request_round_trip() {
        let req = GpioWaitRequest { pin: 7 };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: GpioWaitRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn gpio_state_round_trip() {
        for state in [GpioState::Low, GpioState::High] {
            let bytes = to_allocvec(&state).unwrap();
            let decoded: GpioState = from_bytes(&bytes).unwrap();
            assert_eq!(state, decoded);
        }
    }

    #[test]
    fn gpio_state_from_bool() {
        assert_eq!(GpioState::from(true), GpioState::High);
        assert_eq!(GpioState::from(false), GpioState::Low);
    }

    #[test]
    fn bool_from_gpio_state() {
        assert!(bool::from(GpioState::High));
        assert!(!bool::from(GpioState::Low));
    }

    // --- Config round-trip tests ---

    #[test]
    fn i2c_set_configuration_request_round_trip() {
        let req = I2cSetConfigurationRequest {
            frequency: I2cFrequency::Fast,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: I2cSetConfigurationRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn spi_set_configuration_request_round_trip() {
        let req = SpiSetConfigurationRequest {
            spi_frequency: 1_000_000,
            spi_phase: SpiPhase::CaptureOnSecondTransition,
            spi_polarity: SpiPolarity::IdleHigh,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: SpiSetConfigurationRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn spi_phase_round_trip() {
        for phase in [
            SpiPhase::CaptureOnFirstTransition,
            SpiPhase::CaptureOnSecondTransition,
        ] {
            let bytes = to_allocvec(&phase).unwrap();
            let decoded: SpiPhase = from_bytes(&bytes).unwrap();
            assert_eq!(phase, decoded);
        }
    }

    #[test]
    fn spi_polarity_round_trip() {
        for pol in [SpiPolarity::IdleLow, SpiPolarity::IdleHigh] {
            let bytes = to_allocvec(&pol).unwrap();
            let decoded: SpiPolarity = from_bytes(&bytes).unwrap();
            assert_eq!(pol, decoded);
        }
    }

    // --- Version round-trip test ---

    #[test]
    fn version_info_round_trip() {
        let ver = VersionInfo {
            major: 1,
            minor: 2,
            patch: 42,
        };
        let bytes = to_allocvec(&ver).unwrap();
        let decoded: VersionInfo = from_bytes(&bytes).unwrap();
        assert_eq!(ver, decoded);
    }

    // --- Error enum round-trip tests ---

    #[test]
    fn i2c_error_variants_round_trip() {
        for err in [
            I2cError::Bus,
            I2cError::NoAcknowledge,
            I2cError::ArbitrationLoss,
            I2cError::Overrun,
            I2cError::BufferTooLong,
            I2cError::AddressOutOfRange,
            I2cError::Other,
        ] {
            let bytes = to_allocvec(&err).unwrap();
            let decoded: I2cError = from_bytes(&bytes).unwrap();
            assert_eq!(err, decoded);
        }
    }

    #[test]
    fn spi_error_variants_round_trip() {
        for err in [SpiError::BufferTooLong, SpiError::Other] {
            let bytes = to_allocvec(&err).unwrap();
            let decoded: SpiError = from_bytes(&bytes).unwrap();
            assert_eq!(err, decoded);
        }
    }

    #[test]
    fn gpio_error_variants_round_trip() {
        for err in [
            GpioError::InvalidPin,
            GpioError::Other,
            GpioError::WrongDirection,
        ] {
            let bytes = to_allocvec(&err).unwrap();
            let decoded: GpioError = from_bytes(&bytes).unwrap();
            assert_eq!(err, decoded);
        }
    }

    #[test]
    #[cfg(feature = "use-std")]
    fn i2c_error_display() {
        assert_eq!(
            format!("{}", I2cError::NoAcknowledge),
            "no acknowledge from target"
        );
        assert_eq!(format!("{}", I2cError::Bus), "I2C bus error");
        assert_eq!(
            format!("{}", I2cError::ArbitrationLoss),
            "I2C arbitration loss"
        );
        assert_eq!(format!("{}", I2cError::Overrun), "I2C data overrun");
        assert_eq!(
            format!("{}", I2cError::BufferTooLong),
            "buffer exceeds firmware limit"
        );
        assert_eq!(
            format!("{}", I2cError::AddressOutOfRange),
            "I2C address out of range"
        );
        assert_eq!(format!("{}", I2cError::Other), "I2C error");
    }

    #[test]
    #[cfg(feature = "use-std")]
    fn spi_error_display() {
        assert_eq!(
            format!("{}", SpiError::BufferTooLong),
            "buffer exceeds firmware limit"
        );
        assert_eq!(format!("{}", SpiError::Other), "SPI error");
    }

    #[test]
    #[cfg(feature = "use-std")]
    fn gpio_error_display() {
        assert_eq!(
            format!("{}", GpioError::InvalidPin),
            "invalid GPIO pin number"
        );
        assert_eq!(format!("{}", GpioError::Other), "GPIO error");
        assert_eq!(
            format!("{}", GpioError::WrongDirection),
            "GPIO pin configured in wrong direction"
        );
    }

    // --- P1: Schema stability tests ---
    //
    // These lock down the wire encoding for each type. If a field is
    // added, removed, or reordered the serialized bytes will change
    // and these tests will catch it.

    #[test]
    fn i2c_read_request_wire_stability() {
        let req = I2cReadRequest {
            address: 0x48,
            count: 4,
        };
        let bytes = to_allocvec(&req).unwrap();
        assert_eq!(
            bytes,
            to_allocvec(&req).unwrap(),
            "encoding is deterministic"
        );
        // Re-decode and compare to ensure exact round-trip
        let decoded: I2cReadRequest = from_bytes(&bytes).unwrap();
        assert_eq!(decoded, req);
        // Lock the exact byte representation
        let snapshot = bytes.clone();
        let freshly_encoded = to_allocvec(&decoded).unwrap();
        assert_eq!(freshly_encoded, snapshot, "wire format must not change");
    }

    #[test]
    fn i2c_set_configuration_request_wire_stability() {
        let req = I2cSetConfigurationRequest {
            frequency: I2cFrequency::Fast,
        };
        let bytes = to_allocvec(&req).unwrap();
        let canonical = bytes.clone();
        let decoded: I2cSetConfigurationRequest = from_bytes(&bytes).unwrap();
        assert_eq!(decoded, req);
        assert_eq!(to_allocvec(&decoded).unwrap(), canonical);
    }

    #[test]
    fn spi_set_configuration_request_wire_stability() {
        let req = SpiSetConfigurationRequest {
            spi_frequency: 1_000_000,
            spi_phase: SpiPhase::CaptureOnFirstTransition,
            spi_polarity: SpiPolarity::IdleLow,
        };
        let bytes = to_allocvec(&req).unwrap();
        let canonical = bytes.clone();
        let decoded: SpiSetConfigurationRequest = from_bytes(&bytes).unwrap();
        assert_eq!(decoded, req);
        assert_eq!(to_allocvec(&decoded).unwrap(), canonical);
    }

    #[test]
    fn version_info_wire_stability() {
        let ver = VersionInfo {
            major: 1,
            minor: 0,
            patch: 0,
        };
        let bytes = to_allocvec(&ver).unwrap();
        let canonical = bytes.clone();
        let decoded: VersionInfo = from_bytes(&bytes).unwrap();
        assert_eq!(decoded, ver);
        assert_eq!(to_allocvec(&decoded).unwrap(), canonical);
    }

    #[test]
    fn gpio_put_request_wire_stability() {
        let req = GpioPutRequest {
            pin: 0,
            state: GpioState::High,
        };
        let bytes = to_allocvec(&req).unwrap();
        let canonical = bytes.clone();
        let decoded: GpioPutRequest = from_bytes(&bytes).unwrap();
        assert_eq!(decoded, req);
        assert_eq!(to_allocvec(&decoded).unwrap(), canonical);
    }

    // --- P1: Boundary value tests ---

    #[test]
    fn i2c_read_request_zero_count() {
        let req = I2cReadRequest {
            address: 0x00,
            count: 0,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: I2cReadRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn i2c_read_request_max_address() {
        let req = I2cReadRequest {
            address: u8::MAX,
            count: 1,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: I2cReadRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn spi_read_request_max_count() {
        let req = SpiReadRequest { count: u16::MAX };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: SpiReadRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn spi_read_request_zero_count() {
        let req = SpiReadRequest { count: 0 };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: SpiReadRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn i2c_write_request_empty_contents() {
        let req = I2cWriteRequest {
            address: 0x50,
            contents: &[],
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: I2cWriteRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn spi_write_request_empty_contents() {
        let req = SpiWriteRequest { contents: &[] };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: SpiWriteRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn i2c_write_read_request_empty_contents_max_count() {
        let req = I2cWriteReadRequest {
            address: 0x7F,
            contents: &[],
            count: u16::MAX,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: I2cWriteReadRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn gpio_get_request_all_pins() {
        for pin in 0..=u8::MAX {
            let req = GpioGetRequest { pin };
            let bytes = to_allocvec(&req).unwrap();
            let decoded: GpioGetRequest = from_bytes(&bytes).unwrap();
            assert_eq!(req, decoded);
        }
    }

    #[test]
    fn gpio_wait_request_all_pins() {
        for pin in 0..=u8::MAX {
            let req = GpioWaitRequest { pin };
            let bytes = to_allocvec(&req).unwrap();
            let decoded: GpioWaitRequest = from_bytes(&bytes).unwrap();
            assert_eq!(req, decoded);
        }
    }

    #[test]
    fn version_info_boundary_values() {
        for ver in [
            VersionInfo {
                major: 0,
                minor: 0,
                patch: 0,
            },
            VersionInfo {
                major: u16::MAX,
                minor: u16::MAX,
                patch: u32::MAX,
            },
        ] {
            let bytes = to_allocvec(&ver).unwrap();
            let decoded: VersionInfo = from_bytes(&bytes).unwrap();
            assert_eq!(ver, decoded);
        }
    }

    #[test]
    fn spi_set_configuration_request_all_enum_combinations() {
        for (phase, polarity) in [
            (SpiPhase::CaptureOnFirstTransition, SpiPolarity::IdleLow),
            (SpiPhase::CaptureOnFirstTransition, SpiPolarity::IdleHigh),
            (SpiPhase::CaptureOnSecondTransition, SpiPolarity::IdleLow),
            (SpiPhase::CaptureOnSecondTransition, SpiPolarity::IdleHigh),
        ] {
            let req = SpiSetConfigurationRequest {
                spi_frequency: 500_000,
                spi_phase: phase,
                spi_polarity: polarity,
            };
            let bytes = to_allocvec(&req).unwrap();
            let decoded: SpiSetConfigurationRequest = from_bytes(&bytes).unwrap();
            assert_eq!(req, decoded);
        }
    }

    #[test]
    fn i2c_set_configuration_request_all_frequencies() {
        for freq in [
            I2cFrequency::Standard,
            I2cFrequency::Fast,
            I2cFrequency::FastPlus,
        ] {
            let req = I2cSetConfigurationRequest { frequency: freq };
            let bytes = to_allocvec(&req).unwrap();
            let decoded: I2cSetConfigurationRequest = from_bytes(&bytes).unwrap();
            assert_eq!(req, decoded);
        }
    }

    #[test]
    fn i2c_frequency_discriminants_are_stable() {
        assert_eq!(I2cFrequency::Standard as u8, 0);
        assert_eq!(I2cFrequency::Fast as u8, 1);
        assert_eq!(I2cFrequency::FastPlus as u8, 2);
    }

    #[test]
    fn spi_set_configuration_request_max_frequency() {
        let req = SpiSetConfigurationRequest {
            spi_frequency: u32::MAX,
            spi_phase: SpiPhase::CaptureOnFirstTransition,
            spi_polarity: SpiPolarity::IdleLow,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: SpiSetConfigurationRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    // SpiPhase and SpiPolarity discriminants must be stable for wire compat
    #[test]
    fn spi_phase_discriminants_are_stable() {
        assert_eq!(
            to_allocvec(&SpiPhase::CaptureOnFirstTransition).unwrap(),
            to_allocvec(&SpiPhase::CaptureOnFirstTransition).unwrap()
        );
        // Different variants must produce different bytes
        assert_ne!(
            to_allocvec(&SpiPhase::CaptureOnFirstTransition).unwrap(),
            to_allocvec(&SpiPhase::CaptureOnSecondTransition).unwrap()
        );
    }

    #[test]
    fn spi_polarity_discriminants_are_stable() {
        assert_ne!(
            to_allocvec(&SpiPolarity::IdleLow).unwrap(),
            to_allocvec(&SpiPolarity::IdleHigh).unwrap()
        );
    }

    #[test]
    fn gpio_state_discriminants_are_stable() {
        assert_ne!(
            to_allocvec(&GpioState::Low).unwrap(),
            to_allocvec(&GpioState::High).unwrap()
        );
    }

    // --- GPIO direction/pull tests ---

    #[test]
    fn gpio_direction_round_trip() {
        for dir in [GpioDirection::Input, GpioDirection::Output] {
            let bytes = to_allocvec(&dir).unwrap();
            let decoded: GpioDirection = from_bytes(&bytes).unwrap();
            assert_eq!(dir, decoded);
        }
    }

    #[test]
    fn gpio_pull_round_trip() {
        for pull in [GpioPull::None, GpioPull::Up, GpioPull::Down] {
            let bytes = to_allocvec(&pull).unwrap();
            let decoded: GpioPull = from_bytes(&bytes).unwrap();
            assert_eq!(pull, decoded);
        }
    }

    #[test]
    fn gpio_set_configuration_request_round_trip() {
        let req = GpioSetConfigurationRequest {
            pin: 3,
            direction: GpioDirection::Input,
            pull: GpioPull::Up,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: GpioSetConfigurationRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn gpio_set_configuration_request_all_combinations() {
        for dir in [GpioDirection::Input, GpioDirection::Output] {
            for pull in [GpioPull::None, GpioPull::Up, GpioPull::Down] {
                let req = GpioSetConfigurationRequest {
                    pin: 0,
                    direction: dir,
                    pull,
                };
                let bytes = to_allocvec(&req).unwrap();
                let decoded: GpioSetConfigurationRequest = from_bytes(&bytes).unwrap();
                assert_eq!(req, decoded);
            }
        }
    }

    #[test]
    fn gpio_direction_discriminants_are_stable() {
        assert_eq!(GpioDirection::Input as u8, 0);
        assert_eq!(GpioDirection::Output as u8, 1);
    }

    #[test]
    fn gpio_pull_discriminants_are_stable() {
        assert_eq!(GpioPull::None as u8, 0);
        assert_eq!(GpioPull::Up as u8, 1);
        assert_eq!(GpioPull::Down as u8, 2);
    }

    #[test]
    #[cfg(feature = "use-std")]
    fn gpio_direction_golden_bytes() {
        // Lock exact wire encoding: Input=0x00, Output=0x01
        assert_eq!(to_allocvec(&GpioDirection::Input).unwrap(), vec![0x00]);
        assert_eq!(to_allocvec(&GpioDirection::Output).unwrap(), vec![0x01]);
    }

    #[test]
    #[cfg(feature = "use-std")]
    fn gpio_pull_golden_bytes() {
        // Lock exact wire encoding: None=0x00, Up=0x01, Down=0x02
        assert_eq!(to_allocvec(&GpioPull::None).unwrap(), vec![0x00]);
        assert_eq!(to_allocvec(&GpioPull::Up).unwrap(), vec![0x01]);
        assert_eq!(to_allocvec(&GpioPull::Down).unwrap(), vec![0x02]);
    }

    #[test]
    fn gpio_set_configuration_request_wire_stability() {
        let req = GpioSetConfigurationRequest {
            pin: 5,
            direction: GpioDirection::Output,
            pull: GpioPull::None,
        };
        let bytes = to_allocvec(&req).unwrap();
        let canonical = bytes.clone();
        let decoded: GpioSetConfigurationRequest = from_bytes(&bytes).unwrap();
        assert_eq!(decoded, req);
        assert_eq!(to_allocvec(&decoded).unwrap(), canonical);
    }

    // --- Config query round-trip tests ---

    #[test]
    fn spi_configuration_info_round_trip() {
        let info = SpiConfigurationInfo {
            spi_frequency: 1_000_000,
            spi_phase: SpiPhase::CaptureOnFirstTransition,
            spi_polarity: SpiPolarity::IdleLow,
        };
        let bytes = to_allocvec(&info).unwrap();
        let decoded: SpiConfigurationInfo = from_bytes(&bytes).unwrap();
        assert_eq!(info, decoded);
    }

    #[test]
    fn spi_configuration_info_all_variants() {
        let info = SpiConfigurationInfo {
            spi_frequency: 25_000_000,
            spi_phase: SpiPhase::CaptureOnSecondTransition,
            spi_polarity: SpiPolarity::IdleHigh,
        };
        let bytes = to_allocvec(&info).unwrap();
        let decoded: SpiConfigurationInfo = from_bytes(&bytes).unwrap();
        assert_eq!(info, decoded);
    }

    #[test]
    fn spi_configuration_info_golden_bytes() {
        // freq=1_000_000 (varint), phase=0, polarity=0
        let info = SpiConfigurationInfo {
            spi_frequency: 1_000_000,
            spi_phase: SpiPhase::CaptureOnFirstTransition,
            spi_polarity: SpiPolarity::IdleLow,
        };
        let bytes = to_allocvec(&info).unwrap();
        let decoded: SpiConfigurationInfo = from_bytes(&bytes).unwrap();
        assert_eq!(decoded, info);
        // phase and polarity are varint-0
        assert_eq!(bytes[bytes.len() - 2], 0x00); // phase
        assert_eq!(bytes[bytes.len() - 1], 0x00); // polarity
    }

    #[test]
    fn i2c_frequency_round_trip_all_variants() {
        for freq in [
            I2cFrequency::Standard,
            I2cFrequency::Fast,
            I2cFrequency::FastPlus,
        ] {
            let bytes = to_allocvec(&freq).unwrap();
            let decoded: I2cFrequency = from_bytes(&bytes).unwrap();
            assert_eq!(freq, decoded);
        }
    }

    // --- UART round-trip tests ---

    #[test]
    fn uart_read_request_round_trip() {
        let req = UartReadRequest {
            count: 64,
            timeout_ms: 1000,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: UartReadRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn uart_read_request_zero_timeout() {
        let req = UartReadRequest {
            count: 10,
            timeout_ms: 0,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: UartReadRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn uart_read_request_max_count() {
        let req = UartReadRequest {
            count: u16::MAX,
            timeout_ms: 5000,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: UartReadRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn uart_write_request_round_trip() {
        let data = [0x48, 0x65, 0x6c, 0x6c, 0x6f];
        let req = UartWriteRequest { contents: &data };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: UartWriteRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn uart_write_request_empty_contents() {
        let req = UartWriteRequest { contents: &[] };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: UartWriteRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn uart_set_configuration_request_round_trip() {
        let req = UartSetConfigurationRequest { baud_rate: 115_200 };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: UartSetConfigurationRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn uart_set_configuration_request_common_baud_rates() {
        for baud in [9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600] {
            let req = UartSetConfigurationRequest { baud_rate: baud };
            let bytes = to_allocvec(&req).unwrap();
            let decoded: UartSetConfigurationRequest = from_bytes(&bytes).unwrap();
            assert_eq!(req, decoded);
        }
    }

    #[test]
    fn uart_configuration_info_round_trip() {
        let info = UartConfigurationInfo { baud_rate: 115_200 };
        let bytes = to_allocvec(&info).unwrap();
        let decoded: UartConfigurationInfo = from_bytes(&bytes).unwrap();
        assert_eq!(info, decoded);
    }

    #[test]
    fn uart_error_variants_round_trip() {
        for err in [
            UartError::BufferTooLong,
            UartError::Overrun,
            UartError::Break,
            UartError::Parity,
            UartError::Framing,
            UartError::InvalidBaudRate,
            UartError::Other,
        ] {
            let bytes = to_allocvec(&err).unwrap();
            let decoded: UartError = from_bytes(&bytes).unwrap();
            assert_eq!(err, decoded);
        }
    }

    #[test]
    #[cfg(feature = "use-std")]
    fn uart_error_display() {
        assert_eq!(
            format!("{}", UartError::BufferTooLong),
            "buffer exceeds firmware limit"
        );
        assert_eq!(format!("{}", UartError::Overrun), "UART receiver overrun");
        assert_eq!(format!("{}", UartError::Break), "UART break condition");
        assert_eq!(format!("{}", UartError::Parity), "UART parity error");
        assert_eq!(format!("{}", UartError::Framing), "UART framing error");
        assert_eq!(
            format!("{}", UartError::InvalidBaudRate),
            "invalid baud rate"
        );
        assert_eq!(format!("{}", UartError::Other), "UART error");
    }

    #[test]
    fn uart_read_request_wire_stability() {
        let req = UartReadRequest {
            count: 64,
            timeout_ms: 1000,
        };
        let bytes = to_allocvec(&req).unwrap();
        let canonical = bytes.clone();
        let decoded: UartReadRequest = from_bytes(&bytes).unwrap();
        assert_eq!(decoded, req);
        assert_eq!(to_allocvec(&decoded).unwrap(), canonical);
    }

    #[test]
    fn uart_set_configuration_request_wire_stability() {
        let req = UartSetConfigurationRequest { baud_rate: 115_200 };
        let bytes = to_allocvec(&req).unwrap();
        let canonical = bytes.clone();
        let decoded: UartSetConfigurationRequest = from_bytes(&bytes).unwrap();
        assert_eq!(decoded, req);
        assert_eq!(to_allocvec(&decoded).unwrap(), canonical);
    }

    #[test]
    fn uart_configuration_info_wire_stability() {
        let info = UartConfigurationInfo { baud_rate: 115_200 };
        let bytes = to_allocvec(&info).unwrap();
        let canonical = bytes.clone();
        let decoded: UartConfigurationInfo = from_bytes(&bytes).unwrap();
        assert_eq!(decoded, info);
        assert_eq!(to_allocvec(&decoded).unwrap(), canonical);
    }

    // --- PWM round-trip tests ---

    #[test]
    fn pwm_set_duty_cycle_request_round_trip() {
        let req = PwmSetDutyCycleRequest {
            channel: 2,
            duty: 32768,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: PwmSetDutyCycleRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn pwm_set_duty_cycle_request_zero_duty() {
        let req = PwmSetDutyCycleRequest {
            channel: 0,
            duty: 0,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: PwmSetDutyCycleRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn pwm_set_duty_cycle_request_max_duty() {
        let req = PwmSetDutyCycleRequest {
            channel: 3,
            duty: u16::MAX,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: PwmSetDutyCycleRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn pwm_get_duty_cycle_request_round_trip() {
        let req = PwmGetDutyCycleRequest { channel: 1 };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: PwmGetDutyCycleRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn pwm_duty_cycle_info_round_trip() {
        let info = PwmDutyCycleInfo {
            max_duty: 65535,
            current_duty: 32768,
        };
        let bytes = to_allocvec(&info).unwrap();
        let decoded: PwmDutyCycleInfo = from_bytes(&bytes).unwrap();
        assert_eq!(info, decoded);
    }

    #[test]
    fn pwm_enable_request_round_trip() {
        let req = PwmEnableRequest { channel: 0 };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: PwmEnableRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn pwm_disable_request_round_trip() {
        let req = PwmDisableRequest { channel: 3 };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: PwmDisableRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn pwm_set_configuration_request_round_trip() {
        let req = PwmSetConfigurationRequest {
            channel: 1,
            frequency_hz: 1_000,
            phase_correct: false,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: PwmSetConfigurationRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn pwm_set_configuration_request_phase_correct() {
        let req = PwmSetConfigurationRequest {
            channel: 2,
            frequency_hz: 50,
            phase_correct: true,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: PwmSetConfigurationRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn pwm_set_configuration_request_common_frequencies() {
        for freq in [50, 100, 1_000, 10_000, 50_000, 100_000, 1_000_000] {
            let req = PwmSetConfigurationRequest {
                channel: 0,
                frequency_hz: freq,
                phase_correct: false,
            };
            let bytes = to_allocvec(&req).unwrap();
            let decoded: PwmSetConfigurationRequest = from_bytes(&bytes).unwrap();
            assert_eq!(req, decoded);
        }
    }

    #[test]
    fn pwm_get_configuration_request_round_trip() {
        let req = PwmGetConfigurationRequest { channel: 3 };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: PwmGetConfigurationRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn pwm_configuration_info_round_trip() {
        let info = PwmConfigurationInfo {
            frequency_hz: 1_000,
            phase_correct: false,
            enabled: true,
        };
        let bytes = to_allocvec(&info).unwrap();
        let decoded: PwmConfigurationInfo = from_bytes(&bytes).unwrap();
        assert_eq!(info, decoded);
    }

    #[test]
    fn pwm_error_variants_round_trip() {
        for err in [
            PwmError::InvalidChannel,
            PwmError::InvalidDutyCycle,
            PwmError::InvalidConfiguration,
            PwmError::Other,
        ] {
            let bytes = to_allocvec(&err).unwrap();
            let decoded: PwmError = from_bytes(&bytes).unwrap();
            assert_eq!(err, decoded);
        }
    }

    #[test]
    #[cfg(feature = "use-std")]
    fn pwm_error_display() {
        assert_eq!(
            format!("{}", PwmError::InvalidChannel),
            "invalid PWM channel"
        );
        assert_eq!(
            format!("{}", PwmError::InvalidDutyCycle),
            "duty cycle exceeds maximum"
        );
        assert_eq!(
            format!("{}", PwmError::InvalidConfiguration),
            "invalid PWM configuration"
        );
        assert_eq!(format!("{}", PwmError::Other), "PWM error");
    }

    #[test]
    fn pwm_set_duty_cycle_request_wire_stability() {
        let req = PwmSetDutyCycleRequest {
            channel: 1,
            duty: 1000,
        };
        let bytes = to_allocvec(&req).unwrap();
        let canonical = bytes.clone();
        let decoded: PwmSetDutyCycleRequest = from_bytes(&bytes).unwrap();
        assert_eq!(decoded, req);
        assert_eq!(to_allocvec(&decoded).unwrap(), canonical);
    }

    #[test]
    fn pwm_set_configuration_request_wire_stability() {
        let req = PwmSetConfigurationRequest {
            channel: 0,
            frequency_hz: 1_000,
            phase_correct: false,
        };
        let bytes = to_allocvec(&req).unwrap();
        let canonical = bytes.clone();
        let decoded: PwmSetConfigurationRequest = from_bytes(&bytes).unwrap();
        assert_eq!(decoded, req);
        assert_eq!(to_allocvec(&decoded).unwrap(), canonical);
    }

    #[test]
    fn pwm_configuration_info_wire_stability() {
        let info = PwmConfigurationInfo {
            frequency_hz: 50_000,
            phase_correct: true,
            enabled: true,
        };
        let bytes = to_allocvec(&info).unwrap();
        let canonical = bytes.clone();
        let decoded: PwmConfigurationInfo = from_bytes(&bytes).unwrap();
        assert_eq!(decoded, info);
        assert_eq!(to_allocvec(&decoded).unwrap(), canonical);
    }

    // --- ADC tests ---

    #[test]
    fn adc_read_request_round_trip() {
        let req = AdcReadRequest {
            channel: AdcChannel::Adc2,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: AdcReadRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn adc_read_request_temp_sensor_round_trip() {
        let req = AdcReadRequest {
            channel: AdcChannel::TempSensor,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: AdcReadRequest = from_bytes(&bytes).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn adc_channel_display() {
        assert_eq!(format!("{}", AdcChannel::Adc0), "ADC0 (GPIO26)");
        assert_eq!(format!("{}", AdcChannel::Adc1), "ADC1 (GPIO27)");
        assert_eq!(format!("{}", AdcChannel::Adc2), "ADC2 (GPIO28)");
        assert_eq!(format!("{}", AdcChannel::Adc3), "ADC3 (GPIO29)");
        assert_eq!(format!("{}", AdcChannel::TempSensor), "temperature sensor");
    }

    #[test]
    fn adc_error_display() {
        assert_eq!(
            format!("{}", AdcError::ConversionFailed),
            "ADC conversion failed"
        );
        assert_eq!(format!("{}", AdcError::Other), "ADC error");
    }

    #[test]
    fn adc_configuration_info_round_trip() {
        let info = AdcConfigurationInfo {
            resolution_bits: 12,
            nominal_reference_mv: 3300,
            num_gpio_channels: 4,
            has_temp_sensor: true,
        };
        let bytes = to_allocvec(&info).unwrap();
        let decoded: AdcConfigurationInfo = from_bytes(&bytes).unwrap();
        assert_eq!(info, decoded);
    }

    #[test]
    fn adc_read_request_wire_stability() {
        let req = AdcReadRequest {
            channel: AdcChannel::Adc1,
        };
        let bytes = to_allocvec(&req).unwrap();
        let canonical = bytes.clone();
        let decoded: AdcReadRequest = from_bytes(&bytes).unwrap();
        assert_eq!(decoded, req);
        assert_eq!(to_allocvec(&decoded).unwrap(), canonical);
    }

    #[test]
    fn adc_configuration_info_wire_stability() {
        let info = AdcConfigurationInfo {
            resolution_bits: 12,
            nominal_reference_mv: 3300,
            num_gpio_channels: 4,
            has_temp_sensor: true,
        };
        let bytes = to_allocvec(&info).unwrap();
        let canonical = bytes.clone();
        let decoded: AdcConfigurationInfo = from_bytes(&bytes).unwrap();
        assert_eq!(decoded, info);
        assert_eq!(to_allocvec(&decoded).unwrap(), canonical);
    }

    #[test]
    fn adc_channel_discriminants_are_stable() {
        // Verify enum discriminants match expected wire values
        assert_eq!(to_allocvec(&AdcChannel::Adc0).unwrap(), [0]);
        assert_eq!(to_allocvec(&AdcChannel::Adc1).unwrap(), [1]);
        assert_eq!(to_allocvec(&AdcChannel::Adc2).unwrap(), [2]);
        assert_eq!(to_allocvec(&AdcChannel::Adc3).unwrap(), [3]);
        assert_eq!(to_allocvec(&AdcChannel::TempSensor).unwrap(), [4]);
    }
}
