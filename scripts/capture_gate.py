#!/usr/bin/env python3
"""Capture a publishable gate transcript, preserving the command exit code."""
import argparse
from pathlib import Path
import sys

from roadmap_gate import command


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    executable = args.command[1:] if args.command[:1] == ['--'] else args.command
    if not executable:
        parser.error('a command is required')
    result = command(executable, [])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(result.stdout)
    return result.returncode


if __name__ == '__main__':
    sys.exit(main())
