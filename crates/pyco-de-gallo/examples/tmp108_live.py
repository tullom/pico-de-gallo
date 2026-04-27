"""Live TMP108 temperature graph in the terminal.

Polls the TMP108 sensor every 250 ms and renders a scrolling line
graph using Unicode braille characters in a Rich Live display. A
second line tracks an exponentially-weighted moving average over the
last ~10 seconds (40 samples at 4 Hz).

Run::

    pip install rich
    python examples/tmp108_live.py

Press ``q`` or ``Ctrl+C`` to exit. Graph width auto-fits the terminal.
"""

from __future__ import annotations

import sys
import time
from collections import deque
from typing import Deque, Sequence

import pyco_de_gallo
from rich.console import Console, Group
from rich.live import Live
from rich.panel import Panel
from rich.table import Table
from rich.text import Text

TMP108_ADDR = 0x48
TMP108_TEMP_REG = 0x00
TMP108_LSB_C = 0.0625

POLL_INTERVAL_S = 0.25
GRAPH_HEIGHT = 16  # rows of text → 16 * 4 = 64 vertical braille dots
HISTORY_HEADROOM = 4  # extra columns of history beyond what's drawn

# Cover the last ~10 s in the EWMA window. With a 250 ms poll interval
# that's 40 samples. The "span" → alpha conversion alpha = 2 / (N + 1)
# matches pandas' `ewm(span=N)` and gives the EWMA an effective
# averaging length equal to N samples.
EWMA_SPAN = int(round(10.0 / POLL_INTERVAL_S))
EWMA_ALPHA = 2.0 / (EWMA_SPAN + 1)


def decode_tmp108(msb: int, lsb: int) -> float:
    raw = (msb << 8) | lsb
    raw >>= 4
    if raw & 0x800:
        raw -= 0x1000
    return raw * TMP108_LSB_C


# Braille dot positions inside one cell:
#   (col, row) -> bit
#     col 0 col 1
#  r0   1     8
#  r1   2    16
#  r2   4    32
#  r3  64   128
_BRAILLE_BITS = (
    (0x01, 0x08),
    (0x02, 0x10),
    (0x04, 0x20),
    (0x40, 0x80),
)
_BRAILLE_BASE = 0x2800


def render_braille_lines(
    series: Sequence[Sequence[float]],
    width: int,
    height: int,
) -> list[list[str]]:
    """Render multiple data series sharing the same Y-scale.

    Returns one list-of-strings per input series so each can be
    colored independently when assembled with Rich Text.
    """
    rows = height
    cols = width
    dot_rows = rows * 4
    dot_cols = cols * 2

    flat = [v for s in series for v in s]
    if not flat:
        empty = [chr(_BRAILLE_BASE) * cols for _ in range(rows)]
        return [list(empty) for _ in series]

    lo = min(flat)
    hi = max(flat)
    span = hi - lo
    if span < 1e-6:
        span = 1.0

    def to_dot(value: float) -> int:
        norm = (value - lo) / span
        return int(round((1.0 - norm) * (dot_rows - 1)))

    out: list[list[str]] = []
    for samples in series:
        grid = [[0] * cols for _ in range(rows)]
        recent = list(samples)[-dot_cols:]
        start = dot_cols - len(recent)
        points = [(start + i, to_dot(v)) for i, v in enumerate(recent)]

        def plot(dc: int, dr: int, g=grid) -> None:
            if 0 <= dc < dot_cols and 0 <= dr < dot_rows:
                cc, sc = divmod(dc, 2)
                cr, sr = divmod(dr, 4)
                g[cr][cc] |= _BRAILLE_BITS[sr][sc]

        for (c0, r0), (c1, r1) in zip(points, points[1:]):
            steps = max(abs(c1 - c0), abs(r1 - r0), 1)
            for s in range(steps + 1):
                t = s / steps
                plot(round(c0 + (c1 - c0) * t), round(r0 + (r1 - r0) * t))
        for dc, dr in points:
            plot(dc, dr)

        out.append(["".join(chr(_BRAILLE_BASE + cell) for cell in row) for row in grid])

    return out


def render_braille_line(samples: list[float], width: int, height: int) -> list[str]:
    """Single-series convenience wrapper kept for the smoke test."""
    return render_braille_lines([samples], width, height)[0]


def build_view(
    history: Sequence[float],
    ewma_history: Sequence[float],
    width: int,
    height: int,
) -> Panel:
    if not history:
        body = Text("Waiting for samples…", style="dim")
        return Panel(body, title="TMP108", border_style="cyan")

    samples = list(history)
    ewma = list(ewma_history)
    current = samples[-1]
    ewma_now = ewma[-1]
    lo = min(samples)
    hi = max(samples)

    raw_lines, ewma_lines = render_braille_lines(
        [samples, ewma], width=width, height=height,
    )

    # Overlay the two grids in a single block. A cell may contain dots
    # from the raw series, the EWMA series, both, or neither. When both
    # series light up the same cell we draw it in yellow (overlap) so
    # the viewer can tell the two plots are coincident there. Otherwise
    # raw is green and EWMA-only is magenta.
    blank = chr(_BRAILLE_BASE)
    graph = Text()
    for raw_row, ewma_row in zip(raw_lines, ewma_lines):
        for raw_ch, ewma_ch in zip(raw_row, ewma_row):
            raw_has = raw_ch != blank
            ewma_has = ewma_ch != blank
            if raw_has and ewma_has:
                # Combine the dot bits from both cells so the overlap
                # glyph reflects every dot from either series.
                combined = chr(
                    _BRAILLE_BASE
                    | ((ord(raw_ch) - _BRAILLE_BASE) | (ord(ewma_ch) - _BRAILLE_BASE))
                )
                graph.append(combined, style="bold yellow")
            elif ewma_has:
                graph.append(ewma_ch, style="magenta")
            elif raw_has:
                graph.append(raw_ch, style="green")
            else:
                graph.append(" ")
        graph.append("\n")

    stats = Table.grid(padding=(0, 2))
    stats.add_column(style="bold")
    stats.add_column()
    stats.add_row("now",                f"[green]{current:6.2f} °C[/green]")
    stats.add_row(f"ewma({EWMA_SPAN})", f"[magenta]{ewma_now:6.2f} °C[/magenta]")
    stats.add_row("min",                f"{lo:6.2f} °C")
    stats.add_row("max",                f"{hi:6.2f} °C")
    stats.add_row("n",                  f"{len(samples)}")

    body = Group(graph, Text(""), stats)
    return Panel(
        body,
        title=(
            f"TMP108 @ 0x{TMP108_ADDR:02x} — "
            f"{1.0 / POLL_INTERVAL_S:.1f} Hz   "
            "[green]raw[/green]   [magenta]ewma[/magenta]   "
            "[bold yellow]overlap[/bold yellow]"
        ),
        subtitle="press q or Ctrl+C to exit",
        border_style="cyan",
    )


def open_device(console: Console):
    devices = pyco_de_gallo.list_devices()
    if not devices:
        console.print("[red]No Pico de Gallo devices found.[/red]")
        sys.exit(1)
    serial = devices[0].serial_number
    # `pyco_de_gallo.open()` is enough when only one board is present.
    gallo = pyco_de_gallo.open_with_serial_number(serial)

    found = gallo.i2c_scan(False)
    if TMP108_ADDR not in found:
        console.print(
            f"[red]TMP108 not found at 0x{TMP108_ADDR:02x}.[/red] "
            "Check wiring, pull-ups, and the ADD0 strap."
        )
        sys.exit(1)
    return gallo


def read_temperature(gallo) -> float:
    data = gallo.i2c_write_read(TMP108_ADDR, [TMP108_TEMP_REG], 2)
    if len(data) != 2:
        raise RuntimeError(f"short read from TMP108: {len(data)} bytes")
    return decode_tmp108(data[0], data[1])


class _QuitKeyPoller:
    """Non-blocking 'q' key detector. Cross-platform context manager.

    On POSIX systems we put stdin into cbreak mode so individual
    keypresses are visible without waiting for Enter. On Windows we
    use msvcrt's character-at-a-time API directly. The context
    manager is a no-op when stdin isn't a TTY (e.g. piped input).
    """

    def __init__(self) -> None:
        self._enabled = sys.stdin.isatty()
        self._is_windows = sys.platform == "win32"
        self._old_termios = None
        if self._enabled and not self._is_windows:
            try:
                import termios  # noqa: F401  (probe only)
            except ImportError:
                self._enabled = False

    def __enter__(self) -> "_QuitKeyPoller":
        if self._enabled and not self._is_windows:
            import termios
            import tty

            fd = sys.stdin.fileno()
            self._old_termios = termios.tcgetattr(fd)
            tty.setcbreak(fd)
        return self

    def __exit__(self, *exc) -> None:
        if self._old_termios is not None:
            import termios

            termios.tcsetattr(sys.stdin.fileno(), termios.TCSADRAIN, self._old_termios)

    def quit_requested(self) -> bool:
        if not self._enabled:
            return False
        if self._is_windows:
            import msvcrt

            while msvcrt.kbhit():
                ch = msvcrt.getwch()
                if ch.lower() == "q":
                    return True
        else:
            import select

            while select.select([sys.stdin], [], [], 0)[0]:
                ch = sys.stdin.read(1)
                if not ch:
                    break
                if ch.lower() == "q":
                    return True
        return False


def main() -> int:
    console = Console()
    gallo = open_device(console)

    history: Deque[float] = deque()
    ewma_history: Deque[float] = deque()
    ewma: float | None = None

    with _QuitKeyPoller() as keys, Live(
        console=console, refresh_per_second=8, screen=False
    ) as live:
        try:
            while True:
                if keys.quit_requested():
                    break

                width = max(20, console.size.width - 6)
                cap = width * 2 + HISTORY_HEADROOM
                if history.maxlen != cap:
                    history = deque(history, maxlen=cap)
                    ewma_history = deque(ewma_history, maxlen=cap)

                try:
                    temp = read_temperature(gallo)
                    history.append(temp)
                    ewma = temp if ewma is None else ewma + EWMA_ALPHA * (temp - ewma)
                    ewma_history.append(ewma)
                except Exception as e:  # noqa: BLE001 - surface any I/O hiccup
                    live.update(Panel(
                        Text(f"Read error: {e}", style="red"),
                        title="TMP108",
                        border_style="red",
                    ))
                    time.sleep(POLL_INTERVAL_S)
                    continue

                live.update(build_view(
                    history, ewma_history, width=width, height=GRAPH_HEIGHT,
                ))

                # Sleep in small slices so 'q' is responsive between samples.
                deadline = time.monotonic() + POLL_INTERVAL_S
                while time.monotonic() < deadline:
                    if keys.quit_requested():
                        return 0
                    time.sleep(0.05)
        except KeyboardInterrupt:
            pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
