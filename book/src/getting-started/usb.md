# USB & OS Notes

The Pico de Gallo firmware uses a generic WinUSB-compatible
descriptor, so most operating systems pick it up without a custom
driver. The notes below cover the cases where you need to nudge
the OS.

## Linux

Out-of-the-box, libusb (and therefore `nusb`) requires root to
open arbitrary USB devices. To let your regular user account talk
to Pico de Gallo, drop a udev rule:

```text
# /etc/udev/rules.d/99-pico-de-gallo.rules
SUBSYSTEM=="usb", ATTR{idVendor}=="045e", ATTR{idProduct}=="ffff", MODE="0666"
```

Then reload udev:

```console
$ sudo udevadm control --reload-rules
$ sudo udevadm trigger
```

Unplug and replug the device. `gallo version` should now work
without `sudo`.

> [!NOTE]
>
> The VID `045e` (Microsoft) and PID `ffff` are placeholders
> used by the firmware — Microsoft's vendor block reserves `ffff`
> for prototyping. They are not registered for Pico de Gallo and
> should not be considered stable across firmware versions.

## Windows

The firmware advertises a Microsoft OS 2.0 descriptor that tells
Windows to bind the WinUSB driver automatically. The first time
you plug in a Pico de Gallo, you may see a brief "installing
device" notification — that's normal. After that, `gallo` works
without any extra setup.

If for some reason WinUSB doesn't bind (e.g., a stale Zadig
override, or driver-signing policy on a corporate machine), use
[Zadig](https://zadig.akeo.ie/) to manually install the WinUSB
driver against the Pico de Gallo interface.

## macOS

No extra setup. macOS picks the device up automatically.

If `gallo list` returns nothing, check System Information →
USB and confirm the device enumerates. If it shows up there but
`gallo` can't find it, you might have a code-signing issue with a
locally-built `gallo` binary — try the pre-built release artifact.

## Troubleshooting

- **`gallo: device not found`** — Is the device plugged in? Did
  you flash firmware? Try `gallo list`.
- **`Permission denied` on Linux** — udev rule missing or not
  reloaded. See above.
- **`gallo version` succeeds but `gallo i2c scan` hangs** — the
  bus has no pull-ups, or your peripheral is clock-stretching
  forever. Add 4.7 kΩ pull-ups (v1.0 boards lack them on-board).
- **Device disappears after a write** — likely a brown-out from
  trying to source too much current through the on-board 3.3 V
  rail. Power the peripheral externally.

See also: [Troubleshooting](../appendix/troubleshooting.md) for
the full list.
