#!/usr/bin/env python3
"""Replay HTTP export documents unchanged with this batch's compiled CLI."""
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import time
import urllib.request

REPORT = Path(__file__).resolve().parent
ROOT = REPORT.parents[2]
TARGET = Path(os.environ['CARGO_TARGET_DIR'])
CLI = TARGET / 'debug/ergo-es'
SERVER = TARGET / 'debug/ergo-web'
records = []
with socket.socket() as sock:
    sock.bind(('127.0.0.1', 0))
    port = sock.getsockname()[1]
env = {**os.environ, 'BIND_ADDR': f'127.0.0.1:{port}'}
for key in ['EXPLORER_URL', 'RATE_LIMIT_PER_MINUTE']:
    env.pop(key, None)
server = subprocess.Popen([SERVER], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT)

def post(route, value):
    req = urllib.request.Request(f'http://127.0.0.1:{port}/api/v1/{route}',
        data=json.dumps(value).encode(), headers={'Content-Type': 'application/json'})
    with urllib.request.urlopen(req, timeout=30) as response:
        assert response.status == 200
        return json.load(response)

try:
    for _ in range(100):
        try:
            with urllib.request.urlopen(f'http://127.0.0.1:{port}/api/v1/health', timeout=1):
                break
        except OSError:
            if server.poll() is not None:
                raise RuntimeError('HTTP server stopped before readiness')
            time.sleep(.05)
    else:
        raise RuntimeError('HTTP server did not become ready')
    anyone = post('compile', {'source': 'sigmaProp(true)'})['treeHex']
    refusing = post('compile', {'source': 'sigmaProp(false)'})['treeHex']
    key = 'proveDlog(decodePoint(fromBase16("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798")))'
    for name, source in [
        ('pass', 'sigmaProp(SELF.id == INPUTS(1).id && HEIGHT == 200 && getVar[Int](0).get == 5 && CONTEXT.dataInputs(0).value == 1L && OUTPUTS(0).creationInfo._1 == HEIGHT)'),
        ('fail', 'sigmaProp(HEIGHT < 100)'),
        ('error', 'sigmaProp(SELF.R4[Int].get == 5)'),
        ('needsProof', key),
        ('signed', key),
    ]:
        tree = post('compile', {'source': source})['treeHex']
        request = {'height': 200, 'network': 'testnet', 'boxes': [
            {'boxId': 'aa'*32, 'ergoTree': tree, 'value': 7},
            {'boxId': 'bb'*32, 'ergoTree': refusing, 'value': 3},
            {'boxId': 'cc'*32, 'ergoTree': anyone, 'value': 1},
        ], 'tx': {'inputs': [
            {'boxId': 'bb'*32}, {'boxId': 'aa'*32, 'contextVars': {'0': {'type': 'Int', 'value': 5}}},
        ], 'dataInputs': ['cc'*32], 'outputs': [{'ergoTree': anyone, 'value': 10}]}}
        if name == 'signed':
            request['tx']['inputs'][1]['secrets'] = [{'dlog': '0'*63+'1'}]
        played = post('play', request)
        expected = 'needsProof' if name == 'signed' else name
        assert played['inputs'][1]['verdict'] == ('proofAccepted' if name == 'signed' else name)
        for kind, filename, command in [('test','contract.test.json','test'),('scenario','scenario.json','eval')]:
            document = post('play/export', {**request,'inputIndex':1,'kind':kind})
            assert document['nodeValidated'] is False and document['synthetic'] is True
            path = REPORT / 'cli-artifacts' / name / filename
            path.parent.mkdir(parents=True, exist_ok=True)
            # Write the HTTP response unchanged; CLI consumes exactly this file.
            path.write_text(json.dumps(document, indent=2)+'\n')
            args = [str(CLI),command,str(path.relative_to(ROOT)),'--json']
            result = subprocess.run(args, cwd=ROOT, capture_output=True, text=True)
            entry = {'command':['$CARGO_TARGET_DIR/debug/ergo-es',*args[1:]],'exitCode':result.returncode}
            records.append(entry)
            print(json.dumps(entry), flush=True)
            assert result.returncode == 0, result.stderr
            output = json.loads(result.stdout)
            assert output['nodeValidated'] is False
            actual = output['cases'][0]['actual'] if kind == 'test' else output['verdict']
            assert actual == expected, f'synthetic-drift: {name}: Play/export {expected}, CLI {actual}'
            if kind == 'test':
                assert output['failed'] == 0 and output['passed'] == 1
            entry.update(originalPlayVerdict=played['inputs'][1]['verdict'], exportedVerdict=expected, cliVerdict=actual)
            print(json.dumps(entry), flush=True)
finally:
    server.send_signal(signal.SIGTERM)
    code = server.wait(timeout=10)
    records.append({'command':['$CARGO_TARGET_DIR/debug/ergo-web'],'exitCode':code,'detail':'fixture server; graceful SIGTERM after HTTP checks'})
    (REPORT/'cli-commands.json').write_text(json.dumps(records,indent=2)+'\n')
    print(f'HTTP fixture server exit: {code}', flush=True)
