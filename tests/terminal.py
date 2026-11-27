#!/usr/bin/env python3
"""Optional developer PTY checks; Python is not a dutop runtime dependency."""
import errno
import fcntl
import os
from pathlib import Path
import pty
import re
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time
import unittest

BINARY = str(Path(sys.argv.pop(1)).resolve())
ANSI = re.compile(rb"\x1b\[[0-9;]*[A-Za-z]")


def terminal(directory, width=120, extra_env=None, cancel=False):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, width, 0, 0))
    env = dict(os.environ, TERM="xterm-256color")
    env.pop("NO_COLOR", None)
    env.update(extra_env or {})
    child = subprocess.Popen([BINARY, "--top", "4", str(directory)],
                             stdin=subprocess.DEVNULL, stdout=slave, stderr=slave, env=env)
    os.close(slave)
    output = bytearray()
    interrupted = False
    deadline = time.monotonic() + 15
    try:
        while time.monotonic() < deadline:
            ready, _, _ = select.select([master], [], [], 0.1)
            if ready:
                try:
                    data = os.read(master, 65536)
                except OSError as error:
                    if error.errno == errno.EIO:
                        break
                    raise
                if not data:
                    break
                output.extend(data)
                if cancel and not interrupted and b"Ctrl+C cancels" in output:
                    child.send_signal(signal.SIGINT)
                    interrupted = True
            elif child.poll() is not None:
                break
        code = child.wait(timeout=max(0.1, deadline-time.monotonic()))
        return code, bytes(output), interrupted
    finally:
        if child.poll() is None:
            child.kill()
            child.wait()
        os.close(master)


class TerminalTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / "folder").mkdir()
        (self.root / "folder" / "inside").write_bytes(b"x" * 8192)
        (self.root / "file").write_bytes(b"x" * 4096)

    def tearDown(self):
        self.temp.cleanup()

    def test_tty_has_colors_and_no_alternate_screen(self):
        code, output, _ = terminal(self.root)
        self.assertEqual(code, 0)
        self.assertRegex(output, rb"\x1b\[[0-9;]+m")
        self.assertIn(b"FILES 1 / 1", output)
        self.assertNotIn(b"?1049", output)

    def test_no_color_disables_colored_report(self):
        code, output, _ = terminal(self.root, extra_env={"NO_COLOR": "1"})
        self.assertEqual(code, 0)
        self.assertNotRegex(output, rb"\x1b\[[0-9;]+m")

    def test_dumb_terminal_has_no_escape_sequences(self):
        code, output, _ = terminal(self.root, extra_env={"TERM": "dumb"})
        self.assertEqual(code, 0)
        self.assertNotIn(b"\x1b", output)

    def test_narrow_terminal_stacks_panels(self):
        code, output, _ = terminal(self.root, width=48, extra_env={"NO_COLOR": "1"})
        self.assertEqual(code, 0)
        lines = ANSI.sub(b"", output).decode().splitlines()
        report = lines[next(i for i, line in enumerate(lines) if line.startswith("dutop ")):]
        self.assertTrue(all(len(line) <= 48 for line in report))
        self.assertFalse(any("FOLDERS" in line and "FILES" in line for line in report))

    def test_ctrl_c_cancels_scan_with_status_130(self):
        for i in range(8000):
            (self.root / f"directory-{i}").mkdir()
        code, output, interrupted = terminal(self.root, cancel=True)
        self.assertTrue(interrupted)
        self.assertEqual(code, 130)
        self.assertIn(b"Scan cancelled.", output)
        self.assertNotIn(b"TOTAL", output)

    def test_sigint_interrupts_a_blocked_output_pipe(self):
        child = subprocess.Popen([BINARY, str(self.root)], stdout=subprocess.PIPE,
                                 stderr=subprocess.PIPE,
                                 env=dict(os.environ, COLUMNS="65000", NO_COLOR="1"))
        try:
            self.assertTrue(child.stdout.read(1))
            child.send_signal(signal.SIGINT)
            try:
                code = child.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.fail("SIGINT left dutop blocked writing to its output pipe")
            self.assertEqual(code, 130)
        finally:
            if child.poll() is None:
                child.kill()
            child.communicate(timeout=3)


if __name__ == "__main__":
    unittest.main()
