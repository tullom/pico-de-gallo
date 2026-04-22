#![no_std]
#![no_main]
//! Pico de Gallo firmware for the Raspberry Pi Pico 2 (RP2350).
//!
//! This firmware implements a USB bridge that exposes I2C, SPI, UART, GPIO,
//! PWM, ADC, and 1-Wire peripherals to a host computer via [postcard-rpc](https://docs.rs/postcard-rpc)
//! endpoints. It runs on the [Embassy](https://embassy.dev) async runtime.
//!
//! # Peripheral Mapping
//!
//! | Function | RP2350 Pins | Notes |
//! |----------|-------------|-------|
//! | UART0 TX | GPIO 0 | Buffered, interrupt-driven |
//! | UART0 RX | GPIO 1 | |
//! | I2C1 SDA | GPIO 2 | 7-bit addressing, async mode |
//! | I2C1 SCL | GPIO 3 | |
//! | SPI0 SCK | GPIO 6 | DMA-backed full-duplex |
//! | SPI0 TX  | GPIO 7 | |
//! | SPI0 RX  | GPIO 4 | |
//! | GPIO 0–3 | GPIO 8–11 | Input/output/edge-wait |
//! | PWM 0    | GPIO 12 | Slice 6 channel A |
//! | PWM 1    | GPIO 13 | Slice 6 channel B |
//! | PWM 2    | GPIO 14 | Slice 7 channel A |
//! | PWM 3    | GPIO 15 | Slice 7 channel B |
//! | 1-Wire   | GPIO 16 | PIO0/SM0, open-drain |
//! | ADC 0    | GPIO 26 | 12-bit, 0–3.3 V nominal |
//! | ADC 1    | GPIO 27 | 12-bit, 0–3.3 V nominal |
//! | ADC 2    | GPIO 28 | 12-bit, 0–3.3 V nominal |
//! | ADC 3    | GPIO 29 | 12-bit, 0–3.3 V nominal |
//! | USB      | Native USB | postcard-rpc transport |
//!
//! # Endpoints
//!
//! See [`pico_de_gallo_internal::ENDPOINT_LIST`] for the full list of
//! supported endpoints and their request/response types.
//!
//! # Buffer Size
//!
//! The firmware uses a shared buffer of [`MAX_TRANSFER_SIZE`](pico_de_gallo_internal::MAX_TRANSFER_SIZE)
//! (4096) bytes for I2C and SPI data. Requests exceeding this limit are
//! rejected with an error.

use core::sync::atomic::{AtomicU16, Ordering};
use defmt::{debug, info, warn};
use embassy_embedded_hal::SetConfig;
use embassy_executor::Spawner;
use embassy_rp::adc::{self, Adc};
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::ClockConfig;
use embassy_rp::gpio::{Flex, Level, Pull};
use embassy_rp::i2c::{self, I2c};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, I2C1, PIO0, SPI0, UART0, USB};
use embassy_rp::pio;
use embassy_rp::pio_programs::onewire::{PioOneWire, PioOneWireProgram, PioOneWireSearch};
use embassy_rp::pwm::{self, Pwm};
use embassy_rp::spi::{self, Phase, Polarity, Spi};
use embassy_rp::uart::{self, BufferedUart};
use embassy_rp::usb::Driver;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, with_timeout};
use embedded_io_async::{Read as AsyncRead, Write as AsyncWrite};
use fixed::traits::ToFixed;
// Direct embassy-sync dep required: postcard-rpc's WireStorage is generic over
// embassy_sync_0_7::blocking_mutex::raw::RawMutex, which is the same type as
// embassy-sync 0.7's RawMutex (they share the same crate version).
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_usb::{Config, UsbDevice};
use pico_de_gallo_internal::{
    ADC_NOMINAL_REFERENCE_MV, ADC_RESOLUTION_BITS, AdcChannel, AdcConfigurationInfo, AdcError, AdcGetConfiguration,
    AdcGetConfigurationResponse, AdcRead, AdcReadRequest, AdcReadResponse, ENDPOINT_LIST, GpioDirection, GpioEdge,
    GpioError, GpioEvent, GpioEventTopic, GpioGet, GpioGetRequest, GpioGetResponse, GpioPull, GpioPut, GpioPutRequest,
    GpioPutResponse, GpioSetConfiguration, GpioSetConfigurationRequest, GpioSetConfigurationResponse, GpioState,
    GpioSubscribe, GpioSubscribeRequest, GpioSubscribeResponse, GpioUnsubscribe, GpioUnsubscribeRequest,
    GpioUnsubscribeResponse, GpioWaitForAny, GpioWaitForFalling, GpioWaitForHigh, GpioWaitForLow, GpioWaitForRising,
    GpioWaitRequest, GpioWaitResponse, I2cBatch, I2cBatchError, I2cBatchOp, I2cBatchRequest, I2cBatchResponse,
    I2cError, I2cFrequency, I2cGetConfiguration, I2cGetConfigurationResponse, I2cRead, I2cReadRequest, I2cReadResponse,
    I2cScan, I2cScanRequest, I2cScanResponse, I2cSetConfiguration, I2cSetConfigurationRequest,
    I2cSetConfigurationResponse, I2cWrite, I2cWriteRead, I2cWriteReadRequest, I2cWriteReadResponse, I2cWriteRequest,
    I2cWriteResponse, MAX_BATCH_OPS, MAX_TRANSFER_SIZE, MICROSOFT_VID, NUM_ADC_GPIO_CHANNELS, NUM_PWM_CHANNELS,
    OneWireError, OneWireRead, OneWireReadRequest, OneWireReadResponse, OneWireReset, OneWireResetResponse,
    OneWireSearch, OneWireSearchNext, OneWireSearchResponse, OneWireWrite, OneWireWritePullup,
    OneWireWritePullupRequest, OneWireWritePullupResponse, OneWireWriteRequest, OneWireWriteResponse,
    PICO_DE_GALLO_PID, PingEndpoint, PwmConfigurationInfo, PwmDisable, PwmDisableRequest, PwmDisableResponse,
    PwmDutyCycleInfo, PwmEnable, PwmEnableRequest, PwmEnableResponse, PwmError, PwmGetConfiguration,
    PwmGetConfigurationRequest, PwmGetConfigurationResponse, PwmGetDutyCycle, PwmGetDutyCycleRequest,
    PwmGetDutyCycleResponse, PwmSetConfiguration, PwmSetConfigurationRequest, PwmSetConfigurationResponse,
    PwmSetDutyCycle, PwmSetDutyCycleRequest, PwmSetDutyCycleResponse, SpiBatch, SpiBatchError, SpiBatchOp,
    SpiBatchRequest, SpiBatchResponse, SpiConfigurationInfo, SpiError, SpiFlush, SpiFlushResponse, SpiGetConfiguration,
    SpiGetConfigurationResponse, SpiPhase, SpiPolarity, SpiRead, SpiReadRequest, SpiReadResponse, SpiSetConfiguration,
    SpiSetConfigurationRequest, SpiSetConfigurationResponse, SpiTransfer, SpiTransferRequest, SpiTransferResponse,
    SpiWrite, SpiWriteRequest, SpiWriteResponse, TOPICS_IN_LIST, TOPICS_OUT_LIST, UartConfigurationInfo, UartError,
    UartFlush, UartFlushResponse, UartGetConfiguration, UartGetConfigurationResponse, UartRead, UartReadRequest,
    UartReadResponse, UartSetConfiguration, UartSetConfigurationRequest, UartSetConfigurationResponse, UartWrite,
    UartWriteRequest, UartWriteResponse, Version, VersionInfo,
};
use pico_de_gallo_internal::{
    Capabilities, DeviceInfo, GetDeviceInfo, SCHEMA_VERSION_MAJOR, SCHEMA_VERSION_MINOR, SCHEMA_VERSION_PATCH,
};
use postcard_rpc::{
    define_dispatch,
    header::VarHeader,
    header::VarKeyKind,
    server::{
        Dispatch, Sender, Server,
        impls::embassy_usb_v0_5::{
            PacketBuffers,
            dispatch_impl::{WireRxBuf, WireRxImpl, WireSpawnImpl, WireStorage, WireTxImpl},
        },
    },
};
use static_cell::ConstStaticCell;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

/// Per-pin direction mode tracked by firmware.
///
/// Pins start in `LegacyAuto` mode, which preserves backward-compatible
/// behavior: `gpio_get` auto-switches to input, `gpio_put` auto-switches
/// to output. Once configured via `gpio/set-config`, the pin enters an
/// explicit mode and direction changes are no longer automatic.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PinMode {
    /// Default: auto-switch direction on get/put (backward compatible).
    LegacyAuto,
    /// Explicitly configured as input via `gpio/set-config`.
    ExplicitInput,
    /// Explicitly configured as output via `gpio/set-config`.
    ExplicitOutput,
}

/// Binary info entries for `picotool info` identification.
///
/// These entries are placed in the `.bi_entries` linker section and allow
/// `picotool` to display the firmware name, description, version, and
/// build info without running the firmware.
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"Pico de Gallo"),
    embassy_rp::binary_info::rp_program_description!(c"USB bridge to various buses such as I2C, SPI, UART, and ADC"),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

// auto-generated version information from Cargo.toml
include!(concat!(env!("OUT_DIR"), "/version.rs"));

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
    I2C1_IRQ => embassy_rp::i2c::InterruptHandler<I2C1>;
    DMA_IRQ_0 => embassy_rp::dma::InterruptHandler<DMA_CH0>, embassy_rp::dma::InterruptHandler<DMA_CH1>;
    UART0_IRQ => embassy_rp::uart::BufferedInterruptHandler<UART0>;
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO0>;
});

const NUM_GPIOS: usize = 4;
const NUM_PWM_SLICES: usize = 2;

/// System clock frequency in Hz.
///
/// Must match the `ClockConfig::system_freq()` value passed to `embassy_rp::init`.
const SYS_CLK_HZ: u32 = 150_000_000;

/// Number of ADC channels stored in Context (4 GPIO channels).
const NUM_ADC_CHANNELS: usize = NUM_ADC_GPIO_CHANNELS;

/// Firmware application context holding all peripheral handles.
///
/// NOTE: `buf` is shared between I2C and SPI handlers. This is safe because
/// postcard-rpc dispatches handlers serially (one at a time). If concurrent
/// dispatch is ever enabled, separate buffers would be required.
pub struct Context {
    i2c: I2c<'static, I2C1, i2c::Async>,
    spi: Spi<'static, SPI0, spi::Async>,
    uart: BufferedUart,
    gpios: [Option<Flex<'static>>; NUM_GPIOS],
    pin_modes: [PinMode; NUM_GPIOS],
    pwm_slices: [Pwm<'static>; NUM_PWM_SLICES],
    pwm_configs: [pwm::Config; NUM_PWM_SLICES],
    adc: Adc<'static, adc::Blocking>,
    adc_channels: [adc::Channel<'static>; NUM_ADC_CHANNELS],
    i2c_frequency: I2cFrequency,
    spi_frequency: u32,
    spi_phase: SpiPhase,
    spi_polarity: SpiPolarity,
    uart_baud_rate: u32,
    onewire: PioOneWire<'static, PIO0, 0>,
    onewire_search: PioOneWireSearch,
    buf: [u8; MAX_TRANSFER_SIZE],
}

impl Context {
    #[allow(clippy::too_many_arguments)]
    fn new(
        i2c: I2c<'static, I2C1, i2c::Async>,
        spi: Spi<'static, SPI0, spi::Async>,
        uart: BufferedUart,
        gpios: [Flex<'static>; NUM_GPIOS],
        pwm_slices: [Pwm<'static>; NUM_PWM_SLICES],
        pwm_configs: [pwm::Config; NUM_PWM_SLICES],
        adc: Adc<'static, adc::Blocking>,
        adc_channels: [adc::Channel<'static>; NUM_ADC_CHANNELS],
        onewire: PioOneWire<'static, PIO0, 0>,
    ) -> Self {
        let [g0, g1, g2, g3] = gpios;
        // Defaults match embassy-rp Config::default()
        Self {
            i2c,
            spi,
            uart,
            gpios: [Some(g0), Some(g1), Some(g2), Some(g3)],
            pin_modes: [PinMode::LegacyAuto; NUM_GPIOS],
            pwm_slices,
            pwm_configs,
            adc,
            adc_channels,
            i2c_frequency: I2cFrequency::Standard,
            spi_frequency: 1_000_000,
            spi_phase: SpiPhase::CaptureOnFirstTransition,
            spi_polarity: SpiPolarity::IdleLow,
            uart_baud_rate: 115_200,
            onewire,
            onewire_search: PioOneWireSearch::new(),
            buf: [0; MAX_TRANSFER_SIZE],
        }
    }
}

/// Helper macro to get a GPIO pin by index for input operations.
/// In `LegacyAuto` mode, auto-switches to input. In `ExplicitInput` mode,
/// uses the pin as-is. In `ExplicitOutput` mode, returns `WrongDirection`.
/// Returns `PinMonitored` if the pin is currently subscribed for event monitoring.
macro_rules! gpio_for_input {
    ($context:expr, $pin:expr) => {{
        let idx = usize::from($pin);
        let mode = *$context.pin_modes.get(idx).ok_or(GpioError::InvalidPin)?;
        let gpio = $context
            .gpios
            .get_mut(idx)
            .ok_or(GpioError::InvalidPin)?
            .as_mut()
            .ok_or(GpioError::PinMonitored)?;
        match mode {
            PinMode::LegacyAuto => gpio.set_as_input(),
            PinMode::ExplicitInput => {}
            PinMode::ExplicitOutput => return Err(GpioError::WrongDirection),
        }
        gpio
    }};
}

/// Maps an embassy-rp I2C error to our wire protocol error type.
fn map_i2c_error(e: i2c::Error) -> I2cError {
    match e {
        i2c::Error::Abort(i2c::AbortReason::NoAcknowledge) => I2cError::NoAcknowledge,
        i2c::Error::Abort(i2c::AbortReason::ArbitrationLoss) => I2cError::ArbitrationLoss,
        i2c::Error::Abort(i2c::AbortReason::TxNotEmpty(_)) => I2cError::Overrun,
        i2c::Error::Abort(i2c::AbortReason::Other(_)) => I2cError::Bus,
        i2c::Error::InvalidReadBufferLength => I2cError::BufferTooLong,
        i2c::Error::InvalidWriteBufferLength => I2cError::BufferTooLong,
        i2c::Error::AddressOutOfRange(_) => I2cError::AddressOutOfRange,
        #[allow(deprecated)]
        i2c::Error::AddressReserved(_) => I2cError::AddressOutOfRange,
    }
}

/// USB driver type for the RP2350.
type AppDriver = Driver<'static, USB>;
/// postcard-rpc wire storage with ThreadMode mutex.
type AppStorage = WireStorage<ThreadModeRawMutex, AppDriver, 256, 256, 64, 256>;
/// Packet buffer storage sized for [`MAX_TRANSFER_SIZE`] plus protocol overhead.
type BufStorage = PacketBuffers<{ MAX_TRANSFER_SIZE + 1024 }, { MAX_TRANSFER_SIZE + 1024 }>;
/// postcard-rpc transmit implementation.
type AppTx = WireTxImpl<ThreadModeRawMutex, AppDriver>;
/// postcard-rpc receive implementation.
type AppRx = WireRxImpl<AppDriver>;
/// The complete postcard-rpc server type.
type AppServer = Server<AppTx, AppRx, WireRxBuf, PicoDeGallo>;

static PBUFS: ConstStaticCell<BufStorage> = ConstStaticCell::new(BufStorage::new());
static STORAGE: AppStorage = AppStorage::new();

// --- GPIO event monitoring infrastructure ---

/// Channel for sending a (Flex, GpioEdge) pair to start monitoring pin `n`.
static GPIO_MONITOR_START: [Channel<CriticalSectionRawMutex, (Flex<'static>, GpioEdge), 1>; NUM_GPIOS] =
    [Channel::new(), Channel::new(), Channel::new(), Channel::new()];

/// Signal to stop monitoring pin `n`.
static GPIO_MONITOR_STOP: [Signal<CriticalSectionRawMutex, ()>; NUM_GPIOS] =
    [Signal::new(), Signal::new(), Signal::new(), Signal::new()];

/// Channel for returning the Flex pin after monitoring stops.
static GPIO_MONITOR_RETURN: [Channel<CriticalSectionRawMutex, Flex<'static>, 1>; NUM_GPIOS] =
    [Channel::new(), Channel::new(), Channel::new(), Channel::new()];

/// Ack channel: monitor task signals it is armed and listening.
static GPIO_MONITOR_ARMED: [Channel<CriticalSectionRawMutex, (), 1>; NUM_GPIOS] =
    [Channel::new(), Channel::new(), Channel::new(), Channel::new()];

/// Monotonic sequence counter for published GPIO events.
static GPIO_EVENT_SEQ: AtomicU16 = AtomicU16::new(0);

/// Background task that monitors GPIO pin `N` for edge events.
///
/// The task waits for a `(Flex, GpioEdge)` on `GPIO_MONITOR_START[N]`,
/// sends an armed ack, then loops detecting edges and publishing
/// `GpioEvent` topics until `GPIO_MONITOR_STOP[N]` is signalled.
/// After stopping, the `Flex` pin is returned via `GPIO_MONITOR_RETURN[N]`.
///
/// Edge detection is best-effort: if the pin changes faster than the
/// monitor loop cadence, intermediate transitions may be missed.
#[embassy_executor::task(pool_size = 4)]
async fn gpio_monitor_task(slot: usize, tx: AppTx, vkk: VarKeyKind) {
    let sender = Sender::new(tx, vkk);

    loop {
        // Wait for a subscribe request to hand us a pin
        let (mut flex, edge) = GPIO_MONITOR_START[slot].receive().await;

        // Configure as input for edge detection
        flex.set_as_input();

        // Signal that we are armed and listening
        GPIO_MONITOR_ARMED[slot].send(()).await;

        debug!("gpio monitor[{}]: armed, edge={=u8}", slot, edge as u8);

        // Monitor loop: wait for edge or stop signal
        loop {
            use embassy_futures::select::{Either, select};

            let edge_future = async {
                match edge {
                    GpioEdge::Rising => flex.wait_for_rising_edge().await,
                    GpioEdge::Falling => flex.wait_for_falling_edge().await,
                    GpioEdge::Any => flex.wait_for_any_edge().await,
                }
            };

            match select(edge_future, GPIO_MONITOR_STOP[slot].wait()).await {
                Either::First(()) => {
                    let state = match flex.get_level() {
                        Level::Low => GpioState::Low,
                        Level::High => GpioState::High,
                    };
                    let timestamp_us = Instant::now().as_micros();
                    let seq = GPIO_EVENT_SEQ.fetch_add(1, Ordering::Relaxed);

                    let event = GpioEvent {
                        pin: slot as u8,
                        edge,
                        state,
                        timestamp_us,
                    };

                    debug!(
                        "gpio monitor[{}]: edge detected, state={=u8}, ts={=u64}",
                        slot, state as u8, timestamp_us
                    );

                    let _ = sender.publish::<GpioEventTopic>(seq.into(), &event).await;
                }
                Either::Second(()) => {
                    debug!("gpio monitor[{}]: stop requested", slot);
                    GPIO_MONITOR_RETURN[slot].send(flex).await;
                    break;
                }
            }
        }
    }
}

/// Build the USB device configuration.
///
/// Reads the chip unique ID from OTP to generate a deterministic serial
/// number string. Falls back to all-zeros if OTP read fails.
fn usb_config() -> Config<'static> {
    // Obtain the chip unique ID for USB serial number.
    // Falls back to "UNKNOWN" if OTP read fails (e.g. on some dev boards).
    let unique_id: u64 = embassy_rp::otp::get_chipid().unwrap_or_else(|e| {
        warn!("Failed to read chip ID: {:?}, using fallback serial", e);
        0
    });

    static SERIAL_STRING: StaticCell<[u8; 16]> = StaticCell::new();
    let mut ser_buf = [b'0'; 16];
    unique_id
        .to_be_bytes()
        .iter()
        .zip(ser_buf.chunks_exact_mut(2))
        .for_each(|(b, chs)| {
            let mut b = *b;
            for c in chs {
                *c = match b >> 4 {
                    v @ 0..10 => b'0' + v,
                    v @ 10..16 => b'A' + (v - 10),
                    _ => b'X',
                };
                b <<= 4;
            }
        });
    let ser_buf = SERIAL_STRING.init(ser_buf);
    // Safety: ser_buf contains only ASCII hex digits, which are valid UTF-8.
    let ser_buf = core::str::from_utf8(ser_buf.as_slice()).unwrap();

    let mut config = Config::new(MICROSOFT_VID, PICO_DE_GALLO_PID);
    config.manufacturer = Some("Microsoft");
    config.product = Some("Pico de Gallo");
    config.serial_number = Some(ser_buf);
    config.max_power = 100;
    config.max_packet_size_0 = 64;
    config.self_powered = false;
    config.composite_with_iads = false;
    config.device_class = 0xff;
    config.device_sub_class = 0xff;
    config.device_protocol = 0xff;

    config
}

define_dispatch! {
    app: PicoDeGallo;
    spawn_fn: spawn_fn;
    tx_impl: AppTx;
    spawn_impl: WireSpawnImpl;
    context: Context;

    endpoints: {
        list: ENDPOINT_LIST;

        | EndpointTy           | kind     | handler                       |
        | ----------           | ----     | -------                       |
        | PingEndpoint         | blocking | ping_handler                  |
        | I2cRead              | async    | i2c_read_handler              |
        | I2cWrite             | async    | i2c_write_handler             |
        | I2cWriteRead         | async    | i2c_write_read_handler        |
        | SpiRead              | async    | spi_read_handler              |
        | SpiWrite             | async    | spi_write_handler             |
        | SpiFlush             | async    | spi_flush_handler             |
        | SpiTransfer          | async    | spi_transfer_handler          |
        | GpioGet              | async    | gpio_get_handler              |
        | GpioPut              | async    | gpio_put_handler              |
        | GpioWaitForHigh      | async    | gpio_wait_for_high_handler    |
        | GpioWaitForLow       | async    | gpio_wait_for_low_handler     |
        | GpioWaitForRising    | async    | gpio_wait_for_rising_handler  |
        | GpioWaitForFalling   | async    | gpio_wait_for_falling_handler |
        | GpioWaitForAny       | async    | gpio_wait_for_any_handler     |
        | GpioSetConfiguration | async    | gpio_set_config_handler       |
        | GpioSubscribe        | async    | gpio_subscribe_handler        |
        | GpioUnsubscribe      | async    | gpio_unsubscribe_handler      |
        | I2cSetConfiguration  | async    | i2c_set_config_handler        |
        | I2cScan              | async    | i2c_scan_handler              |
        | I2cBatch             | async    | i2c_batch_handler             |
        | SpiSetConfiguration  | async    | spi_set_config_handler        |
        | I2cGetConfiguration  | blocking | i2c_get_config_handler        |
        | SpiGetConfiguration  | blocking | spi_get_config_handler        |
        | SpiBatch             | async    | spi_batch_handler             |
        | UartRead             | async    | uart_read_handler             |
        | UartWrite            | async    | uart_write_handler            |
        | UartFlush            | async    | uart_flush_handler            |
        | UartSetConfiguration | async    | uart_set_config_handler       |
        | UartGetConfiguration | blocking | uart_get_config_handler       |
        | PwmSetDutyCycle      | blocking | pwm_set_duty_cycle_handler    |
        | PwmGetDutyCycle      | blocking | pwm_get_duty_cycle_handler    |
        | PwmEnable            | blocking | pwm_enable_handler            |
        | PwmDisable           | blocking | pwm_disable_handler           |
        | PwmSetConfiguration  | blocking | pwm_set_config_handler        |
        | PwmGetConfiguration  | blocking | pwm_get_config_handler        |
        | AdcRead              | blocking | adc_read_handler              |
        | AdcGetConfiguration  | blocking | adc_get_config_handler        |
        | OneWireReset         | async    | onewire_reset_handler         |
        | OneWireRead          | async    | onewire_read_handler          |
        | OneWireWrite         | async    | onewire_write_handler         |
        | OneWireWritePullup   | async    | onewire_write_pullup_handler  |
        | OneWireSearch        | async    | onewire_search_handler        |
        | OneWireSearchNext    | async    | onewire_search_next_handler   |
        | Version              | async    | version_handler               |
        | GetDeviceInfo        | blocking | device_info_handler           |
    };
    topics_in: {
        list: TOPICS_IN_LIST;

        | TopicTy                   | kind      | handler                 |
        | ----------                | ----      | -------                 |
    };
    topics_out: {
        list: TOPICS_OUT_LIST;
    };
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let config = embassy_rp::config::Config::new(ClockConfig::system_freq(150_000_000).unwrap());
    let p = embassy_rp::init(config);

    // USB/RPC INIT
    let driver = Driver::new(p.USB, Irqs);
    let pbufs = PBUFS.take();
    let config = usb_config();

    let i2c = embassy_rp::i2c::I2c::new_async(p.I2C1, p.PIN_3, p.PIN_2, Irqs, embassy_rp::i2c::Config::default());
    let spi = embassy_rp::spi::Spi::new(
        p.SPI0,
        p.PIN_6,
        p.PIN_7,
        p.PIN_4,
        p.DMA_CH0,
        p.DMA_CH1,
        Irqs,
        embassy_rp::spi::Config::default(),
    );

    let gpios = [
        Flex::new(p.PIN_8),
        Flex::new(p.PIN_9),
        Flex::new(p.PIN_10),
        Flex::new(p.PIN_11),
    ];

    // PWM on GPIO12-15 (slices 6-7)
    let pwm_config = pwm::Config::default();
    let pwm_slices = [
        Pwm::new_output_ab(p.PWM_SLICE6, p.PIN_12, p.PIN_13, pwm_config.clone()),
        Pwm::new_output_ab(p.PWM_SLICE7, p.PIN_14, p.PIN_15, pwm_config.clone()),
    ];
    let pwm_configs = [pwm::Config::default(), pwm::Config::default()];

    // UART0 on GPIO0 (TX) and GPIO1 (RX) — default Pico 2 header pins
    static UART_TX_BUF: StaticCell<[u8; 1024]> = StaticCell::new();
    static UART_RX_BUF: StaticCell<[u8; 1024]> = StaticCell::new();
    let uart_tx_buf = UART_TX_BUF.init([0u8; 1024]);
    let uart_rx_buf = UART_RX_BUF.init([0u8; 1024]);
    let uart = BufferedUart::new(
        p.UART0,
        p.PIN_0,
        p.PIN_1,
        Irqs,
        uart_tx_buf,
        uart_rx_buf,
        uart::Config::default(),
    );

    // ADC — blocking mode for single-shot reads (GPIO26–29)
    let adc = Adc::new_blocking(p.ADC, adc::Config::default());
    let adc_channels = [
        adc::Channel::new_pin(p.PIN_26, Pull::None),
        adc::Channel::new_pin(p.PIN_27, Pull::None),
        adc::Channel::new_pin(p.PIN_28, Pull::None),
        adc::Channel::new_pin(p.PIN_29, Pull::None),
    ];

    // 1-Wire — PIO0/SM0 on GPIO16
    let pio::Pio { mut common, sm0, .. } = pio::Pio::new(p.PIO0, Irqs);
    static OW_PROGRAM: StaticCell<PioOneWireProgram<'static, PIO0>> = StaticCell::new();
    let ow_program = OW_PROGRAM.init(PioOneWireProgram::new(&mut common));
    let onewire = PioOneWire::new(&mut common, sm0, p.PIN_16, ow_program);

    let context = Context::new(
        i2c,
        spi,
        uart,
        gpios,
        pwm_slices,
        pwm_configs,
        adc,
        adc_channels,
        onewire,
    );

    let (device, tx_impl, rx_impl) = STORAGE.init(
        driver,
        config,
        pbufs.tx_buf.as_mut_slice(),
        postcard_rpc::server::impls::embassy_usb_v0_5::USB_FS_MAX_PACKET_SIZE,
    );
    let dispatcher = PicoDeGallo::new(context, spawner.into());
    let vkk = dispatcher.min_key_len();

    // Spawn GPIO monitor tasks before creating the server.
    // EUsbWireTx is Clone, so we clone tx_impl for each task.
    for slot in 0..NUM_GPIOS {
        spawner.must_spawn(gpio_monitor_task(slot, tx_impl.clone(), vkk));
    }

    let mut server: AppServer = Server::new(tx_impl, rx_impl, pbufs.rx_buf.as_mut_slice(), dispatcher, vkk);
    spawner.must_spawn(usb_task(device));

    loop {
        // If the host disconnects, we'll return an error here.
        // If this happens, just wait until the host reconnects
        let _ = server.run().await;
    }
}

/// This handles the low level USB management
#[embassy_executor::task]
pub async fn usb_task(mut usb: UsbDevice<'static, AppDriver>) {
    usb.run().await;
}

// --- Handlers ---

/// Handler for the `ping` endpoint — echoes back the received `u32`.
fn ping_handler(_context: &mut Context, _header: VarHeader, rqst: u32) -> u32 {
    info!("ping: {=u32:#x}", rqst);
    rqst
}

/// Handler for `i2c/read` — reads bytes from an I2C slave.
async fn i2c_read_handler<'a>(
    context: &'a mut Context,
    _header: VarHeader,
    req: I2cReadRequest,
) -> I2cReadResponse<'a> {
    let count = usize::from(req.count);
    if count > MAX_TRANSFER_SIZE {
        warn!("i2c read: requested count {} exceeds buffer", count);
        return Err(I2cError::BufferTooLong);
    }

    debug!("i2c read: addr={=u8:#x} count={=usize}", req.address, count);
    let buf = &mut context.buf[..count];
    context.i2c.read_async(req.address, buf).await.map_err(map_i2c_error)?;
    Ok(&context.buf[..count])
}

/// Handler for `i2c/write` — writes bytes to an I2C slave.
async fn i2c_write_handler<'a>(
    context: &mut Context,
    _header: VarHeader,
    req: I2cWriteRequest<'a>,
) -> I2cWriteResponse {
    debug!("i2c write: addr={=u8:#x} len={=usize}", req.address, req.contents.len());
    context
        .i2c
        .write_async(req.address, req.contents.iter().copied())
        .await
        .map_err(map_i2c_error)
}

/// Handler for `i2c/write-read` — writes then reads in a single I2C transaction.
async fn i2c_write_read_handler<'a>(
    context: &'a mut Context,
    _header: VarHeader,
    req: I2cWriteReadRequest<'a>,
) -> I2cWriteReadResponse<'a> {
    let count = usize::from(req.count);
    if count > MAX_TRANSFER_SIZE {
        warn!("i2c write_read: requested count {} exceeds buffer", count);
        return Err(I2cError::BufferTooLong);
    }

    debug!(
        "i2c write_read: addr={=u8:#x} write_len={=usize} read_count={=usize}",
        req.address,
        req.contents.len(),
        count
    );
    let buf = &mut context.buf[..count];
    context
        .i2c
        .write_read_async(req.address, req.contents.iter().copied(), buf)
        .await
        .map_err(map_i2c_error)?;
    Ok(&context.buf[..count])
}

/// First standard (non-reserved) 7-bit I2C address.
const I2C_ADDR_FIRST: u8 = 0x08;
/// Last standard (non-reserved) 7-bit I2C address.
const I2C_ADDR_LAST: u8 = 0x77;

/// Handler for `i2c/scan` — probes I2C addresses and returns those that ACK.
async fn i2c_scan_handler<'a>(
    context: &'a mut Context,
    _header: VarHeader,
    req: I2cScanRequest,
) -> I2cScanResponse<'a> {
    let (start, end) = if req.include_reserved {
        (0x00u8, 0x7Fu8)
    } else {
        (I2C_ADDR_FIRST, I2C_ADDR_LAST)
    };

    debug!("i2c scan: range={=u8:#x}..={=u8:#x}", start, end);

    let mut found = 0usize;

    for addr in start..=end {
        // Probe by attempting a 1-byte read. ACK means a device is present.
        let mut probe_buf = [0u8];
        if context.i2c.read_async(addr, &mut probe_buf).await.is_ok() {
            if found >= MAX_TRANSFER_SIZE {
                break;
            }
            context.buf[found] = addr;
            found += 1;
        }
    }

    debug!("i2c scan: found {=usize} device(s)", found);
    Ok(&context.buf[..found])
}

/// Handler for `i2c/batch` — executes multiple I2C operations in one USB transfer.
///
/// Decodes postcard-serialized ops and executes each operation sequentially.
/// Read data is accumulated in `context.buf`. If any operation fails,
/// subsequent operations are skipped and the error includes the index of
/// the failed operation.
async fn i2c_batch_handler<'a>(
    context: &'a mut Context,
    _header: VarHeader,
    req: I2cBatchRequest<'a>,
) -> I2cBatchResponse<'a> {
    let ops = req.ops;
    let count = req.count as usize;

    // Pre-validate op count
    if count > MAX_BATCH_OPS {
        return Err(I2cBatchError {
            failed_op: 0,
            kind: I2cError::BufferTooLong,
        });
    }

    // Pre-validate: walk the ops to compute total read length
    let mut total_read = 0usize;
    let mut remaining = ops;
    let mut validated = 0usize;
    while !remaining.is_empty() {
        let (op, rest) = postcard::take_from_bytes::<I2cBatchOp>(remaining).map_err(|_| I2cBatchError {
            failed_op: validated as u16,
            kind: I2cError::Other,
        })?;
        match op {
            I2cBatchOp::Read { len } => total_read += len as usize,
            I2cBatchOp::Write { .. } => {}
        }
        remaining = rest;
        validated += 1;
    }
    if validated != count {
        return Err(I2cBatchError {
            failed_op: 0,
            kind: I2cError::Other,
        });
    }
    if total_read > MAX_TRANSFER_SIZE {
        return Err(I2cBatchError {
            failed_op: 0,
            kind: I2cError::BufferTooLong,
        });
    }

    debug!(
        "i2c batch: addr={=u8:#x} ops={=usize} total_read={=usize}",
        req.address, count, total_read
    );

    // Execute ops
    let mut remaining = ops;
    let mut read_offset = 0usize;
    let mut op_index = 0u16;

    while !remaining.is_empty() {
        let (op, rest) = postcard::take_from_bytes::<I2cBatchOp>(remaining).unwrap();
        remaining = rest;

        match op {
            I2cBatchOp::Read { len } => {
                let len = len as usize;
                let buf = &mut context.buf[read_offset..read_offset + len];
                context
                    .i2c
                    .read_async(req.address, buf)
                    .await
                    .map_err(|e| I2cBatchError {
                        failed_op: op_index,
                        kind: map_i2c_error(e),
                    })?;
                read_offset += len;
            }
            I2cBatchOp::Write { data } => {
                context
                    .i2c
                    .write_async(req.address, data.iter().copied())
                    .await
                    .map_err(|e| I2cBatchError {
                        failed_op: op_index,
                        kind: map_i2c_error(e),
                    })?;
            }
        }
        op_index += 1;
    }

    Ok(&context.buf[..read_offset])
}

/// Handler for `spi/read` — reads bytes from the SPI bus.
async fn spi_read_handler<'a>(
    context: &'a mut Context,
    _header: VarHeader,
    req: SpiReadRequest,
) -> SpiReadResponse<'a> {
    let count = usize::from(req.count);
    if count > MAX_TRANSFER_SIZE {
        warn!("spi read: requested count {} exceeds buffer", count);
        return Err(SpiError::BufferTooLong);
    }

    debug!("spi read: count={=usize}", count);
    let buf = &mut context.buf[..count];
    context.spi.read(buf).await.map_err(|_| SpiError::Other)?;
    Ok(&context.buf[..count])
}

/// Handler for `spi/write` — writes bytes to the SPI bus.
async fn spi_write_handler<'a>(
    context: &mut Context,
    _header: VarHeader,
    req: SpiWriteRequest<'a>,
) -> SpiWriteResponse {
    debug!("spi write: len={=usize}", req.contents.len());
    context.spi.write(req.contents).await.map_err(|_| SpiError::Other)
}

/// Handler for `spi/flush` — flushes the SPI interface.
async fn spi_flush_handler(context: &mut Context, _header: VarHeader, _req: ()) -> SpiFlushResponse {
    debug!("spi flush");
    context.spi.flush().map_err(|_| SpiError::Other)
}

/// Handler for `spi/transfer` — performs a full-duplex SPI transfer via DMA.
async fn spi_transfer_handler<'a>(
    context: &'a mut Context,
    _header: VarHeader,
    req: SpiTransferRequest<'a>,
) -> SpiTransferResponse<'a> {
    let len = req.contents.len();
    if len > MAX_TRANSFER_SIZE {
        warn!("spi transfer: requested len {} exceeds buffer", len);
        return Err(SpiError::BufferTooLong);
    }

    debug!("spi transfer: len={=usize}", len);
    let buf = &mut context.buf[..len];
    context
        .spi
        .transfer(buf, req.contents)
        .await
        .map_err(|_| SpiError::Other)?;
    Ok(&context.buf[..len])
}

/// Handler for `spi/batch` — executes multiple SPI operations atomically under CS.
///
/// The firmware asserts CS on the specified GPIO pin before executing
/// operations, and deasserts it after completion (even on error).
/// Read and Transfer data is accumulated in `context.buf`.
async fn spi_batch_handler<'a>(
    context: &'a mut Context,
    _header: VarHeader,
    req: SpiBatchRequest<'a>,
) -> SpiBatchResponse<'a> {
    let ops = req.ops;
    let count = req.count as usize;
    let cs_idx = usize::from(req.cs_pin);

    // Pre-validate op count
    if count > MAX_BATCH_OPS {
        return Err(SpiBatchError {
            failed_op: 0,
            kind: SpiError::BufferTooLong,
        });
    }

    // Pre-validate: walk the ops to compute total read length
    let mut total_read = 0usize;
    let mut remaining = ops;
    let mut validated = 0usize;
    while !remaining.is_empty() {
        let (op, rest) = postcard::take_from_bytes::<SpiBatchOp>(remaining).map_err(|_| SpiBatchError {
            failed_op: validated as u16,
            kind: SpiError::Other,
        })?;
        match op {
            SpiBatchOp::Read { len } => total_read += len as usize,
            SpiBatchOp::Transfer { data } => total_read += data.len(),
            _ => {}
        }
        remaining = rest;
        validated += 1;
    }
    if validated != count {
        return Err(SpiBatchError {
            failed_op: 0,
            kind: SpiError::Other,
        });
    }
    if total_read > MAX_TRANSFER_SIZE {
        return Err(SpiBatchError {
            failed_op: 0,
            kind: SpiError::BufferTooLong,
        });
    }

    // Validate and get the CS pin
    let cs = context
        .gpios
        .get_mut(cs_idx)
        .ok_or(SpiBatchError {
            failed_op: 0,
            kind: SpiError::Other,
        })?
        .as_mut()
        .ok_or(SpiBatchError {
            failed_op: 0,
            kind: SpiError::Other,
        })?;
    cs.set_as_output();
    cs.set_high();

    debug!(
        "spi batch: cs_pin={=u8} ops={=usize} total_read={=usize}",
        req.cs_pin, count, total_read
    );

    // Assert CS (active low)
    let cs = context.gpios[cs_idx].as_mut().unwrap();
    cs.set_low();

    let result = spi_batch_execute(&mut context.spi, &mut context.buf, ops).await;

    // Deassert CS (always, even on error)
    let cs = context.gpios[cs_idx].as_mut().unwrap();
    cs.set_high();

    result
}

/// Inner execution loop for SPI batch, separated so CS can be
/// reliably deasserted in the caller regardless of outcome.
async fn spi_batch_execute<'a>(
    spi: &mut Spi<'static, SPI0, spi::Async>,
    buf: &'a mut [u8; MAX_TRANSFER_SIZE],
    ops: &[u8],
) -> Result<&'a [u8], SpiBatchError> {
    let mut remaining = ops;
    let mut read_offset = 0usize;
    let mut op_index = 0u16;

    while !remaining.is_empty() {
        let (op, rest) = postcard::take_from_bytes::<SpiBatchOp>(remaining).unwrap();
        remaining = rest;

        match op {
            SpiBatchOp::Read { len } => {
                let len = len as usize;
                let slice = &mut buf[read_offset..read_offset + len];
                spi.read(slice).await.map_err(|_| SpiBatchError {
                    failed_op: op_index,
                    kind: SpiError::Other,
                })?;
                read_offset += len;
            }
            SpiBatchOp::Write { data } => {
                spi.write(data).await.map_err(|_| SpiBatchError {
                    failed_op: op_index,
                    kind: SpiError::Other,
                })?;
            }
            SpiBatchOp::Transfer { data } => {
                let len = data.len();
                let slice = &mut buf[read_offset..read_offset + len];
                spi.transfer(slice, data).await.map_err(|_| SpiBatchError {
                    failed_op: op_index,
                    kind: SpiError::Other,
                })?;
                read_offset += len;
            }
            SpiBatchOp::DelayNs { ns } => {
                embassy_time::Timer::after(Duration::from_nanos(ns as u64)).await;
            }
        }
        op_index += 1;
    }

    Ok(&buf[..read_offset])
}

/// Handler for `gpio/get` — reads the current logic level of a pin.
async fn gpio_get_handler(context: &mut Context, _header: VarHeader, req: GpioGetRequest) -> GpioGetResponse {
    let gpio = gpio_for_input!(context, req.pin);
    debug!("gpio get: pin={=u8}", req.pin);
    match gpio.get_level() {
        Level::Low => Ok(GpioState::Low),
        Level::High => Ok(GpioState::High),
    }
}

/// Handler for `gpio/put` — sets a GPIO pin to the requested level.
async fn gpio_put_handler(context: &mut Context, _header: VarHeader, req: GpioPutRequest) -> GpioPutResponse {
    let idx = usize::from(req.pin);
    let mode = *context.pin_modes.get(idx).ok_or(GpioError::InvalidPin)?;
    let gpio = context
        .gpios
        .get_mut(idx)
        .ok_or(GpioError::InvalidPin)?
        .as_mut()
        .ok_or(GpioError::PinMonitored)?;

    match mode {
        PinMode::LegacyAuto => gpio.set_as_output(),
        PinMode::ExplicitOutput => {}
        PinMode::ExplicitInput => return Err(GpioError::WrongDirection),
    }

    let level = match req.state {
        GpioState::Low => Level::Low,
        GpioState::High => Level::High,
    };

    debug!("gpio put: pin={=u8} level={=u8}", req.pin, level as u8);
    gpio.set_level(level);

    Ok(())
}

/// Handler for `gpio/wait-high` — blocks until the pin goes high.
async fn gpio_wait_for_high_handler(
    context: &mut Context,
    _header: VarHeader,
    req: GpioWaitRequest,
) -> GpioWaitResponse {
    let gpio = gpio_for_input!(context, req.pin);
    debug!("gpio wait_for_high: pin={=u8}", req.pin);
    gpio.wait_for_high().await;
    Ok(())
}

/// Handler for `gpio/wait-low` — blocks until the pin goes low.
async fn gpio_wait_for_low_handler(
    context: &mut Context,
    _header: VarHeader,
    req: GpioWaitRequest,
) -> GpioWaitResponse {
    let gpio = gpio_for_input!(context, req.pin);
    debug!("gpio wait_for_low: pin={=u8}", req.pin);
    gpio.wait_for_low().await;
    Ok(())
}

/// Handler for `gpio/wait-rising` — blocks until a rising edge.
async fn gpio_wait_for_rising_handler(
    context: &mut Context,
    _header: VarHeader,
    req: GpioWaitRequest,
) -> GpioWaitResponse {
    let gpio = gpio_for_input!(context, req.pin);
    debug!("gpio wait_for_rising: pin={=u8}", req.pin);
    gpio.wait_for_rising_edge().await;
    Ok(())
}

/// Handler for `gpio/wait-falling` — blocks until a falling edge.
async fn gpio_wait_for_falling_handler(
    context: &mut Context,
    _header: VarHeader,
    req: GpioWaitRequest,
) -> GpioWaitResponse {
    let gpio = gpio_for_input!(context, req.pin);
    debug!("gpio wait_for_falling: pin={=u8}", req.pin);
    gpio.wait_for_falling_edge().await;
    Ok(())
}

/// Handler for `gpio/wait-any` — blocks until any edge.
async fn gpio_wait_for_any_handler(
    context: &mut Context,
    _header: VarHeader,
    req: GpioWaitRequest,
) -> GpioWaitResponse {
    let gpio = gpio_for_input!(context, req.pin);
    debug!("gpio wait_for_any: pin={=u8}", req.pin);
    gpio.wait_for_any_edge().await;
    Ok(())
}

/// Handler for `gpio/set-config` — configures a pin's direction and pull resistor.
///
/// Once configured, the pin enters explicit mode and `gpio_get`/`gpio_put` will
/// no longer auto-switch direction. To restore auto-switching, reset the firmware.
async fn gpio_set_config_handler(
    context: &mut Context,
    _header: VarHeader,
    req: GpioSetConfigurationRequest,
) -> GpioSetConfigurationResponse {
    let idx = usize::from(req.pin);
    let mode = context.pin_modes.get_mut(idx).ok_or(GpioError::InvalidPin)?;
    let gpio = context
        .gpios
        .get_mut(idx)
        .ok_or(GpioError::InvalidPin)?
        .as_mut()
        .ok_or(GpioError::PinMonitored)?;

    // Apply pull resistor setting
    gpio.set_pull(match req.pull {
        GpioPull::None => Pull::None,
        GpioPull::Up => Pull::Up,
        GpioPull::Down => Pull::Down,
    });

    // Apply direction and update tracked mode
    match req.direction {
        GpioDirection::Input => {
            gpio.set_as_input();
            *mode = PinMode::ExplicitInput;
        }
        GpioDirection::Output => {
            gpio.set_as_output();
            *mode = PinMode::ExplicitOutput;
        }
    }

    debug!(
        "gpio set_config: pin={=u8} dir={=u8} pull={=u8}",
        req.pin, req.direction as u8, req.pull as u8
    );
    Ok(())
}

/// Handler for `gpio/subscribe` — starts edge-event monitoring on a pin.
///
/// Takes ownership of the pin from Context, sends it to a background monitor
/// task, and waits for the armed acknowledgement before returning. While
/// subscribed, the pin cannot be used by other GPIO operations.
async fn gpio_subscribe_handler(
    context: &mut Context,
    _header: VarHeader,
    req: GpioSubscribeRequest,
) -> GpioSubscribeResponse {
    let idx = usize::from(req.pin);
    if idx >= NUM_GPIOS {
        return Err(GpioError::InvalidPin);
    }

    let flex = context.gpios[idx].take().ok_or(GpioError::PinMonitored)?;

    debug!("gpio subscribe: pin={=u8} edge={=u8}", req.pin, req.edge as u8);

    GPIO_MONITOR_START[idx].send((flex, req.edge)).await;
    GPIO_MONITOR_ARMED[idx].receive().await;

    Ok(())
}

/// Handler for `gpio/unsubscribe` — stops edge-event monitoring on a pin.
///
/// Signals the monitor task to stop, waits for the pin to be returned, and
/// puts it back into Context so regular GPIO operations can resume.
async fn gpio_unsubscribe_handler(
    context: &mut Context,
    _header: VarHeader,
    req: GpioUnsubscribeRequest,
) -> GpioUnsubscribeResponse {
    let idx = usize::from(req.pin);
    if idx >= NUM_GPIOS {
        return Err(GpioError::InvalidPin);
    }

    if context.gpios[idx].is_some() {
        return Err(GpioError::PinNotMonitored);
    }

    debug!("gpio unsubscribe: pin={=u8}", req.pin);

    GPIO_MONITOR_STOP[idx].signal(());
    let flex = GPIO_MONITOR_RETURN[idx].receive().await;
    context.gpios[idx] = Some(flex);

    Ok(())
}

/// Handler for `i2c/set-config` — reconfigures I2C bus parameters.
async fn i2c_set_config_handler(
    context: &mut Context,
    _header: VarHeader,
    req: I2cSetConfigurationRequest,
) -> I2cSetConfigurationResponse {
    let frequency = match req.frequency {
        I2cFrequency::Standard => 100_000,
        I2cFrequency::Fast => 400_000,
        I2cFrequency::FastPlus => 1_000_000,
    };

    let mut i2c_config = i2c::Config::default();
    i2c_config.frequency = frequency;

    debug!("i2c_set_config: freq={=u32}", frequency);
    context
        .i2c
        .set_config(&i2c_config)
        .map(|_| {
            context.i2c_frequency = req.frequency;
        })
        .map_err(|_| I2cError::Other)
}

/// Handler for `spi/set-config` — reconfigures SPI bus parameters.
///
/// Validates the requested frequency before applying. The RP2350 SPI peripheral
/// requires a non-zero frequency no greater than half the peripheral clock
/// (75 MHz at the default 150 MHz system clock).
async fn spi_set_config_handler(
    context: &mut Context,
    _header: VarHeader,
    req: SpiSetConfigurationRequest,
) -> SpiSetConfigurationResponse {
    // Guard: embassy-rp's calc_prescs panics on freq == 0 or impossibly high values
    if req.spi_frequency == 0 {
        warn!("spi_set_config: frequency must be non-zero");
        return Err(SpiError::Other);
    }

    let mut spi_config = spi::Config::default();
    spi_config.frequency = req.spi_frequency;
    spi_config.phase = match req.spi_phase {
        SpiPhase::CaptureOnFirstTransition => Phase::CaptureOnFirstTransition,
        SpiPhase::CaptureOnSecondTransition => Phase::CaptureOnSecondTransition,
    };
    spi_config.polarity = match req.spi_polarity {
        SpiPolarity::IdleLow => Polarity::IdleLow,
        SpiPolarity::IdleHigh => Polarity::IdleHigh,
    };

    debug!("spi_set_config: freq={=u32}", req.spi_frequency);
    context.spi.set_config(&spi_config);
    context.spi_frequency = req.spi_frequency;
    context.spi_phase = req.spi_phase;
    context.spi_polarity = req.spi_polarity;
    Ok(())
}

/// Handler for `i2c/get-config` — returns the current I2C bus configuration.
fn i2c_get_config_handler(context: &mut Context, _header: VarHeader, _req: ()) -> I2cGetConfigurationResponse {
    context.i2c_frequency
}

/// Handler for `spi/get-config` — returns the current SPI bus configuration.
fn spi_get_config_handler(context: &mut Context, _header: VarHeader, _req: ()) -> SpiGetConfigurationResponse {
    SpiConfigurationInfo {
        spi_frequency: context.spi_frequency,
        spi_phase: context.spi_phase,
        spi_polarity: context.spi_polarity,
    }
}

// --- UART Handlers ---

/// Handler for `uart/read` — reads bytes from the UART receive buffer.
///
/// Reads up to `count` bytes with a timeout. Returns whatever bytes are
/// available (1 to count), or an empty slice on timeout.
async fn uart_read_handler<'a>(
    context: &'a mut Context,
    _header: VarHeader,
    req: UartReadRequest,
) -> UartReadResponse<'a> {
    let count = (req.count as usize).min(MAX_TRANSFER_SIZE);
    if count == 0 {
        return Ok(&[]);
    }

    let buf = &mut context.buf[..count];

    if req.timeout_ms == 0 {
        // Non-blocking: try to read whatever is buffered
        match with_timeout(Duration::from_millis(1), AsyncRead::read(&mut context.uart, buf)).await {
            Ok(Ok(n)) => Ok(&context.buf[..n]),
            Ok(Err(_)) => Err(UartError::Other),
            Err(_) => Ok(&[]),
        }
    } else {
        match with_timeout(
            Duration::from_millis(req.timeout_ms as u64),
            AsyncRead::read(&mut context.uart, buf),
        )
        .await
        {
            Ok(Ok(n)) => Ok(&context.buf[..n]),
            Ok(Err(_)) => Err(UartError::Other),
            Err(_) => Ok(&[]),
        }
    }
}

/// Handler for `uart/write` — writes bytes to the UART transmit buffer.
async fn uart_write_handler(context: &mut Context, _header: VarHeader, req: UartWriteRequest<'_>) -> UartWriteResponse {
    if req.contents.len() > MAX_TRANSFER_SIZE {
        return Err(UartError::BufferTooLong);
    }

    AsyncWrite::write_all(&mut context.uart, req.contents)
        .await
        .map_err(|_| UartError::Other)
}

/// Handler for `uart/flush` — flushes the UART transmit buffer.
async fn uart_flush_handler(context: &mut Context, _header: VarHeader, _req: ()) -> UartFlushResponse {
    AsyncWrite::flush(&mut context.uart).await.map_err(|_| UartError::Other)
}

/// Handler for `uart/set-config` — changes the UART baud rate.
async fn uart_set_config_handler(
    context: &mut Context,
    _header: VarHeader,
    req: UartSetConfigurationRequest,
) -> UartSetConfigurationResponse {
    if req.baud_rate == 0 {
        warn!("uart_set_config: baud_rate must be non-zero");
        return Err(UartError::InvalidBaudRate);
    }

    debug!("uart_set_config: baud_rate={=u32}", req.baud_rate);
    context.uart.set_baudrate(req.baud_rate);
    context.uart_baud_rate = req.baud_rate;
    Ok(())
}

/// Handler for `uart/get-config` — returns the current UART configuration.
fn uart_get_config_handler(context: &mut Context, _header: VarHeader, _req: ()) -> UartGetConfigurationResponse {
    UartConfigurationInfo {
        baud_rate: context.uart_baud_rate,
    }
}

// ---------------------------------------------------------------------------
// PWM handlers
// ---------------------------------------------------------------------------

/// Returns the (slice_index, is_channel_b) pair for a PWM channel number.
///
/// Channel 0 → slice 0, channel A
/// Channel 1 → slice 0, channel B
/// Channel 2 → slice 1, channel A
/// Channel 3 → slice 1, channel B
fn pwm_channel_parts(channel: u8) -> Result<(usize, bool), PwmError> {
    if channel as usize >= NUM_PWM_CHANNELS {
        return Err(PwmError::InvalidChannel);
    }
    let slice_idx = usize::from(channel) / 2;
    let is_b = !channel.is_multiple_of(2);
    Ok((slice_idx, is_b))
}

/// Compute `top` and integer divider from a target frequency.
///
/// For non-phase-correct:  `f_pwm = f_sys / (divider * (top + 1))`
/// For phase-correct:      `f_pwm = f_sys / (2 * divider * top)`
fn compute_pwm_params(freq_hz: u32, phase_correct: bool) -> Result<(u16, u16), PwmError> {
    if freq_hz == 0 {
        return Err(PwmError::InvalidConfiguration);
    }

    // Try integer dividers 1..=4095 until top fits in u16.
    for div in 1u32..=4095 {
        let top_val = if phase_correct {
            // f = sys / (2 * div * top), so top = sys / (2 * div * f)
            let denom = 2u64 * u64::from(div) * u64::from(freq_hz);
            if denom == 0 {
                continue;
            }
            u64::from(SYS_CLK_HZ) / denom
        } else {
            // f = sys / (div * (top + 1)), so top = sys / (div * f) - 1
            let denom = u64::from(div) * u64::from(freq_hz);
            if denom == 0 {
                continue;
            }
            let raw = u64::from(SYS_CLK_HZ) / denom;
            if raw == 0 {
                continue;
            }
            raw - 1
        };

        if top_val <= u64::from(u16::MAX) && top_val > 0 {
            return Ok((top_val as u16, div as u16));
        }
    }

    Err(PwmError::InvalidConfiguration)
}

/// Handler for `pwm/set-duty-cycle` — sets the duty cycle of a PWM channel.
fn pwm_set_duty_cycle_handler(
    context: &mut Context,
    _header: VarHeader,
    req: PwmSetDutyCycleRequest,
) -> PwmSetDutyCycleResponse {
    let (slice_idx, is_b) = pwm_channel_parts(req.channel)?;
    let top = context.pwm_configs[slice_idx].top;

    // Clamp duty to top+1 (which means always-on)
    let compare = req.duty.min(top.saturating_add(1));

    if is_b {
        context.pwm_configs[slice_idx].compare_b = compare;
    } else {
        context.pwm_configs[slice_idx].compare_a = compare;
    }

    context.pwm_slices[slice_idx].set_config(&context.pwm_configs[slice_idx]);
    debug!(
        "pwm set duty: ch={=u8} compare={=u16} top={=u16}",
        req.channel, compare, top
    );
    Ok(())
}

/// Handler for `pwm/get-duty-cycle` — returns the current duty cycle info.
fn pwm_get_duty_cycle_handler(
    context: &mut Context,
    _header: VarHeader,
    req: PwmGetDutyCycleRequest,
) -> PwmGetDutyCycleResponse {
    let (slice_idx, is_b) = pwm_channel_parts(req.channel)?;
    let cfg = &context.pwm_configs[slice_idx];
    let compare = if is_b { cfg.compare_b } else { cfg.compare_a };
    // max_duty_cycle is top + 1 (full scale)
    let max_duty = cfg.top.saturating_add(1);

    debug!(
        "pwm get duty: ch={=u8} compare={=u16} max={=u16}",
        req.channel, compare, max_duty
    );

    Ok(PwmDutyCycleInfo {
        current_duty: compare,
        max_duty,
    })
}

/// Handler for `pwm/enable` — enables a PWM slice (identified by channel).
fn pwm_enable_handler(context: &mut Context, _header: VarHeader, req: PwmEnableRequest) -> PwmEnableResponse {
    let (slice_idx, _) = pwm_channel_parts(req.channel)?;
    context.pwm_configs[slice_idx].enable = true;
    context.pwm_slices[slice_idx].set_config(&context.pwm_configs[slice_idx]);
    debug!("pwm enable: ch={=u8} slice={=usize}", req.channel, slice_idx);
    Ok(())
}

/// Handler for `pwm/disable` — disables a PWM slice (identified by channel).
fn pwm_disable_handler(context: &mut Context, _header: VarHeader, req: PwmDisableRequest) -> PwmDisableResponse {
    let (slice_idx, _) = pwm_channel_parts(req.channel)?;
    context.pwm_configs[slice_idx].enable = false;
    context.pwm_slices[slice_idx].set_config(&context.pwm_configs[slice_idx]);
    debug!("pwm disable: ch={=u8} slice={=usize}", req.channel, slice_idx);
    Ok(())
}

/// Handler for `pwm/set-config` — configures PWM frequency and phase-correct mode.
fn pwm_set_config_handler(
    context: &mut Context,
    _header: VarHeader,
    req: PwmSetConfigurationRequest,
) -> PwmSetConfigurationResponse {
    let (slice_idx, _) = pwm_channel_parts(req.channel)?;
    let (top, div) = compute_pwm_params(req.frequency_hz, req.phase_correct)?;

    // Preserve existing compare values (scaled to new top if needed)
    let old_cfg = &context.pwm_configs[slice_idx];
    let old_top = old_cfg.top;
    let old_a = old_cfg.compare_a;
    let old_b = old_cfg.compare_b;

    // Scale compare values proportionally to the new top
    let new_a = if old_top == 0 {
        0
    } else {
        ((u32::from(old_a) * u32::from(top)) / u32::from(old_top)) as u16
    };
    let new_b = if old_top == 0 {
        0
    } else {
        ((u32::from(old_b) * u32::from(top)) / u32::from(old_top)) as u16
    };

    context.pwm_configs[slice_idx].top = top;
    context.pwm_configs[slice_idx].divider = div.to_fixed();
    context.pwm_configs[slice_idx].phase_correct = req.phase_correct;
    context.pwm_configs[slice_idx].compare_a = new_a;
    context.pwm_configs[slice_idx].compare_b = new_b;
    context.pwm_slices[slice_idx].set_config(&context.pwm_configs[slice_idx]);

    debug!(
        "pwm set config: ch={=u8} freq={=u32} top={=u16} div={=u16} pc={=bool}",
        req.channel, req.frequency_hz, top, div, req.phase_correct
    );
    Ok(())
}

/// Handler for `pwm/get-config` — returns the current PWM configuration.
fn pwm_get_config_handler(
    context: &mut Context,
    _header: VarHeader,
    req: PwmGetConfigurationRequest,
) -> PwmGetConfigurationResponse {
    let (slice_idx, _) = pwm_channel_parts(req.channel)?;
    let cfg = &context.pwm_configs[slice_idx];

    // Reconstruct frequency from top/divider/phase_correct
    let div_int: u32 = cfg.divider.to_bits() as u32 >> 4; // integer part of 12.4 fixed
    let div_val = if div_int == 0 { 1u32 } else { div_int };

    let frequency_hz = if cfg.phase_correct {
        // f = sys / (2 * div * top)
        let denom = 2u64 * u64::from(div_val) * u64::from(cfg.top);
        if denom == 0 {
            0
        } else {
            (u64::from(SYS_CLK_HZ) / denom) as u32
        }
    } else {
        // f = sys / (div * (top + 1))
        let denom = u64::from(div_val) * (u64::from(cfg.top) + 1);
        if denom == 0 {
            0
        } else {
            (u64::from(SYS_CLK_HZ) / denom) as u32
        }
    };

    debug!(
        "pwm get config: ch={=u8} freq={=u32} pc={=bool} en={=bool}",
        req.channel, frequency_hz, cfg.phase_correct, cfg.enable
    );

    Ok(PwmConfigurationInfo {
        frequency_hz,
        phase_correct: cfg.phase_correct,
        enabled: cfg.enable,
    })
}

// ---- ADC handlers ----

/// Map an [`AdcChannel`] variant to the index into `context.adc_channels`.
fn adc_channel_index(channel: AdcChannel) -> usize {
    match channel {
        AdcChannel::Adc0 => 0,
        AdcChannel::Adc1 => 1,
        AdcChannel::Adc2 => 2,
        AdcChannel::Adc3 => 3,
    }
}

/// Handler for `adc/read` — single-shot ADC read returning a raw 12-bit value.
fn adc_read_handler(context: &mut Context, _header: VarHeader, req: AdcReadRequest) -> AdcReadResponse {
    let idx = adc_channel_index(req.channel);
    let ch = &mut context.adc_channels[idx];

    match context.adc.blocking_read(ch) {
        Ok(raw) => {
            debug!("adc read: ch={=usize} raw={=u16}", idx, raw);
            Ok(raw)
        }
        Err(_) => Err(AdcError::ConversionFailed),
    }
}

/// Handler for `adc/get-config` — returns ADC configuration info.
fn adc_get_config_handler(_context: &mut Context, _header: VarHeader, _req: ()) -> AdcGetConfigurationResponse {
    debug!("adc get config");
    AdcConfigurationInfo {
        resolution_bits: ADC_RESOLUTION_BITS,
        nominal_reference_mv: ADC_NOMINAL_REFERENCE_MV,
        num_gpio_channels: NUM_ADC_GPIO_CHANNELS as u8,
    }
}

// ---- 1-Wire handlers ----

/// Handler for `onewire/reset` — performs a bus reset and returns presence detection.
async fn onewire_reset_handler(context: &mut Context, _header: VarHeader, _req: ()) -> OneWireResetResponse {
    debug!("onewire reset");
    let present = context.onewire.reset().await;
    Ok(present)
}

/// Handler for `onewire/read` — reads bytes from the 1-Wire bus.
async fn onewire_read_handler<'a>(
    context: &'a mut Context,
    _header: VarHeader,
    req: OneWireReadRequest,
) -> OneWireReadResponse<'a> {
    let len = usize::from(req.len);
    if len > MAX_TRANSFER_SIZE {
        warn!("onewire read: requested len {} exceeds buffer", len);
        return Err(OneWireError::BufferTooLong);
    }

    debug!("onewire read: len={=usize}", len);
    let buf = &mut context.buf[..len];
    context.onewire.read_bytes(buf).await;
    Ok(&context.buf[..len])
}

/// Handler for `onewire/write` — writes bytes to the 1-Wire bus.
async fn onewire_write_handler<'a>(
    context: &mut Context,
    _header: VarHeader,
    req: OneWireWriteRequest<'a>,
) -> OneWireWriteResponse {
    if req.data.len() > MAX_TRANSFER_SIZE {
        warn!("onewire write: data len {} exceeds buffer", req.data.len());
        return Err(OneWireError::BufferTooLong);
    }

    debug!("onewire write: len={=usize}", req.data.len());
    context.onewire.write_bytes(req.data).await;
    Ok(())
}

/// Handler for `onewire/write-pullup` — writes bytes then applies strong pullup.
async fn onewire_write_pullup_handler<'a>(
    context: &mut Context,
    _header: VarHeader,
    req: OneWireWritePullupRequest<'a>,
) -> OneWireWritePullupResponse {
    if req.data.len() > MAX_TRANSFER_SIZE {
        warn!("onewire write-pullup: data len {} exceeds buffer", req.data.len());
        return Err(OneWireError::BufferTooLong);
    }

    let duration = Duration::from_millis(u64::from(req.pullup_duration_ms));
    debug!(
        "onewire write-pullup: len={=usize} pullup_ms={=u16}",
        req.data.len(),
        req.pullup_duration_ms
    );
    context.onewire.write_bytes_pullup(req.data, duration).await;
    Ok(())
}

/// Handler for `onewire/search` — starts a new ROM search from scratch.
async fn onewire_search_handler(context: &mut Context, _header: VarHeader, _req: ()) -> OneWireSearchResponse {
    debug!("onewire search: starting new search");
    context.onewire_search = PioOneWireSearch::new();
    let result = context.onewire_search.next(&mut context.onewire).await;
    Ok(result)
}

/// Handler for `onewire/search-next` — continues the current ROM search.
async fn onewire_search_next_handler(context: &mut Context, _header: VarHeader, _req: ()) -> OneWireSearchResponse {
    debug!("onewire search-next");
    if context.onewire_search.is_finished() {
        return Ok(None);
    }
    let result = context.onewire_search.next(&mut context.onewire).await;
    Ok(result)
}

/// Handler for `version` — returns the firmware version.
async fn version_handler(_context: &mut Context, _header: VarHeader, _req: ()) -> VersionInfo {
    VersionInfo {
        major: VERSION_MAJOR,
        minor: VERSION_MINOR,
        patch: VERSION_PATCH,
    }
}

/// Hardware revision. Bump when the physical board layout changes.
const HW_VERSION: u8 = 1;

/// Handler for `device/info` — returns firmware version, schema version,
/// hardware version, and peripheral capabilities.
fn device_info_handler(_context: &mut Context, _header: VarHeader, _req: ()) -> DeviceInfo {
    DeviceInfo {
        fw_major: VERSION_MAJOR,
        fw_minor: VERSION_MINOR,
        fw_patch: VERSION_PATCH,
        schema_major: SCHEMA_VERSION_MAJOR,
        schema_minor: SCHEMA_VERSION_MINOR,
        schema_patch: SCHEMA_VERSION_PATCH,
        hw_version: HW_VERSION,
        capabilities: Capabilities::I2C
            | Capabilities::SPI
            | Capabilities::UART
            | Capabilities::GPIO
            | Capabilities::PWM
            | Capabilities::ADC
            | Capabilities::ONEWIRE,
    }
}
