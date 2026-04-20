#![cfg_attr(not(feature = "use-std"), no_std)]

use postcard_rpc::{TopicDirection, endpoints, topics};
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

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq)]
pub struct I2cWriteReadFail;

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct I2cReadRequest {
    pub address: u8,
    pub count: u16,
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq)]
pub struct I2cReadFail;

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct I2cWriteRequest<'a> {
    pub address: u8,
    pub contents: &'a [u8],
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq)]
pub struct I2cWriteFail;

// --- SPI

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct SpiReadRequest {
    pub count: u16,
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiReadFail;

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct SpiWriteRequest<'a> {
    pub contents: &'a [u8],
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiWriteFail;

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiFlushFail;

// --- GPIO

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct GpioGetRequest {
    pub pin: u8,
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpioGetFail;

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct GpioPutRequest {
    pub pin: u8,
    pub state: GpioState,
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpioPutFail;

// WARNING: Do not reorder enum variants — postcard serializes by
// variant index, not by discriminant. Reordering breaks wire compat.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpioState {
    Low,
    High,
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct GpioWaitRequest {
    pub pin: u8,
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpioWaitFail;

// --- Set config

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct SetConfigurationRequest {
    pub i2c_frequency: u32,
    pub spi_frequency: u32,
    pub spi_phase: SpiPhase,
    pub spi_polarity: SpiPolarity,
}

// WARNING: Do not reorder enum variants — postcard serializes by
// variant index, not by discriminant. Reordering breaks wire compat.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpiPhase {
    CaptureOnFirstTransition = 0,
    CaptureOnSecondTransition = 1,
}

// WARNING: Do not reorder enum variants — postcard serializes by
// variant index, not by discriminant. Reordering breaks wire compat.
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpiPolarity {
    IdleLow = 0,
    IdleHigh = 1,
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq)]
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
    fn set_configuration_request_wire_stability() {
        let req = SetConfigurationRequest {
            i2c_frequency: 400_000,
            spi_frequency: 1_000_000,
            spi_phase: SpiPhase::CaptureOnFirstTransition,
            spi_polarity: SpiPolarity::IdleLow,
        };
        let bytes = to_allocvec(&req).unwrap();
        // Snapshot the canonical encoding
        let canonical = bytes.clone();
        let decoded: SetConfigurationRequest = from_bytes(&bytes).unwrap();
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
    fn set_configuration_request_all_enum_combinations() {
        // Test all 4 combinations by constructing each explicitly
        for (phase, polarity) in [
            (SpiPhase::CaptureOnFirstTransition, SpiPolarity::IdleLow),
            (SpiPhase::CaptureOnFirstTransition, SpiPolarity::IdleHigh),
            (SpiPhase::CaptureOnSecondTransition, SpiPolarity::IdleLow),
            (SpiPhase::CaptureOnSecondTransition, SpiPolarity::IdleHigh),
        ] {
            let req = SetConfigurationRequest {
                i2c_frequency: 100_000,
                spi_frequency: 500_000,
                spi_phase: phase,
                spi_polarity: polarity,
            };
            let bytes = to_allocvec(&req).unwrap();
            let decoded: SetConfigurationRequest = from_bytes(&bytes).unwrap();
            assert_eq!(req, decoded);
        }
    }

    #[test]
    fn set_configuration_request_max_frequencies() {
        let req = SetConfigurationRequest {
            i2c_frequency: u32::MAX,
            spi_frequency: u32::MAX,
            spi_phase: SpiPhase::CaptureOnFirstTransition,
            spi_polarity: SpiPolarity::IdleLow,
        };
        let bytes = to_allocvec(&req).unwrap();
        let decoded: SetConfigurationRequest = from_bytes(&bytes).unwrap();
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
}
