[![check](https://github.com/OpenDevicePartnership/pico-de-gallo/actions/workflows/check.yml/badge.svg)](https://github.com/OpenDevicePartnership/pico-de-gallo/actions/workflows/check.yml)
[![no-std](https://github.com/OpenDevicePartnership/pico-de-gallo/actions/workflows/nostd.yml/badge.svg)](https://github.com/OpenDevicePartnership/pico-de-gallo/actions/workflows/nostd.yml)
[![book](https://github.com/OpenDevicePartnership/pico-de-gallo/actions/workflows/gh-pages.yml/badge.svg)](https://github.com/OpenDevicePartnership/pico-de-gallo/actions/workflows/gh-pages.yml)

# Pico de Gallo

A USB bridge that turns a [Raspberry Pi Pico 2](https://www.raspberrypi.com/products/raspberry-pi-pico-2/)
into a host-accessible I²C, SPI, and GPIO interface. Write and test
embedded Rust drivers on your development machine without cross-compiling
or flashing firmware to a target board.

## Overview

Pico de Gallo provides:

- **I²C**: read, write, write-then-read, and bus scanning
- **SPI**: read, write, full-duplex transfer, and write-then-read
- **GPIO**: digital input/output and edge detection
- **Configuration**: runtime I²C/SPI frequency and SPI mode changes

All communication uses [postcard-rpc](https://docs.rs/postcard-rpc) over
USB — a compact, typed, binary RPC protocol.

## Book

The [Pico de Gallo Book](https://opendevicepartnership.github.io/pico-de-gallo/)
covers hardware assembly, firmware flashing, and a step-by-step guide to
writing an embedded device driver using Pico de Gallo.

## Crates

| Crate | Description |
|-------|-------------|
| [`pico-de-gallo-firmware`](crates/pico-de-gallo-firmware) | Embassy-rs firmware for the RP2350 |
| [`pico-de-gallo-lib`](crates/pico-de-gallo-lib) | Async host library (requires tokio) |
| [`pico-de-gallo-hal`](crates/pico-de-gallo-hal) | `embedded-hal` + `embedded-hal-async` implementation |
| [`pico-de-gallo-ffi`](crates/pico-de-gallo-ffi) | C FFI bindings (shared library + header) |
| [`gallo`](crates/pico-de-gallo-app) | CLI application for batch-mode access |
| [`pico-de-gallo-internal`](crates/pico-de-gallo-internal) | Shared wire-protocol types (internal) |

### Firmware

Embassy-rs firmware for the RP2350 that exposes I²C, SPI, and GPIO
peripherals over USB via postcard-rpc endpoints.

### Library

Async host-side library wrapping the postcard-rpc transport. Provides
typed methods for every firmware endpoint. Requires the tokio runtime.

### HAL

Implements both `embedded-hal` and `embedded-hal-async` traits — including
`I2c`, `SpiBus`, `SpiDevice`, GPIO digital I/O, and delay — so embedded device
drivers written against those traits can be tested on a host machine with real
hardware attached to the Pico de Gallo.

### App

Command-line tool (`gallo`) for interactive and batch-mode I²C/SPI/GPIO
access. Supports hex, binary, and ASCII output formats.

### FFI

C-compatible shared library wrapping the Rust host library. Generates a
C header via cbindgen. Suitable for integration into C/C++/Python/etc.
projects.

### Internal

Shared wire-protocol crate defining all postcard-rpc endpoints, request
and response types, and constants. Used by both firmware and host crates.

## Hardware

KiCAD schematic and PCB design for a Pico 2 daughter board with labeled
pin headers for I²C, SPI, and GPIO connections.

## Case

3D-printable snap-fit enclosure (FreeCAD) in two parts — body and lid.
