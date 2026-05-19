# Architecture

At a high level, Pico de Gallo is a typed RPC bridge between a host computer and
an RP2350 microcontroller. The host talks USB; the firmware talks peripherals.
The wire contract in the middle is shared Rust code.

```text
host app / script / test
        |
        +--> pico-de-gallo-lib  (async Rust client)
        +--> gallo             (CLI)
        +--> pico-de-gallo-ffi (C ABI)
        +--> pyco-de-gallo     (Python bindings)
                     |
                     v
                   nusb
                     |
                 USB cable
                     |
                     v
        pico-de-gallo-firmware (Embassy on RP2350)
                     |
      +--------------+--------------+--------------+
      |              |              |              |
     I2C            SPI            UART        GPIO/PWM/ADC/1-Wire
      |              |              |              |
      +--------------------- external hardware --------------------+
```

Three design choices make that stack work.

## Why postcard-rpc?

Pico de Gallo uses [postcard-rpc](https://docs.rs/postcard-rpc) because it keeps
both sides honest:

- the request and response types live in one shared crate,
- endpoints are schema-typed instead of stringly typed APIs,
- the protocol is transport-agnostic even though Pico de Gallo currently uses
  USB.

That shared crate is `pico-de-gallo-internal`. Both the firmware and every host
surface depend on it, so the same Rust types define the wire format everywhere.

> [!TIP]
> This is why new protocol features tend to start in `pico-de-gallo-internal`:
> once the type exists there, the rest of the stack can wire it through.

## Why Embassy?

The firmware is built on [Embassy](https://embassy.dev), which gives Pico de
Gallo an async executor on bare metal. That matters because USB traffic,
interrupt-driven peripherals, DMA-backed transfers, and GPIO event monitoring
all need to coexist without a giant polling loop.

Embassy also maps well to the hardware roles here:

- async I<sup>2</sup>C and SPI transfers,
- interrupt-driven UART,
- low-jitter timing primitives,
- background tasks for server dispatch and GPIO event publishing.

## Why a host-side `embedded-hal` shim?

`pico-de-gallo-hal` adapts the RPC client into familiar `embedded-hal` and
`embedded-io` traits. The goal is simple: write a driver once, then exercise it
from your laptop before you ever flash target firmware.

That is the main idea behind Pico de Gallo as a project: move driver iteration
from a flash-debug cycle to a normal host test loop.

## The trust boundary

The host trusts the firmware to validate requests before touching hardware.
That is an intentional boundary, and the firmware enforces it: handlers reject
out-of-range pins, oversized buffers, unsupported peripherals on `hw-rev1`, and
invalid configuration values before they access the RP2350 peripherals.

> [!IMPORTANT]
> Validation lives at the firmware edge because the firmware is the part that
> knows the real hardware limits. Host code should still be well-behaved, but
> the last line of defense is on-device.

For contributor-only detail, see
[`AGENTS.md`](https://github.com/OpenDevicePartnership/pico-de-gallo/blob/main/AGENTS.md)
and [`CONTRIBUTING.md`](https://github.com/OpenDevicePartnership/pico-de-gallo/blob/main/CONTRIBUTING.md).
