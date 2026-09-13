//! W03 drives the exact browser codec from Rust; no second codec or HTTP route.
use std::process::Command;

fn node(script: &str) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let output = Command::new("node")
        .current_dir(root)
        .arg("-e")
        .arg(script)
        .output()
        .expect("Node is required for the browser codec gate");
    assert!(
        output.status.success(),
        "Node codec failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn share_link_roundtrips_play_state() {
    node(
        r#"
const assert = require('node:assert/strict');
const {encodeShare, decodeShare} = require('./ui/share.js');
const play = {k:'play',v:1,height:200,network:'testnet',boxes:[
 {boxId:'aa'.repeat(32),ergoTree:'10010101d17300',value:7000000,creationHeight:50,spent:true,tokens:[],registers:{R4:{type:'Int',value:5}}},
 {boxId:'bb'.repeat(32),ergoTree:'10010101d17300',value:7000000,creationHeight:200,spent:false}
],history:['height 200: λ → output'],synthetic:true,nodeValidated:false};
for (const state of [play,{...play,history:undefined},{s:'// λ\nsigmaProp(true)',p:{},n:'mainnet'},
 {k:'suite',v:1,suite:{tree:'10010101d17300',network:'testnet',scenarios:[{name:'synthetic pass',expect:'pass',height:1}]}}]) {
 const fragment=encodeShare(state);
 assert.match(fragment,/^[A-Za-z0-9_-]+$/);
 assert.deepEqual(decodeShare(fragment),JSON.parse(JSON.stringify(state)));
 assert.deepEqual(decodeShare('#s='+fragment),JSON.parse(JSON.stringify(state)));
}
"#,
    );
}

#[test]
fn share_link_over_cap_is_refused_with_size() {
    node(
        r#"
const assert = require('node:assert/strict');
const {encodeShare, decodeShare, FRAGMENT_CAP: cap} = require('./ui/share.js');
const state={k:'play',v:1,height:200,network:'testnet',boxes:[],history:['λ'.repeat(cap)]};
const size=3+Buffer.from(JSON.stringify(state)).toString('base64url').length;
for (const action of [()=>encodeShare(state),()=>decodeShare(Buffer.from(JSON.stringify(state)).toString('base64url'))]) {
 assert.throws(action, error=>error.message.includes(`${size} bytes`) && error.message.includes(`${cap} bytes`));
}
let boundary={s:''};
while (3+Buffer.from(JSON.stringify({s:boundary.s+'x'})).toString('base64url').length<=cap) boundary.s+='x';
assert.ok(3+encodeShare(boundary).length<=cap);
assert.deepEqual(decodeShare(encodeShare(boundary)),boundary);
boundary.s+='x'; assert.throws(()=>encodeShare(boundary),/cap/);
assert.throws(()=>decodeShare('a'.repeat(cap)),error=>error.message.includes(`${cap+3} bytes`));
"#,
    );
}
