use is31fl3743b_driver::{CSy, Is31fl3743b, SWx};
use pico_de_gallo_hal::Hal;
use std::time::Duration;

fn main() {
    let hal = Hal::new();

    // Built-in SpiDevice with firmware-managed CS on GPIO 0
    let spi_dev = hal.spi_device(0).expect("failed to create spi device");

    // Instantiate IS31FL3743B device
    let mut driver = Is31fl3743b::new(spi_dev).unwrap();

    // Enable phase delay to help reduce power noise
    let _ = driver.enable_phase_delay();
    // Set global current, check method documentation for more info
    let _ = driver.set_global_current(90);

    let _ = driver.set_led_peak_current_bulk(SWx::SW1, CSy::CS1, &[100; 11 * 18]);

    // Driver is fully set up, we can now start turning on LEDs!
    // Create a white breathing effect
    loop {
        for brightness in (0..=255_u8).chain((0..=255).rev()) {
            let _ = driver.set_led_brightness_bulk(SWx::SW1, CSy::CS1, &[brightness; 11 * 18]);
            std::thread::sleep(Duration::from_micros(1));
        }
    }
}
