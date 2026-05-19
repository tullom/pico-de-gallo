# PWM

Pico de Gallo provides 4 PWM output channels through the RP2350's hardware
PWM slices. Each channel maps to a specific GPIO pin and slice/channel
combination.

## Pin Mapping

| PWM Channel | GPIO | Slice | Slice Channel |
|-------------|------|-------|---------------|
| 0           | 12   | 6     | A             |
| 1           | 13   | 6     | B             |
| 2           | 14   | 7     | A             |
| 3           | 15   | 7     | B             |

The total number of available channels is defined by the constant
`NUM_PWM_CHANNELS = 4`. Channel indices are 0–3 in all APIs.

## Operations

| Operation | Description |
|-----------|-------------|
| **Set Duty** | Sets the duty cycle for a channel (0–65535) |
| **Get Duty** | Returns current and maximum duty cycle |
| **Enable** | Enables PWM output on a channel |
| **Disable** | Disables PWM output on a channel |
| **Set Config** | Configures frequency and phase-correct mode |
| **Get Config** | Returns current configuration |

## LED Brightness Example

A common use case is driving an LED at variable brightness. Connect an LED
(with appropriate current-limiting resistor) to one of the PWM pins and
control its brightness through the duty cycle.

### CLI

```bash
# 1. Configure channel 0 for 1 kHz, normal (not phase-correct) mode
gallo pwm set-config --channel 0 --frequency 1000

# 2. Enable PWM output on channel 0
gallo pwm enable --channel 0

# 3. Set duty cycle to 50% (32768 out of 65535)
gallo pwm set-duty --channel 0 --duty 32768

# 4. Read back the current duty cycle
gallo pwm get-duty --channel 0

# 5. Dim the LED to ~25%
gallo pwm set-duty --channel 0 --duty 16384

# 6. Read back the current configuration
gallo pwm get-config --channel 0

# 7. Switch to phase-correct mode at 500 Hz
gallo pwm set-config --channel 0 --frequency 500 --phase-correct

# 8. Disable the channel when done
gallo pwm disable --channel 0
```

### Rust Library

```rust,no_run
use pico_de_gallo_lib::PicoDeGallo;

async fn led_brightness(gallo: &PicoDeGallo) {
    // Configure channel 0: 1 kHz, no phase-correct
    gallo.pwm_set_config(0, 1_000, false).await.unwrap();

    // Enable PWM output
    gallo.pwm_enable(0).await.unwrap();

    // Set 50% duty cycle
    gallo.pwm_set_duty_cycle(0, 32_768).await.unwrap();

    // Read back duty info
    let duty_info = gallo.pwm_get_duty_cycle(0).await.unwrap();
    println!(
        "Duty: {}/{} ({:.1}%)",
        duty_info.current_duty,
        duty_info.max_duty,
        duty_info.current_duty as f32 / duty_info.max_duty as f32 * 100.0
    );

    // Read back configuration
    let config = gallo.pwm_get_config(0).await.unwrap();
    println!(
        "Frequency: {} Hz, Phase-correct: {}, Enabled: {}",
        config.frequency_hz, config.phase_correct, config.enabled
    );

    // Disable when done
    gallo.pwm_disable(0).await.unwrap();
}
```

### C (FFI)

```c
#include "pico_de_gallo.h"
#include <stdio.h>

void led_brightness(PicoDeGallo *gallo) {
    /* Configure channel 0: 1 kHz, no phase-correct */
    gallo_pwm_set_config(gallo, 0, 1000, false);

    /* Enable PWM output */
    gallo_pwm_enable(gallo, 0);

    /* Set 50% duty cycle */
    gallo_pwm_set_duty_cycle(gallo, 0, 32768);

    /* Read back duty info */
    GalloPwmDutyCycleInfo duty_info;
    gallo_pwm_get_duty_cycle(gallo, 0, &duty_info);
    printf("Duty: %u/%u\n", duty_info.current_duty, duty_info.max_duty);

    /* Read back configuration */
    GalloPwmConfigurationInfo config_info;
    gallo_pwm_get_config(gallo, 0, &config_info);
    printf("Frequency: %u Hz, Phase-correct: %d, Enabled: %d\n",
           config_info.frequency_hz, config_info.phase_correct,
           config_info.enabled);

    /* Disable when done */
    gallo_pwm_disable(gallo, 0);
}
```

### HAL

The HAL crate exposes individual PWM channels as `embedded_hal::pwm::SetDutyCycle`
implementors, allowing use with any driver that accepts the standard trait.

```rust,no_run
use pico_de_gallo_hal::Hal;
use embedded_hal::pwm::SetDutyCycle;

fn servo_control(hal: &Hal) {
    let mut pwm = hal.pwm_channel(0);

    // SetDutyCycle trait methods
    pwm.set_duty_cycle(32_768).unwrap();

    let max = pwm.max_duty_cycle();
    println!("Max duty: {max}");

    // 75% duty cycle
    pwm.set_duty_cycle(max * 3 / 4).unwrap();

    // Fully on / fully off
    pwm.set_duty_cycle_fully_on().unwrap();
    pwm.set_duty_cycle_fully_off().unwrap();

    // Percentage-based (if available via trait extension)
    pwm.set_duty_cycle_percent(50).unwrap();
}
```

## Lib API Reference

All library methods are `async` and return `Result` types. The
`PicoDeGallo` instance is created with `PicoDeGallo::new()` (which is
**not** async).

| Method | Return Type |
|--------|-------------|
| `pwm_set_duty_cycle(channel: u8, duty: u16)` | `Result<(), PicoDeGalloError<PwmError>>` |
| `pwm_get_duty_cycle(channel: u8)` | `Result<PwmDutyCycleInfo, PicoDeGalloError<PwmError>>` |
| `pwm_enable(channel: u8)` | `Result<(), PicoDeGalloError<PwmError>>` |
| `pwm_disable(channel: u8)` | `Result<(), PicoDeGalloError<PwmError>>` |
| `pwm_set_config(channel: u8, frequency_hz: u32, phase_correct: bool)` | `Result<(), PicoDeGalloError<PwmError>>` |
| `pwm_get_config(channel: u8)` | `Result<PwmConfigurationInfo, PicoDeGalloError<PwmError>>` |

### Response Types

**`PwmDutyCycleInfo`**

| Field | Type | Description |
|-------|------|-------------|
| `max_duty` | `u16` | Maximum duty cycle value (65535) |
| `current_duty` | `u16` | Currently configured duty cycle |

**`PwmConfigurationInfo`**

| Field | Type | Description |
|-------|------|-------------|
| `frequency_hz` | `u32` | Configured PWM frequency in Hz |
| `phase_correct` | `bool` | Whether phase-correct mode is enabled |
| `enabled` | `bool` | Whether the channel is currently enabled |

## FFI Reference

All FFI functions follow the `gallo_pwm_*` naming convention and return
a `Status` code.

```c
Status gallo_pwm_set_duty_cycle(PicoDeGallo *gallo, uint8_t channel, uint16_t duty);
Status gallo_pwm_get_duty_cycle(PicoDeGallo *gallo, uint8_t channel, GalloPwmDutyCycleInfo *out_info);
Status gallo_pwm_enable(PicoDeGallo *gallo, uint8_t channel);
Status gallo_pwm_disable(PicoDeGallo *gallo, uint8_t channel);
Status gallo_pwm_set_config(PicoDeGallo *gallo, uint8_t channel, uint32_t frequency, bool phase_correct);
Status gallo_pwm_get_config(PicoDeGallo *gallo, uint8_t channel, GalloPwmConfigurationInfo *out_info);
```

## Hardware Setup

Connect an LED (or other PWM-compatible load) to one of the PWM pins with
a current-limiting resistor:

```
GPIO 12 (PWM 0) ── 330Ω ──┬── LED ── GND
                           │
GPIO 13 (PWM 1) ── 330Ω ──┘  (or separate LEDs)
```

For servo motors, connect the signal wire directly to a PWM pin and
configure for the servo's expected frequency (typically 50 Hz). Adjust
the duty cycle to control the servo position.
