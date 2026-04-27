"""Read ambient temperature from a TMP108 sensor over I2C.

Wire the TMP108 to the Pico de Gallo I2C bus (SDA/SCL) and run::

    python examples/tmp108_read.py

The TMP108 returns a 16-bit big-endian word whose upper 12 bits are a
two's-complement temperature in units of 0.0625 °C/LSB. The lower 4
bits are reserved.
"""

from __future__ import annotations

import sys

import pyco_de_gallo

# TMP108 default 7-bit address when ADD0 is tied to GND. If your board
# straps ADD0 differently the address will be 0x49, 0x4A, or 0x4B.
TMP108_ADDR = 0x48

# Register pointer for the temperature register.
TMP108_TEMP_REG = 0x00

# 0.0625 °C per LSB, per the TMP108 datasheet.
TMP108_LSB_C = 0.0625


def decode_tmp108(msb: int, lsb: int) -> float:
    """Convert the two raw bytes from the temperature register to °C."""
    raw = (msb << 8) | lsb
    raw >>= 4  # drop the 4 reserved low bits → 12-bit value
    if raw & 0x800:  # sign-extend from 12 bits
        raw -= 0x1000
    return raw * TMP108_LSB_C


def main() -> int:
    devices = pyco_de_gallo.list_devices()
    if not devices:
        print("No Pico de Gallo devices found.", file=sys.stderr)
        return 1

    for d in devices:
        print(f"Found: serial={d.serial_number} product={d.product}")

    # If only one Pico de Gallo is connected, `pyco_de_gallo.open()`
    # is enough — `open_with_serial_number` is shown here for the
    # general multi-device case.
    serial = devices[0].serial_number
    gallo = pyco_de_gallo.open_with_serial_number(serial)

    # `include_reserved=False` skips the I2C reserved address ranges.
    found = gallo.i2c_scan(False)
    print("I2C devices: " + ", ".join(f"0x{a:02x}" for a in found))

    if TMP108_ADDR not in found:
        print(
            f"TMP108 not found at 0x{TMP108_ADDR:02x}. "
            "Check wiring, pull-ups, and the ADD0 strap.",
            file=sys.stderr,
        )
        return 1

    # Pointer write (register address) followed by a 2-byte read.
    data = gallo.i2c_write_read(TMP108_ADDR, [TMP108_TEMP_REG], 2)
    if len(data) != 2:
        print(f"Short read from TMP108: got {len(data)} bytes", file=sys.stderr)
        return 1

    celsius = decode_tmp108(data[0], data[1])
    print(f"Temperature: {celsius:.2f} \u00b0C")
    return 0


if __name__ == "__main__":
    sys.exit(main())
