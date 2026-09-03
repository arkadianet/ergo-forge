//! Storage rent: every box on Ergo pays rent after a storage period, and a
//! box whose value cannot cover the fee is swept. Users of any contract
//! should see this, not just the burn recipe.

use ergo_sandbox::rent::{estimate, STORAGE_FEE_FACTOR, STORAGE_PERIOD};

#[test]
fn a_minimal_box_costs_about_a_tenth_of_an_erg_per_period() {
    // sigmaProp(true) tree, no tokens, no registers.
    let tree = hex::decode("10010101d17300").unwrap();
    let r = estimate(&tree, &[], &[], None);
    assert_eq!(r.period_blocks, STORAGE_PERIOD);
    assert_eq!(r.fee_factor, STORAGE_FEE_FACTOR);
    // ~50 bytes → ~0.06 ERG; the exact size includes ids and lengths.
    assert!(r.box_bytes >= 45 && r.box_bytes <= 80, "{}", r.box_bytes);
    assert_eq!(
        r.fee_nanoerg,
        r.box_bytes as u64 * STORAGE_FEE_FACTOR as u64
    );
    assert!(
        r.fee_nanoerg > 50_000_000 && r.fee_nanoerg < 120_000_000,
        "{}",
        r.fee_nanoerg
    );
    assert_eq!(r.next_collection_height, None);
}

#[test]
fn tokens_and_registers_make_the_box_bigger_and_the_fee_higher() {
    let tree = hex::decode("10010101d17300").unwrap();
    let plain = estimate(&tree, &[], &[], None);
    let with = estimate(&tree, &[1000, 5], &[hex::decode("040a").unwrap()], None);
    assert!(
        with.box_bytes > plain.box_bytes + 60,
        "{} vs {}",
        with.box_bytes,
        plain.box_bytes
    );
    assert!(with.fee_nanoerg > plain.fee_nanoerg);
}

#[test]
fn the_next_collection_is_one_period_after_creation() {
    let tree = hex::decode("10010101d17300").unwrap();
    let r = estimate(&tree, &[], &[], Some(1_000_000));
    assert_eq!(r.next_collection_height, Some(1_000_000 + STORAGE_PERIOD));
}
