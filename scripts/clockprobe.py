#!/usr/bin/env python3
"""Sleep detector for a benchmark cell.

macOS suspends the monotonic clock while the machine sleeps and keeps the wall clock
running. `time.monotonic()` on Darwin is `mach_absolute_time()` (verified via
`time.get_clock_info`), which does not advance during sleep; `time.time()` does. So for any
interval:

    wall_elapsed - monotonic_elapsed  ~=  seconds spent asleep

A cell whose interval spanned a sleep has a wall-clock duration that includes time the CPU
was not running. Its `real` seconds, its `(user+sys)/real` ratio and every rate derived from
them are garbage that looks like data. Such a cell is marked INVALID_SLEEP and rerun.

Usage:
    clockprobe.py mark              -> prints "<monotonic> <wall>" for one instant
    clockprobe.py diff M0 W0 M1 W1  -> prints "<mono_s> <wall_s> <slept_s> <verdict>"

The threshold is deliberately loose (2 s) so that ordinary clock jitter and NTP steps do not
flag a healthy cell; a real idle sleep on this machine lasts minutes.
"""

import sys
import time

SLEEP_THRESHOLD_S = 2.0


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__, file=sys.stderr)
        return 2

    command = sys.argv[1]

    if command == "mark":
        print(f"{time.monotonic():.6f} {time.time():.6f}")
        return 0

    if command == "diff":
        if len(sys.argv) != 6:
            print("diff needs: M0 W0 M1 W1", file=sys.stderr)
            return 2
        m0, w0, m1, w1 = (float(x) for x in sys.argv[2:6])
        mono = m1 - m0
        wall = w1 - w0
        slept = wall - mono
        verdict = "INVALID_SLEEP" if slept > SLEEP_THRESHOLD_S else "OK"
        print(f"{mono:.3f} {wall:.3f} {slept:.3f} {verdict}")
        return 0

    print(f"unknown command: {command}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
