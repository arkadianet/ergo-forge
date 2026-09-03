//! Spending proofs from secrets, through the node's own wallet prover
//! (`ergo-wallet::proving::sigma::prove_sigma`: Schnorr for `proveDlog`,
//! Diffie-Hellman tuples, AND / OR / k-of-n with simulated branches and
//! Fiat-Shamir). A scenario that names secrets gets a real proof for the
//! proposition its script reduced to, and that proof is then checked
//! through the consensus verification path like any supplied proof.

use ergo_ser::sigma_value::SigmaBoolean;
use ergo_wallet::proving::external::ProverExternalSecret;
use ergo_wallet::proving::hints::HintsBag;
use ergo_wallet::proving::randomness::OsRngBackend;
use ergo_wallet::proving::secrets::SecretRegistry;
use ergo_wallet::proving::sigma::prove_sigma;
use k256::elliptic_curve::sec1::{FromEncodedPoint, ToEncodedPoint};
use k256::elliptic_curve::PrimeField;
use k256::{AffinePoint, EncodedPoint, FieldBytes, ProjectivePoint, Scalar};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::SandboxError;

/// A secret the scenario's spender holds.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SecretSpec {
    /// `x` (32-byte hex) for `proveDlog(g^x)`.
    Dlog(String),
    /// `x` for `proveDHTuple(g, h, g^x, h^x)`; `u` and `v` are derived.
    Dht { g: String, h: String, x: String },
}

fn err(msg: impl Into<String>) -> SandboxError {
    SandboxError::Scenario(msg.into())
}

fn scalar(hex_str: &str) -> Result<Scalar, SandboxError> {
    let bytes = hex::decode(hex_str.trim()).map_err(|e| err(format!("secret hex: {e}")))?;
    if bytes.len() != 32 {
        return Err(err(format!("a secret is 32 bytes, got {}", bytes.len())));
    }
    let arr: [u8; 32] = bytes.try_into().unwrap();
    let s: Option<Scalar> = Scalar::from_repr(FieldBytes::from(arr)).into();
    match s {
        Some(s) if s != Scalar::ZERO => Ok(s),
        _ => Err(err(
            "a secret must be a nonzero scalar below the group order",
        )),
    }
}

fn point(hex_str: &str) -> Result<ProjectivePoint, SandboxError> {
    let bytes = hex::decode(hex_str.trim()).map_err(|e| err(format!("point hex: {e}")))?;
    let ep = EncodedPoint::from_bytes(&bytes).map_err(|e| err(format!("point: {e}")))?;
    let a: Option<AffinePoint> = AffinePoint::from_encoded_point(&ep).into();
    a.map(ProjectivePoint::from)
        .ok_or_else(|| err("point is not on the curve"))
}

fn compressed(p: &ProjectivePoint) -> [u8; 33] {
    let ep = p.to_affine().to_encoded_point(true);
    ep.as_bytes()
        .try_into()
        .expect("compressed SEC1 is 33 bytes")
}

/// The secp256k1 generator, compressed hex — `g` in ErgoScript.
pub fn generator_hex() -> String {
    hex::encode(compressed(&ProjectivePoint::GENERATOR))
}

/// `g^x` for a 32-byte hex scalar, compressed hex.
pub fn pubkey_hex(x_hex: &str) -> Result<String, SandboxError> {
    let x = scalar(x_hex)?;
    Ok(hex::encode(compressed(&(ProjectivePoint::GENERATOR * x))))
}

/// `(g^x, h^x)` for bases `g`, `h` and scalar `x`, compressed hex.
pub fn dht_hex(g_hex: &str, h_hex: &str, x_hex: &str) -> Result<(String, String), SandboxError> {
    let x = scalar(x_hex)?;
    let g = point(g_hex)?;
    let h = point(h_hex)?;
    Ok((
        hex::encode(compressed(&(g * x))),
        hex::encode(compressed(&(h * x))),
    ))
}

fn registry(secrets: &[SecretSpec]) -> Result<SecretRegistry, SandboxError> {
    let mut externals = Vec::with_capacity(secrets.len());
    for s in secrets {
        match s {
            SecretSpec::Dlog(x_hex) => {
                let x = scalar(x_hex)?;
                externals.push(ProverExternalSecret::Dlog {
                    pk: compressed(&(ProjectivePoint::GENERATOR * x)),
                    scalar: Zeroizing::new(x),
                });
            }
            SecretSpec::Dht { g, h, x } => {
                let xs = scalar(x)?;
                let gp = point(g)?;
                let hp = point(h)?;
                externals.push(ProverExternalSecret::DhTuple {
                    g: compressed(&gp),
                    h: compressed(&hp),
                    u: compressed(&(gp * xs)),
                    v: compressed(&(hp * xs)),
                    scalar: Zeroizing::new(xs),
                });
            }
        }
    }
    SecretRegistry::empty()
        .merge_external_secrets(&externals)
        .map_err(|e| err(e.to_string()))
}

/// A proof for `proposition` over `message` from these secrets, or why
/// none can be made (a missing secret, an unprovable proposition).
pub fn prove(
    proposition: &SigmaBoolean,
    secrets: &[SecretSpec],
    message: &[u8],
) -> Result<Vec<u8>, SandboxError> {
    let reg = registry(secrets)?;
    prove_sigma(
        proposition,
        &reg,
        message,
        &HintsBag::empty(),
        &mut OsRngBackend,
    )
    .map(|(proof, _cost)| proof)
    .map_err(|e| err(e.to_string()))
}
