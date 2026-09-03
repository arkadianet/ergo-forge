//! Spending proofs from secrets, through the node's own wallet prover
//! (`ergo-wallet::proving::sigma::prove_sigma`: Schnorr for `proveDlog`,
//! Diffie-Hellman tuples, AND / OR / k-of-n with simulated branches and
//! Fiat-Shamir). A scenario that names secrets gets a real proof for the
//! proposition its script reduced to, and that proof is then checked
//! through the consensus verification path like any supplied proof.

use ergo_primitives::group_element::GroupElement;
use ergo_ser::sigma_value::SigmaBoolean;
use ergo_wallet::proving::commitments::generate_commitments_for;
use ergo_wallet::proving::external::ProverExternalSecret;
use ergo_wallet::proving::extract::bag_for_multisig;
use ergo_wallet::proving::hints::{Hint, HintsBag};
use ergo_wallet::proving::randomness::OsRngBackend;
use ergo_wallet::proving::secrets::SecretRegistry;
use ergo_wallet::proving::sigma::{prove_sigma, prove_sigma_partial};
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

/// One signer in a multi-party ceremony: the secrets it alone holds.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartySpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub secrets: Vec<SecretSpec>,
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

/// The sigma leaves these secrets can prove.
fn images(secrets: &[SecretSpec]) -> Result<Vec<SigmaBoolean>, SandboxError> {
    let mut out = Vec::with_capacity(secrets.len());
    for s in secrets {
        out.push(match s {
            SecretSpec::Dlog(x_hex) => {
                let x = scalar(x_hex)?;
                SigmaBoolean::ProveDlog(GroupElement::from_bytes(compressed(
                    &(ProjectivePoint::GENERATOR * x),
                )))
            }
            SecretSpec::Dht { g, h, x } => {
                let xs = scalar(x)?;
                let gp = point(g)?;
                let hp = point(h)?;
                SigmaBoolean::ProveDHTuple {
                    g: GroupElement::from_bytes(compressed(&gp)),
                    h: GroupElement::from_bytes(compressed(&hp)),
                    u: GroupElement::from_bytes(compressed(&(gp * xs))),
                    v: GroupElement::from_bytes(compressed(&(hp * xs))),
                }
            }
        });
    }
    Ok(out)
}

/// Every atomic leaf of a proposition, depth-first.
fn leaves(prop: &SigmaBoolean, out: &mut Vec<SigmaBoolean>) {
    match prop {
        SigmaBoolean::TrivialProp(_) => {}
        SigmaBoolean::ProveDlog(_) | SigmaBoolean::ProveDHTuple { .. } => out.push(prop.clone()),
        SigmaBoolean::Cand(cs) | SigmaBoolean::Cor(cs) => cs.iter().for_each(|c| leaves(c, out)),
        SigmaBoolean::Cthreshold { children, .. } => children.iter().for_each(|c| leaves(c, out)),
    }
}

/// A proof made the way separate wallets make one: each party commits to
/// its own leaves; the first signs against everyone's commitments (a
/// partial proof); each next party extracts what came before and adds
/// its own signature; the last one's proof is complete. No registry
/// ever holds two parties' secrets. Mirrors Scala's `ProverUtils` flow.
pub fn prove_parties(
    proposition: &SigmaBoolean,
    parties: &[PartySpec],
    message: &[u8],
) -> Result<Vec<u8>, SandboxError> {
    if parties.is_empty() {
        return Err(err("no parties"));
    }
    let mut rng = OsRngBackend;
    let regs: Vec<SecretRegistry> = parties
        .iter()
        .map(|p| registry(&p.secrets))
        .collect::<Result<_, _>>()?;
    let imgs: Vec<Vec<SigmaBoolean>> = parties
        .iter()
        .map(|p| images(&p.secrets))
        .collect::<Result<_, _>>()?;
    let owned: Vec<SigmaBoolean> = imgs.iter().flatten().cloned().collect();
    let mut all_leaves = Vec::new();
    leaves(proposition, &mut all_leaves);
    let nobody: Vec<SigmaBoolean> = all_leaves
        .into_iter()
        .filter(|l| !owned.contains(l))
        .collect();
    // Round 0: everyone commits to their own leaves.
    let bags: Vec<HintsBag> = imgs
        .iter()
        .map(|im| {
            generate_commitments_for(proposition, im, &mut rng).map_err(|e| err(e.to_string()))
        })
        .collect::<Result<_, _>>()?;
    let public_of = |i: usize| HintsBag {
        hints: bags[i]
            .hints
            .iter()
            .filter(|h| matches!(h, Hint::RealCommitment(_)))
            .cloned()
            .collect(),
    };
    // Rounds 1..n: sign in order, each on top of what came before.
    let mut proof: Option<Vec<u8>> = None;
    for (i, name) in parties.iter().enumerate() {
        let who = name
            .name
            .clone()
            .unwrap_or_else(|| format!("party {}", i + 1));
        let mut hints = bags[i].clone();
        for (j, _) in parties.iter().enumerate() {
            if j != i {
                hints.extend(public_of(j));
            }
        }
        if let Some(prev) = &proof {
            let real: Vec<SigmaBoolean> = imgs[..i].iter().flatten().cloned().collect();
            let extracted = bag_for_multisig(proposition, prev, &real, &nobody)
                .map_err(|e| err(format!("{who}: extracting earlier signatures: {e}")))?;
            hints.extend(extracted);
        }
        let last = i + 1 == parties.len();
        let r = if last {
            prove_sigma(proposition, &regs[i], message, &hints, &mut rng)
        } else {
            prove_sigma_partial(proposition, &regs[i], message, &hints, &mut rng)
        };
        proof = Some(r.map(|(p, _)| p).map_err(|e| err(format!("{who}: {e}")))?);
    }
    Ok(proof.expect("at least one party"))
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
