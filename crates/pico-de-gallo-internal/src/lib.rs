#![cfg_attr(not(feature = "use-std"), no_std)]

use postcard_rpc::{endpoints, topics, TopicDirection};
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

pub const MICROSOFT_VID: u16 = 0x045e;
pub const PICO_DE_GALLO_PID: u16 = 0x067d;

// ---

pub type I2cWriteResponse = Result<(), I2cWriteFail>;

#[cfg(feature = "use-std")]
pub type I2cReadResponse<'a> = Result<Vec<u8>, I2cReadFail>;
#[cfg(not(feature = "use-std"))]
pub type I2cReadResponse<'a> = Result<&'a [u8], I2cReadFail>;

#[cfg(feature = "use-std")]
pub type I2cWriteReadResponse<'a> = Result<Vec<u8>, I2cWriteReadFail>;
#[cfg(not(feature = "use-std"))]
pub type I2cWriteReadResponse<'a> = Result<&'a [u8], I2cWriteReadFail>;

pub type SpiWriteResponse = Result<(), SpiWriteFail>;

#[cfg(feature = "use-std")]
pub type SpiReadResponse<'a> = Result<Vec<u8>, SpiReadFail>;
#[cfg(not(feature = "use-std"))]
pub type SpiReadResponse<'a> = Result<&'a [u8], SpiReadFail>;

pub type SpiFlushResponse = Result<(), SpiFlushFail>;
pub type GpioGetResponse = Result<GpioState, GpioGetFail>;
pub type GpioPutResponse = Result<(), GpioPutFail>;
pub type GpioWaitResponse = Result<(), GpioWaitFail>;
pub type SetConfigurationResponse = Result<(), SetConfigurationFail>;

endpoints! {
    list = ENDPOINT_LIST;
    | EndpointTy         | RequestTy               | ResponseTy               | Path                |
    | ----------         | ---------               | ----------               | ----                |
    | PingEndpoint       | u32                     | u32                      | "ping"              |
    | I2cRead            | I2cReadRequest          | I2cReadResponse<'a>      | "i2c/read"          |
    | I2cWrite           | I2cWriteRequest<'a>     | I2cWriteResponse         | "i2c/write"         |
    | I2cWriteRead       | I2cWriteReadRequest<'a> | I2cWriteReadResponse<'b> | "i2c/write-read"    |
    | SpiRead            | SpiReadRequest          | SpiReadResponse<'a>      | "spi/read"          |
    | SpiWrite           | SpiWriteRequest<'a>     | SpiWriteResponse         | "spi/write"         |
    | SpiFlush           | ()                      | SpiFlushResponse         | "spi/flush"         |
    | GpioGet            | GpioGetRequest          | GpioGetResponse          | "gpio/get"          |
    | GpioPut            | GpioPutRequest          | GpioPutResponse          | "gpio/put"          |
    | GpioWaitForHigh    | GpioWaitRequest         | GpioWaitResponse         | "gpio/wait-high"    |
    | GpioWaitForLow     | GpioWaitRequest         | GpioWaitResponse         | "gpio/wait-low"     |
    | GpioWaitForRising  | GpioWaitRequest         | GpioWaitResponse         | "gpio/wait-rising"  |
    | GpioWaitForFalling | GpioWaitRequest         | GpioWaitResponse         | "gpio/wait-falling" |
    | GpioWaitForAny     | GpioWaitRequest         | GpioWaitResponse         | "gpio/wait-any"     |
    | SetConfiguration   | SetConfigurationRequest | SetConfigurationResponse | "set-config"        |
    | Version            | ()                      | VersionInfo              | "version"           |
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

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct I2cWriteReadRequest<'a> {
    pub address: u8,
    pub contents: &'a [u8],
    pub count: u16,
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct I2cWriteReadFail;

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct I2cReadRequest {
    pub address: u8,
    pub count: u16,
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct I2cReadFail;

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct I2cWriteRequest<'a> {
    pub address: u8,
    pub contents: &'a [u8],
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct I2cWriteFail;

// --- SPI

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct SpiReadRequest {
    pub count: u16,
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct SpiReadFail;

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct SpiWriteRequest<'a> {
    pub contents: &'a [u8],
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct SpiWriteFail;

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct SpiFlushFail;

// --- GPIO

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct GpioGetRequest {
    pub pin: u8,
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct GpioGetFail;

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct GpioPutRequest {
    pub pin: u8,
    pub state: GpioState,
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct GpioPutFail;

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub enum GpioState {
    Low,
    High,
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct GpioWaitRequest {
    pub pin: u8,
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct GpioWaitFail;

// --- Set config

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct SetConfigurationRequest {
    pub i2c_frequency: u32,
    pub spi_frequency: u32,
    pub spi_phase: SpiPhase,
    pub spi_polarity: SpiPolarity,
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub enum SpiPhase {
    CaptureOnFirstTransition = 0,
    CaptureOnSecondTransition = 1,
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub enum SpiPolarity {
    IdleLow = 0,
    IdleHigh = 1,
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct SetConfigurationFail;

// --- Version
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct VersionInfo {
    pub major: u16,
    pub minor: u16,
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

    // --- Config round-trip tests ---

    #[test]
    fn set_configuration_request_round_trip() {
        let req = SetConfigurationRequest {
            i2c_frequency: 400_000,
            spi_frequency: 1_000_000,
            spi_phase: SpiPhase::CaptureOnSecondTransition,
            spi_polarity: SpiPolarity::IdleHigh,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: SetConfigurationRequest = from_bytes(&bytes).unwrap();
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

    // --- Fail type round-trip tests ---

    #[test]
    fn fail_types_round_trip() {
        let bytes = to_allocvec(&I2cReadFail).unwrap();
        let _: I2cReadFail = from_bytes(&bytes).unwrap();

        let bytes = to_allocvec(&I2cWriteFail).unwrap();
        let _: I2cWriteFail = from_bytes(&bytes).unwrap();

        let bytes = to_allocvec(&I2cWriteReadFail).unwrap();
        let _: I2cWriteReadFail = from_bytes(&bytes).unwrap();

        let bytes = to_allocvec(&SpiReadFail).unwrap();
        let _: SpiReadFail = from_bytes(&bytes).unwrap();

        let bytes = to_allocvec(&SpiWriteFail).unwrap();
        let _: SpiWriteFail = from_bytes(&bytes).unwrap();

        let bytes = to_allocvec(&SpiFlushFail).unwrap();
        let _: SpiFlushFail = from_bytes(&bytes).unwrap();

        let bytes = to_allocvec(&GpioGetFail).unwrap();
        let _: GpioGetFail = from_bytes(&bytes).unwrap();

        let bytes = to_allocvec(&GpioPutFail).unwrap();
        let _: GpioPutFail = from_bytes(&bytes).unwrap();

        let bytes = to_allocvec(&GpioWaitFail).unwrap();
        let _: GpioWaitFail = from_bytes(&bytes).unwrap();

        let bytes = to_allocvec(&SetConfigurationFail).unwrap();
        let _: SetConfigurationFail = from_bytes(&bytes).unwrap();
    }
}
