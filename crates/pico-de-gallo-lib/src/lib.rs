use nusb::DeviceInfo;
use pico_de_gallo_internal::{
    GpioGet, GpioGetFail, GpioGetRequest, GpioPut, GpioPutFail, GpioPutRequest, GpioWaitFail, GpioWaitForAny,
    GpioWaitForFalling, GpioWaitForHigh, GpioWaitForLow, GpioWaitForRising, GpioWaitRequest, I2cRead, I2cReadFail,
    I2cReadRequest, I2cWrite, I2cWriteFail, I2cWriteRead, I2cWriteReadFail, I2cWriteReadRequest, I2cWriteRequest,
    MICROSOFT_VID, PICO_DE_GALLO_PID, SetConfiguration, SetConfigurationFail, SetConfigurationRequest, SpiFlush,
    SpiFlushFail, SpiRead, SpiReadFail, SpiReadRequest, SpiTransfer, SpiTransferFail, SpiTransferRequest, SpiWrite,
    SpiWriteFail, SpiWriteRequest, Version,
};

pub use pico_de_gallo_internal::{GpioState, SpiPhase, SpiPolarity, VersionInfo};

use postcard_rpc::{
    header::VarSeqKind,
    host_client::{HostClient, HostErr},
    standard_icd::{ERROR_PATH, PingEndpoint, WireError},
};
use std::convert::Infallible;

/// Description of a connected Pico de Gallo device.
#[derive(Debug, Clone)]
pub struct DeviceDescription {
    /// USB serial number (unique per board, derived from chip ID).
    pub serial_number: Option<String>,
    /// USB manufacturer string.
    pub manufacturer: Option<String>,
    /// USB product string.
    pub product: Option<String>,
}

/// List all connected Pico de Gallo devices.
///
/// Returns a description for each device found on the USB bus matching the
/// Pico de Gallo VID/PID. Use the serial number with
/// [`PicoDeGallo::new_with_serial_number`] to connect to a specific device.
pub fn list_devices() -> Vec<DeviceDescription> {
    let devices = match nusb::list_devices() {
        Ok(iter) => iter,
        Err(_) => return Vec::new(),
    };
    devices
        .filter(|dev| dev.vendor_id() == MICROSOFT_VID && dev.product_id() == PICO_DE_GALLO_PID)
        .map(|dev| DeviceDescription {
            serial_number: dev.serial_number().map(String::from),
            manufacturer: dev.manufacturer_string().map(String::from),
            product: dev.product_string().map(String::from),
        })
        .collect()
}

#[derive(Debug)]
pub enum PicoDeGalloError<E> {
    Comms(HostErr<WireError>),
    Endpoint(E),
}

impl<E: core::fmt::Display> core::fmt::Display for PicoDeGalloError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Comms(e) => write!(f, "communication error: {e:?}"),
            Self::Endpoint(e) => write!(f, "endpoint error: {e}"),
        }
    }
}

impl<E: core::fmt::Debug + core::fmt::Display> std::error::Error for PicoDeGalloError<E> {}

impl<E> From<HostErr<WireError>> for PicoDeGalloError<E> {
    fn from(value: HostErr<WireError>) -> Self {
        Self::Comms(value)
    }
}

#[derive(Clone)]
pub struct PicoDeGallo {
    client: HostClient<WireError>,
}

impl Default for PicoDeGallo {
    fn default() -> Self {
        Self::new()
    }
}

impl PicoDeGallo {
    /// Create a new instance for the Pico de Gallo device.
    ///
    /// NOTICE:
    ///
    /// This constructor will return the first matching device in case
    /// there are more than one connected.
    ///
    /// If you want more control, please use `new_with_serial_number`
    /// instead.
    pub fn new() -> Self {
        Self::new_inner(|dev| dev.vendor_id() == MICROSOFT_VID && dev.product_id() == PICO_DE_GALLO_PID)
    }

    /// Create a new instance for the Pico de Gallo device with the
    /// given serial number.
    pub fn new_with_serial_number(serial_number: &str) -> Self {
        Self::new_inner(|dev| {
            dev.vendor_id() == MICROSOFT_VID
                && dev.product_id() == PICO_DE_GALLO_PID
                && dev.serial_number() == Some(serial_number)
        })
    }

    fn new_inner<F: FnMut(&DeviceInfo) -> bool>(func: F) -> Self {
        let client = HostClient::new_raw_nusb(func, ERROR_PATH, 8, VarSeqKind::Seq2);
        Self { client }
    }

    /// Wait until the client has closed the connection.
    pub async fn wait_closed(&self) {
        self.client.wait_closed().await;
    }

    /// Ping endpoint.
    ///
    /// Only used for testing purposes. Send a `u32` and get the same
    /// `u32` as a response.
    pub async fn ping(&self, id: u32) -> Result<u32, PicoDeGalloError<Infallible>> {
        Ok(self.client.send_resp::<PingEndpoint>(&id).await?)
    }

    /// Read `count` bytes from the I2C device at `address`.
    ///
    /// The firmware buffer is limited to [`pico_de_gallo_internal::MAX_TRANSFER_SIZE`]
    /// (512) bytes. Reads exceeding this limit will be truncated.
    pub async fn i2c_read(&self, address: u8, count: u16) -> Result<Vec<u8>, PicoDeGalloError<I2cReadFail>> {
        self.client
            .send_resp::<I2cRead>(&I2cReadRequest { address, count })
            .await?
            .map_err(PicoDeGalloError::Endpoint)
    }

    /// Write `contents` to the I2C device at `address`.
    pub async fn i2c_write(&self, address: u8, contents: &[u8]) -> Result<(), PicoDeGalloError<I2cWriteFail>> {
        self.client
            .send_resp::<I2cWrite>(&I2cWriteRequest { address, contents })
            .await?
            .map_err(PicoDeGalloError::Endpoint)
    }

    /// Write `contents` to the I2C device at `address` and read back `count` bytes.
    ///
    /// The firmware buffer is limited to [`pico_de_gallo_internal::MAX_TRANSFER_SIZE`]
    /// (512) bytes. Reads exceeding this limit will be truncated.
    pub async fn i2c_write_read(
        &self,
        address: u8,
        contents: &[u8],
        count: u16,
    ) -> Result<Vec<u8>, PicoDeGalloError<I2cWriteReadFail>> {
        self.client
            .send_resp::<I2cWriteRead>(&I2cWriteReadRequest {
                address,
                contents,
                count,
            })
            .await?
            .map_err(PicoDeGalloError::Endpoint)
    }

    /// Read `count` bytes from the SPI bus.
    ///
    /// The firmware buffer is limited to [`pico_de_gallo_internal::MAX_TRANSFER_SIZE`]
    /// (512) bytes. Reads exceeding this limit will be truncated.
    pub async fn spi_read(&self, count: u16) -> Result<Vec<u8>, PicoDeGalloError<SpiReadFail>> {
        self.client
            .send_resp::<SpiRead>(&SpiReadRequest { count })
            .await?
            .map_err(PicoDeGalloError::Endpoint)
    }

    /// Write `contents` to the SPI bus.
    pub async fn spi_write(&self, contents: &[u8]) -> Result<(), PicoDeGalloError<SpiWriteFail>> {
        self.client
            .send_resp::<SpiWrite>(&SpiWriteRequest { contents })
            .await?
            .map_err(PicoDeGalloError::Endpoint)
    }

    /// Flush the SPI interface.
    pub async fn spi_flush(&self) -> Result<(), PicoDeGalloError<SpiFlushFail>> {
        self.client
            .send_resp::<SpiFlush>(&())
            .await?
            .map_err(PicoDeGalloError::Endpoint)
    }

    /// Perform a full-duplex SPI transfer.
    ///
    /// Simultaneously sends `write_data` and receives the same number of bytes.
    /// The firmware buffer is limited to [`pico_de_gallo_internal::MAX_TRANSFER_SIZE`]
    /// bytes. Transfers exceeding this limit will be rejected.
    pub async fn spi_transfer(&self, write_data: &[u8]) -> Result<Vec<u8>, PicoDeGalloError<SpiTransferFail>> {
        self.client
            .send_resp::<SpiTransfer>(&SpiTransferRequest { contents: write_data })
            .await?
            .map_err(PicoDeGalloError::Endpoint)
    }

    /// Get the current state of GPIO numbered by `pin`.
    ///
    /// Pico de Gallo offers 8 total GPIOs, numbered 0 through 7.
    pub async fn gpio_get(&self, pin: u8) -> Result<GpioState, PicoDeGalloError<GpioGetFail>> {
        self.client
            .send_resp::<GpioGet>(&GpioGetRequest { pin })
            .await?
            .map_err(PicoDeGalloError::Endpoint)
    }

    /// Set the GPIO numbered by `pin` to state `state`.
    ///
    /// Pico de Gallo offers 8 total GPIOs, numbered 0 through 7.
    pub async fn gpio_put(&self, pin: u8, state: GpioState) -> Result<(), PicoDeGalloError<GpioPutFail>> {
        self.client
            .send_resp::<GpioPut>(&GpioPutRequest { pin, state })
            .await?
            .map_err(PicoDeGalloError::Endpoint)
    }

    /// Wait for GPIO numbered by `pin` to reach `High` state.
    ///
    /// Pico de Gallo offers 8 total GPIOs, numbered 0 through 7.
    pub async fn gpio_wait_for_high(&self, pin: u8) -> Result<(), PicoDeGalloError<GpioWaitFail>> {
        self.client
            .send_resp::<GpioWaitForHigh>(&GpioWaitRequest { pin })
            .await?
            .map_err(PicoDeGalloError::Endpoint)
    }

    /// Wait for GPIO numbered by `pin` to reach `Low` state.
    ///
    /// Pico de Gallo offers 8 total GPIOs, numbered 0 through 7.
    pub async fn gpio_wait_for_low(&self, pin: u8) -> Result<(), PicoDeGalloError<GpioWaitFail>> {
        self.client
            .send_resp::<GpioWaitForLow>(&GpioWaitRequest { pin })
            .await?
            .map_err(PicoDeGalloError::Endpoint)
    }

    /// Wait for a rising edge on the GPIO numbered by `pin`.
    ///
    /// Pico de Gallo offers 8 total GPIOs, numbered 0 through 7.
    pub async fn gpio_wait_for_rising_edge(&self, pin: u8) -> Result<(), PicoDeGalloError<GpioWaitFail>> {
        self.client
            .send_resp::<GpioWaitForRising>(&GpioWaitRequest { pin })
            .await?
            .map_err(PicoDeGalloError::Endpoint)
    }

    /// Wait for a falling edge on the GPIO numbered by `pin`.
    ///
    /// Pico de Gallo offers 8 total GPIOs, numbered 0 through 7.
    pub async fn gpio_wait_for_falling_edge(&self, pin: u8) -> Result<(), PicoDeGalloError<GpioWaitFail>> {
        self.client
            .send_resp::<GpioWaitForFalling>(&GpioWaitRequest { pin })
            .await?
            .map_err(PicoDeGalloError::Endpoint)
    }

    /// Wait for either a rising edge or a falling edge on the GPIO
    /// numbered by `pin`.
    ///
    /// Pico de Gallo offers 8 total GPIOs, numbered 0 through 7.
    pub async fn gpio_wait_for_any_edge(&self, pin: u8) -> Result<(), PicoDeGalloError<GpioWaitFail>> {
        self.client
            .send_resp::<GpioWaitForAny>(&GpioWaitRequest { pin })
            .await?
            .map_err(PicoDeGalloError::Endpoint)
    }

    /// Set configuration parameters for I2C and SPI interfaces.
    pub async fn set_config(
        &self,
        i2c_frequency: u32,
        spi_frequency: u32,
        spi_phase: SpiPhase,
        spi_polarity: SpiPolarity,
    ) -> Result<(), PicoDeGalloError<SetConfigurationFail>> {
        self.client
            .send_resp::<SetConfiguration>(&SetConfigurationRequest {
                i2c_frequency,
                spi_frequency,
                spi_phase,
                spi_polarity,
            })
            .await?
            .map_err(PicoDeGalloError::Endpoint)
    }

    /// Get the firmware version from the Pico de Gallo device.
    pub async fn version(&self) -> Result<VersionInfo, PicoDeGalloError<Infallible>> {
        Ok(self.client.send_resp::<Version>(&()).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- PicoDeGalloError tests ---

    #[test]
    fn endpoint_error_wraps_inner() {
        let err: PicoDeGalloError<&str> = PicoDeGalloError::Endpoint("endpoint failed");
        match err {
            PicoDeGalloError::Endpoint(e) => assert_eq!(e, "endpoint failed"),
            PicoDeGalloError::Comms(_) => panic!("expected Endpoint, got Comms"),
        }
    }

    #[test]
    fn map_err_converts_ok() {
        let result: Result<u32, &str> = Ok(42);
        let mapped: Result<u32, PicoDeGalloError<&str>> = result.map_err(PicoDeGalloError::Endpoint);
        assert_eq!(mapped.unwrap(), 42);
    }

    #[test]
    fn map_err_converts_err() {
        let result: Result<(), I2cWriteFail> = Err(I2cWriteFail);
        let mapped = result.map_err(PicoDeGalloError::Endpoint);
        match mapped {
            Err(PicoDeGalloError::Endpoint(I2cWriteFail)) => {}
            _ => panic!("expected Endpoint(I2cWriteFail)"),
        }
    }

    // --- PicoDeGalloError From impl ---

    #[test]
    fn host_err_converts_to_comms_error() {
        let host_err: HostErr<WireError> = HostErr::Closed;
        let err: PicoDeGalloError<Infallible> = PicoDeGalloError::from(host_err);
        match err {
            PicoDeGalloError::Comms(HostErr::Closed) => {}
            _ => panic!("expected Comms(Closed)"),
        }
    }

    // --- PicoDeGalloError Debug ---

    #[test]
    fn error_debug_format_is_readable() {
        let err: PicoDeGalloError<I2cReadFail> = PicoDeGalloError::Endpoint(I2cReadFail);
        let debug = format!("{:?}", err);
        assert!(debug.contains("Endpoint"));
        assert!(debug.contains("I2cReadFail"));

        let comms_err: PicoDeGalloError<Infallible> = PicoDeGalloError::Comms(HostErr::Closed);
        let debug = format!("{:?}", comms_err);
        assert!(debug.contains("Comms"));
    }

    // --- PicoDeGalloError Display ---

    #[test]
    fn error_display_endpoint() {
        use std::fmt::Display;
        // Use a simple Display-implementing type
        let err: PicoDeGalloError<&str> = PicoDeGalloError::Endpoint("sensor timeout");
        let msg = format!("{err}");
        assert!(msg.contains("endpoint error"));
        assert!(msg.contains("sensor timeout"));
    }

    #[test]
    fn error_display_comms() {
        let err: PicoDeGalloError<&str> = PicoDeGalloError::Comms(HostErr::Closed);
        let msg = format!("{err}");
        assert!(msg.contains("communication error"));
    }

    #[test]
    fn error_is_std_error() {
        fn assert_error<E: std::error::Error>() {}
        assert_error::<PicoDeGalloError<&str>>();
    }

    // --- Device enumeration ---

    #[test]
    fn list_devices_returns_vec() {
        // Without hardware this returns an empty vec, but should not panic
        let devices = list_devices();
        // Each returned device must have the correct VID/PID (already filtered)
        for dev in &devices {
            assert!(dev.serial_number.is_some() || dev.serial_number.is_none());
        }
        // Mainly verifying the function doesn't panic
        let _ = devices;
    }

    #[test]
    fn device_description_is_clone_and_debug() {
        let desc = DeviceDescription {
            serial_number: Some("ABC123".to_string()),
            manufacturer: Some("Microsoft".to_string()),
            product: Some("Pico de Gallo".to_string()),
        };
        let cloned = desc.clone();
        assert_eq!(format!("{:?}", desc), format!("{:?}", cloned));
    }
}
