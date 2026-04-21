#![no_std]
#![no_main]
//! Pico de Gallo firmware for the Raspberry Pi Pico 2 (RP2350).
//!
//! This firmware implements a USB bridge that exposes I2C, SPI, UART, and GPIO
//! peripherals to a host computer via [postcard-rpc](https://docs.rs/postcard-rpc)
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
//! | GPIO 0–7 | GPIO 8–15 | Input/output/edge-wait |
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

use defmt::{debug, info, warn};
use embassy_embedded_hal::SetConfig;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::ClockConfig;
use embassy_rp::gpio::{Flex, Level, Pull};
use embassy_time::{Duration, with_timeout};

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
use embassy_rp::i2c::{self, I2c};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, I2C1, SPI0, UART0, USB};
use embassy_rp::spi::{self, Phase, Polarity, Spi};
use embassy_rp::uart::{self, BufferedUart};
use embassy_rp::usb::Driver;
use embedded_io_async::{Read as AsyncRead, Write as AsyncWrite};
// Direct embassy-sync dep required: postcard-rpc's WireStorage is generic over
// embassy_sync_0_7::blocking_mutex::raw::RawMutex, which is the same type as
// embassy-sync 0.7's RawMutex (they share the same crate version).
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_usb::{Config, UsbDevice};
use pico_de_gallo_internal::{
    ENDPOINT_LIST, GpioDirection, GpioError, GpioGet, GpioGetRequest, GpioGetResponse, GpioPull, GpioPut,
    GpioPutRequest, GpioPutResponse, GpioSetConfiguration, GpioSetConfigurationRequest, GpioSetConfigurationResponse,
    GpioState, GpioWaitForAny, GpioWaitForFalling, GpioWaitForHigh, GpioWaitForLow, GpioWaitForRising, GpioWaitRequest,
    GpioWaitResponse, I2cError, I2cFrequency, I2cGetConfiguration, I2cGetConfigurationResponse, I2cRead,
    I2cReadRequest, I2cReadResponse, I2cScan, I2cScanRequest, I2cScanResponse, I2cSetConfiguration,
    I2cSetConfigurationRequest, I2cSetConfigurationResponse, I2cWrite, I2cWriteRead, I2cWriteReadRequest,
    I2cWriteReadResponse, I2cWriteRequest, I2cWriteResponse, MAX_TRANSFER_SIZE, MICROSOFT_VID, PICO_DE_GALLO_PID,
    PingEndpoint, SpiConfigurationInfo, SpiError, SpiFlush, SpiFlushResponse, SpiGetConfiguration,
    SpiGetConfigurationResponse, SpiPhase, SpiPolarity, SpiRead, SpiReadRequest, SpiReadResponse, SpiSetConfiguration,
    SpiSetConfigurationRequest, SpiSetConfigurationResponse, SpiTransfer, SpiTransferRequest, SpiTransferResponse,
    SpiWrite, SpiWriteRequest, SpiWriteResponse, TOPICS_IN_LIST, TOPICS_OUT_LIST, UartConfigurationInfo, UartError,
    UartFlush, UartFlushResponse, UartGetConfiguration, UartGetConfigurationResponse, UartRead, UartReadRequest,
    UartReadResponse, UartSetConfiguration, UartSetConfigurationRequest, UartSetConfigurationResponse, UartWrite,
    UartWriteRequest, UartWriteResponse, Version, VersionInfo,
};
use postcard_rpc::{
    define_dispatch,
    header::VarHeader,
    server::{
        Dispatch, Server,
        impls::embassy_usb_v0_5::{
            PacketBuffers,
            dispatch_impl::{WireRxBuf, WireRxImpl, WireSpawnImpl, WireStorage, WireTxImpl},
        },
    },
};
use static_cell::ConstStaticCell;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

/// Binary info entries for `picotool info` identification.
///
/// These entries are placed in the `.bi_entries` linker section and allow
/// `picotool` to display the firmware name, description, version, and
/// build info without running the firmware.
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"Pico de Gallo"),
    embassy_rp::binary_info::rp_program_description!(c"USB bridge to various buses such as I2C, SPI, and UART"),
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
});

const NUM_GPIOS: usize = 8;

/// Firmware application context holding all peripheral handles.
///
/// NOTE: `buf` is shared between I2C and SPI handlers. This is safe because
/// postcard-rpc dispatches handlers serially (one at a time). If concurrent
/// dispatch is ever enabled, separate buffers would be required.
pub struct Context {
    i2c: I2c<'static, I2C1, i2c::Async>,
    spi: Spi<'static, SPI0, spi::Async>,
    uart: BufferedUart,
    gpios: [Flex<'static>; NUM_GPIOS],
    pin_modes: [PinMode; NUM_GPIOS],
    i2c_frequency: I2cFrequency,
    spi_frequency: u32,
    spi_phase: SpiPhase,
    spi_polarity: SpiPolarity,
    uart_baud_rate: u32,
    buf: [u8; MAX_TRANSFER_SIZE],
}

impl Context {
    fn new(
        i2c: I2c<'static, I2C1, i2c::Async>,
        spi: Spi<'static, SPI0, spi::Async>,
        uart: BufferedUart,
        gpios: [Flex<'static>; NUM_GPIOS],
    ) -> Self {
        // Defaults match embassy-rp Config::default()
        Self {
            i2c,
            spi,
            uart,
            gpios,
            pin_modes: [PinMode::LegacyAuto; NUM_GPIOS],
            i2c_frequency: I2cFrequency::Standard,
            spi_frequency: 1_000_000,
            spi_phase: SpiPhase::CaptureOnFirstTransition,
            spi_polarity: SpiPolarity::IdleLow,
            uart_baud_rate: 115_200,
            buf: [0; MAX_TRANSFER_SIZE],
        }
    }
}

/// Helper macro to get a GPIO pin by index for input operations.
/// In `LegacyAuto` mode, auto-switches to input. In `ExplicitInput` mode,
/// uses the pin as-is. In `ExplicitOutput` mode, returns `WrongDirection`.
macro_rules! gpio_for_input {
    ($context:expr, $pin:expr) => {{
        let idx = usize::from($pin);
        let mode = *$context.pin_modes.get(idx).ok_or(GpioError::InvalidPin)?;
        let gpio = $context.gpios.get_mut(idx).ok_or(GpioError::InvalidPin)?;
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

        | EndpointTy          | kind     | handler                       |
        | ----------          | ----     | -------                       |
        | PingEndpoint        | blocking | ping_handler                  |
        | I2cRead             | async    | i2c_read_handler              |
        | I2cWrite            | async    | i2c_write_handler             |
        | I2cWriteRead        | async    | i2c_write_read_handler        |
        | SpiRead             | async    | spi_read_handler              |
        | SpiWrite            | async    | spi_write_handler             |
        | SpiFlush            | async    | spi_flush_handler             |
        | SpiTransfer         | async    | spi_transfer_handler          |
        | GpioGet             | async    | gpio_get_handler              |
        | GpioPut             | async    | gpio_put_handler              |
        | GpioWaitForHigh     | async    | gpio_wait_for_high_handler    |
        | GpioWaitForLow      | async    | gpio_wait_for_low_handler     |
        | GpioWaitForRising   | async    | gpio_wait_for_rising_handler  |
        | GpioWaitForFalling  | async    | gpio_wait_for_falling_handler |
        | GpioWaitForAny      | async    | gpio_wait_for_any_handler     |
        | GpioSetConfiguration | async    | gpio_set_config_handler       |
        | I2cSetConfiguration  | async    | i2c_set_config_handler        |
        | I2cScan             | async    | i2c_scan_handler              |
        | SpiSetConfiguration  | async    | spi_set_config_handler        |
        | I2cGetConfiguration  | blocking | i2c_get_config_handler        |
        | SpiGetConfiguration  | blocking | spi_get_config_handler        |
        | UartRead             | async    | uart_read_handler             |
        | UartWrite            | async    | uart_write_handler            |
        | UartFlush            | async    | uart_flush_handler            |
        | UartSetConfiguration | async    | uart_set_config_handler       |
        | UartGetConfiguration | blocking | uart_get_config_handler       |
        | Version             | async    | version_handler               |
    };
    topics_in: {
        list: TOPICS_IN_LIST;

        | TopicTy                   | kind      | handler                       |
        | ----------                | ----      | -------                       |
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
        Flex::new(p.PIN_12),
        Flex::new(p.PIN_13),
        Flex::new(p.PIN_14),
        Flex::new(p.PIN_15),
    ];

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

    let context = Context::new(i2c, spi, uart, gpios);

    let (device, tx_impl, rx_impl) = STORAGE.init(
        driver,
        config,
        pbufs.tx_buf.as_mut_slice(),
        postcard_rpc::server::impls::embassy_usb_v0_5::USB_FS_MAX_PACKET_SIZE,
    );
    let dispatcher = PicoDeGallo::new(context, spawner.into());
    let vkk = dispatcher.min_key_len();
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
    let gpio = context.gpios.get_mut(idx).ok_or(GpioError::InvalidPin)?;

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
    let gpio = context.gpios.get_mut(idx).ok_or(GpioError::InvalidPin)?;

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

/// Handler for `version` — returns the firmware version.
async fn version_handler(_context: &mut Context, _header: VarHeader, _req: ()) -> VersionInfo {
    VersionInfo {
        major: VERSION_MAJOR,
        minor: VERSION_MINOR,
        patch: VERSION_PATCH,
    }
}
