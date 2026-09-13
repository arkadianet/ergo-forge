#!/usr/bin/env python3
"""Verify project locks against independent sibling source/params/suite inputs."""
import glob
import json
from pathlib import Path
import shlex
import subprocess
import sys


def supports_verify_lock():
    """The installed ergo-es must know `verify-lock`; releases before it do not."""
    try:
        probe = subprocess.run(['ergo-es', '--help'], check=False, capture_output=True, text=True)
    except OSError as error:
        print(f'ergo-es is not runnable: {error}', file=sys.stderr)
        return False
    if 'verify-lock' in (probe.stdout or '') + (probe.stderr or ''):
        return True
    print('The installed ergo-es predates `verify-lock`, so `lockfile` cannot be verified. '
          'Use `version: source`, or a release whose ergo-es lists verify-lock in `ergo-es --help`.',
          file=sys.stderr)
    return False


def verify(patterns):
    if not supports_verify_lock():
        return 1
    paths = set()
    failed = False
    for pattern in shlex.split(patterns):
        matches = glob.glob(pattern, recursive=True)
        if not matches:
            print(f'No lockfiles matched: {pattern}', file=sys.stderr)
            failed = True
        paths.update(matches)
    if not paths:
        return 1
    for name in sorted(paths):
        path = Path(name)
        try:
            if not path.name.endswith('.lock.json'):
                raise ValueError('lockfile name must end with .lock.json')
            stem = path.name.removesuffix('.lock.json')
            source = path.with_name(stem + '.es')
            if not source.is_file():
                raise ValueError(f'missing source: {source}')
            params = path.with_name('params.json')
            suite = path.with_name(stem + '.test.json')
            # Network and compiler version are independent build inputs. Never
            # derive the expected address or version from a drifted lock field.
            config = json.loads(suite.read_text()) if suite.is_file() else {}
            network = config.get('network', 'mainnet')
            version = config.get('treeVersion', 3)
            if network not in ('mainnet', 'testnet') or type(version) is not int or not 0 <= version <= 255:
                raise ValueError('invalid suite network/treeVersion')
            command = ['ergo-es', 'verify-lock', str(path), '--source', str(source), '--network', network, '--tree-version', str(version)]
            if params.is_file():
                command += ['--params', str(params)]
            print(shlex.join(command), flush=True)
            if subprocess.run(command, check=False).returncode != 0:
                failed = True
        except (OSError, ValueError, TypeError, AttributeError) as error:
            print(f'{path}: {error}', file=sys.stderr)
            failed = True
    return int(failed)


if __name__ == '__main__':
    sys.exit(verify(sys.argv[1]))
