# The Firmware

The Pico de Gallo firmware lives in its own Cargo workspace at
`crates/pico-de-gallo-firmware/`. That separation is intentional: it targets
`thumbv8m.main-none-eabihf`, is `no_std`, and carries its own committed
`Cargo.lock`.

## Runtime model

The firmware runs on the RP2350 using Embassy:

- `embassy-executor` for async task scheduling,
- `embassy-rp` for RP2350 peripherals,
- `embassy-usb` for the USB device stack.

`postcard-rpc` sits on top of that USB transport and dispatches endpoint
handlers into async peripheral code. Requests are serialized on a shared
context, while background tasks handle work such as GPIO event publication.

> [!TIP]
> This is why the firmware can do DMA-backed transfers and interrupt-driven I/O
> without turning into a hand-written state machine maze.

## `no_std` and logging

This crate is `no_std`. Logging uses `defmt` over RTT.

> [!IMPORTANT]
> There is no `println!` fallback in firmware. If you need diagnostics, use
> `defmt`.

## Hardware revisions

Two feature flags select the board revision:

| Feature | Default | Board | Capabilities |
|---------|---------|-------|--------------|
| `hw-rev1` | yes | v1.0 | I<sup>2</sup>C, SPI, GPIO, PWM |
| `hw-rev2` | no | v1.1+ | I<sup>2</sup>C, SPI, UART, GPIO, PWM, ADC, 1-Wire |

On `hw-rev1`, unsupported peripherals return `Unsupported` instead of touching
unrouted hardware.

Build the two variants exactly as CI does:

```bash
cd crates/pico-de-gallo-firmware

cargo fmt --check
cargo clippy --target thumbv8m.main-none-eabihf -- -D warnings
cargo build --release --locked --target thumbv8m.main-none-eabihf

cargo clippy --target thumbv8m.main-none-eabihf \
    --no-default-features --features hw-rev2 -- -D warnings
cargo build --release --locked --target thumbv8m.main-none-eabihf \
    --no-default-features --features hw-rev2
```

## Peripheral notes

The RP2350 pin map matches the hardware docs in
[Pinout & Connector](../hardware/pinout.md):

- I<sup>2</sup>C uses I2C1 on GPIO 2/3 and runs asynchronously with Embassy.
- SPI uses SPI0 on GPIO 4/6/7 and supports DMA-backed full-duplex transfers.
- UART uses UART0 on GPIO 0/1 with buffered, interrupt-driven I/O.
- GPIO user pins are GPIO 8-11, with wait and subscribe support.
- PWM outputs are GPIO 12-15 on slices 6 and 7.
- 1-Wire uses PIO0 state machine 0 on GPIO 16.
- ADC reads are single-shot samples on GPIO 26-29 in firmware, with board
  routing exposing ADC0-2 on current hardware.

The shared transfer buffer is 4096 bytes (`MAX_TRANSFER_SIZE`), and handlers
validate lengths before indexing into it.

## Dependency pins that matter

The firmware intentionally pins `embassy-usb-driver = "=0.2.0"`.
That exact version is documented in
[`AGENTS.md`](https://github.com/OpenDevicePartnership/pico-de-gallo/blob/main/AGENTS.md)
because `0.2.1` pulled in an incompatible `embedded-io-async` update for the
current `embassy-usb 0.5` stack.

That documentation is part of the contributor contract: exact pins are not
supposed to look mysterious.

## Flashing

Flashing is the normal Pico UF2 flow:

1. Hold `BOOTSEL` while connecting USB.
2. Wait for the `RP2350` mass-storage device to appear.
3. Drag and drop the firmware `.uf2`.
4. The board auto-resets and reconnects with the new firmware.

After flashing, `gallo version` is the quickest sanity check because it shows
firmware version, schema version, hardware revision, and capabilities.
