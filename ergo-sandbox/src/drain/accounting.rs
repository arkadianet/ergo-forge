//! Existing recognized-attacker-receipts-v1 accounting, shared without semantic changes.
use super::*;

/// Per-asset holdings of a box: `nanoErg` plus one entry per token id.
pub(crate) fn holdings(value: &i64, tokens: &[TokenAmount]) -> BTreeMap<String, u128> {
    let mut m = BTreeMap::new();
    if *value > 0 {
        m.insert("nanoErg".to_string(), *value as u128);
    }
    for t in tokens {
        *m.entry(t.id.to_lowercase()).or_default() += t.amount as u128;
    }
    m
}

pub(crate) fn is_victim(role: DrainRole) -> bool {
    matches!(role, DrainRole::Protected | DrainRole::Companion)
}

pub(crate) type Amounts = BTreeMap<String, u128>;

pub(crate) fn sum_boxes<'a>(boxes: impl Iterator<Item = &'a ScenarioBox>) -> Amounts {
    let mut sum = Amounts::new();
    for b in boxes {
        for (asset, amount) in holdings(&b.value, &b.tokens) {
            *sum.entry(asset).or_default() += amount;
        }
    }
    sum
}

pub(crate) fn tree(b: &ScenarioBox) -> String {
    b.ergo_tree
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_lowercase()
}

pub(crate) fn recognized_trees(req: &DrainRequest) -> Result<HashSet<String>, SandboxError> {
    let mut trees = HashSet::from(["10010101d17300".to_string()]);
    for key in &req.attacker_public_keys {
        let bytes = hex::decode(key.trim())
            .map_err(|e| SandboxError::Scenario(format!("attackerPublicKeys: {e}")))?;
        if bytes.len() != 33
            || !matches!(bytes[0], 2 | 3)
            || k256::PublicKey::from_sec1_bytes(&bytes).is_err()
        {
            return Err(SandboxError::Scenario(
                "attackerPublicKeys requires compressed SEC1 public keys".into(),
            ));
        }
        trees.insert(format!("0008cd{}", hex::encode(bytes)));
    }
    Ok(trees)
}

pub(crate) fn objective_errors(req: &DrainRequest) -> Vec<String> {
    let Some(policy) = &req.objective else {
        return vec![
            "incomplete objective: declare objective.terms (empty means no authorized releases)"
                .into(),
        ];
    };
    let mut errors = Vec::new();
    for (i, term) in policy.terms.iter().enumerate() {
        if !req
            .inputs
            .get(term.source_input)
            .is_some_and(|i| is_victim(i.role))
        {
            errors.push(format!("incomplete objective: term {i} sourceInput must name a declared victim spending input"));
        }
        let valid_asset = |asset: &str| {
            asset == "nanoErg" || hex::decode(asset).is_ok_and(|bytes| bytes.len() == 32)
        };
        if !valid_asset(&term.asset)
            || term
                .payment
                .as_ref()
                .is_some_and(|p| !valid_asset(&p.asset))
        {
            errors.push(format!(
                "incomplete objective: term {i} asset must be nanoErg or a 32-byte token id"
            ));
        }
        if term.destination_tree.trim().is_empty()
            || hex::decode(term.destination_tree.trim()).is_err()
        {
            errors.push(format!(
                "incomplete objective: term {i} invalid destinationTree"
            ));
        }
        if let Some(payment) = &term.payment {
            if payment.amount_per_unit == 0
                || payment.seller_tree.trim().is_empty()
                || hex::decode(payment.seller_tree.trim()).is_err()
                || payment
                    .retained_trees
                    .iter()
                    .any(|t| t.trim().is_empty() || hex::decode(t.trim()).is_err())
            {
                errors.push(format!(
                    "incomplete objective: term {i} invalid payment binding/rate"
                ));
            }
            // A bounded fixed-rate policy supports one claim per payment pool.
            // Exact duplicates are harmless. More complex joint exchanges
            // need an explicit allocation model; never silently double spend
            // consideration or issue a verdict with an underspecified policy.
            for prev in &policy.terms[..i] {
                if let Some(p) = &prev.payment {
                    if p.asset.eq_ignore_ascii_case(&payment.asset)
                        && p.seller_tree
                            .trim()
                            .eq_ignore_ascii_case(payment.seller_tree.trim())
                        && prev != term
                    {
                        errors.push(format!("incomplete objective: term {i} shares payment with a different term; joint payment allocation is unsupported"));
                    }
                }
            }
        }
    }
    errors
}

/// Measurement only; called after transaction validity and missing-key gates.
pub(crate) fn leak(
    req: &DrainRequest,
    victim: &Amounts,
    roles: &[DrainRole],
    inputs: &[ScenarioBox],
    outputs: &[ScenarioBox],
    recognized: &HashSet<String>,
) -> DrainAccounting {
    let indices: Vec<usize> = outputs
        .iter()
        .enumerate()
        .filter(|(_, b)| recognized.contains(&tree(b)))
        .map(|(i, _)| i)
        .collect();
    let attacker = sum_boxes(indices.iter().map(|&i| &outputs[i]));
    let input_total = sum_boxes(inputs.iter());
    let output_total = sum_boxes(outputs.iter());
    let mut outside = sum_boxes(
        inputs
            .iter()
            .zip(roles)
            .filter(|(_, r)| !is_victim(**r))
            .map(|(b, _)| b),
    );
    // On a valid realized transaction only the permitted mint id can have
    // positive token supply growth. ERG cannot be minted; burns add nothing.
    for (asset, amount) in &output_total {
        if asset != "nanoErg" {
            *outside.entry(asset.clone()).or_default() +=
                amount.saturating_sub(*input_total.get(asset).unwrap_or(&0));
        }
    }
    let mut terms_used = Vec::new();
    // Capacities: (asset, declared source, exact destination). Max deduplicates
    // overlapping claims; a max-flow below shares source and receipt holdings.
    let mut allowances: BTreeMap<(String, usize, String), u128> = BTreeMap::new();
    if let Some(policy) = &req.objective {
        for (i, term) in policy.terms.iter().enumerate() {
            let Some(source) = req
                .inputs
                .get(term.source_input)
                .filter(|s| is_victim(s.role))
            else {
                continue;
            };
            let asset = if term.asset == "nanoErg" {
                term.asset.clone()
            } else {
                term.asset.to_lowercase()
            };
            let source_amount = *holdings(&source.box_.value, &source.box_.tokens)
                .get(&asset)
                .unwrap_or(&0);
            let destination = term.destination_tree.trim().to_lowercase();
            let receipts = sum_boxes(
                indices
                    .iter()
                    .map(|&i| &outputs[i])
                    .filter(|b| tree(b) == destination),
            );
            let mut released = source_amount;
            let mut paid = None;
            let mut satisfied = true;
            if let Some(payment) = &term.payment {
                let seller = payment.seller_tree.trim().to_lowercase();
                let retained = sum_boxes(outputs.iter().filter(|b| {
                    tree(b) == seller
                        || payment
                            .retained_trees
                            .iter()
                            .any(|t| t.trim().eq_ignore_ascii_case(&tree(b)))
                }));
                released = source_amount.saturating_sub(*retained.get(&asset).unwrap_or(&0));
                let payment_asset = if payment.asset == "nanoErg" {
                    payment.asset.clone()
                } else {
                    payment.asset.to_lowercase()
                };
                let before = sum_boxes(inputs.iter().filter(|b| tree(b) == seller));
                let after = sum_boxes(outputs.iter().filter(|b| tree(b) == seller));
                let received = after
                    .get(&payment_asset)
                    .unwrap_or(&0)
                    .saturating_sub(*before.get(&payment_asset).unwrap_or(&0));
                satisfied = payment.amount_per_unit > 0
                    && released
                        .checked_mul(u128::from(payment.amount_per_unit))
                        .is_some_and(|required| received >= required);
                paid = Some(received.to_string());
            }
            let eligible = if satisfied {
                u128::from(term.max_amount)
                    .min(released)
                    .min(*receipts.get(&asset).unwrap_or(&0))
            } else {
                0
            };
            let cap = allowances
                .entry((asset, term.source_input, destination))
                .or_default();
            *cap = (*cap).max(eligible);
            terms_used.push(TermUsed {
                term_index: i,
                term: term.clone(),
                released: released.to_string(),
                payment_received: paid,
                satisfied,
                eligible: eligible.to_string(),
            });
        }
    }
    let mut sanctioned = Amounts::new();
    for asset in victim.keys() {
        // Source -> victim input -> recipient script -> sink. Grouping exact
        // scripts counts each realized output once even under duplicate terms.
        let destinations: Vec<String> = allowances
            .keys()
            .filter(|(a, _, _)| a == asset)
            .map(|(_, _, d)| d.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let sink = 1 + req.inputs.len() + destinations.len();
        let mut capacities = vec![vec![0u128; sink + 1]; sink + 1];
        for (i, input) in req
            .inputs
            .iter()
            .enumerate()
            .filter(|(_, i)| is_victim(i.role))
        {
            capacities[0][1 + i] = *holdings(&input.box_.value, &input.box_.tokens)
                .get(asset)
                .unwrap_or(&0);
        }
        for (j, destination) in destinations.iter().enumerate() {
            let node = 1 + req.inputs.len() + j;
            capacities[node][sink] = *sum_boxes(
                indices
                    .iter()
                    .map(|&i| &outputs[i])
                    .filter(|b| tree(b) == *destination),
            )
            .get(asset)
            .unwrap_or(&0);
            for ((a, source, d), amount) in &allowances {
                if a == asset && d == destination {
                    capacities[1 + source][node] = *amount;
                }
            }
        }
        sanctioned.insert(asset.clone(), max_flow(capacities, sink));
    }
    let assets: std::collections::BTreeSet<_> = victim
        .keys()
        .chain(attacker.keys())
        .chain(outside.keys())
        .chain(sanctioned.keys())
        .cloned()
        .collect();
    let strings = |amounts: &Amounts| {
        assets
            .iter()
            .map(|a| (a.clone(), amounts.get(a).unwrap_or(&0).to_string()))
            .collect()
    };
    let extracted = victim
        .iter()
        .filter_map(|(asset, v)| {
            let amount = attacker
                .get(asset)
                .unwrap_or(&0)
                .saturating_sub(*outside.get(asset).unwrap_or(&0))
                .saturating_sub(*sanctioned.get(asset).unwrap_or(&0))
                .min(*v);
            (amount > 0).then(|| (asset.clone(), amount.to_string()))
        })
        .collect();
    let victim_trees: HashSet<_> = req
        .inputs
        .iter()
        .filter(|i| is_victim(i.role))
        .map(|i| tree(&i.box_))
        .collect();
    let retained = sum_boxes(outputs.iter().filter(|b| victim_trees.contains(&tree(b))));
    let custody_deficit = victim
        .iter()
        .filter_map(|(a, v)| {
            let deficit = v.saturating_sub(*retained.get(a).unwrap_or(&0));
            (deficit > 0).then(|| (a.clone(), deficit.to_string()))
        })
        .collect();
    let unknown_output_indices = (0..outputs.len())
        .filter(|i| !indices.contains(i))
        .collect();
    DrainAccounting {
        v: strings(victim),
        a: strings(&attacker),
        n: strings(&outside),
        s: strings(&sanctioned),
        recognized_output_indices: indices,
        unknown_output_indices,
        custody_deficit,
        terms_used,
        extracted,
    }
}

/// Edmonds-Karp on the small allowance graph. Unlike greedy allocation this
/// obtains an upper bound independent of term/source ordering.
pub(crate) fn max_flow(mut residual: Vec<Vec<u128>>, sink: usize) -> u128 {
    let mut total = 0;
    loop {
        let mut parents = vec![usize::MAX; residual.len()];
        parents[0] = 0;
        let mut queue = std::collections::VecDeque::from([0]);
        while let Some(u) = queue.pop_front() {
            for (v, parent) in parents.iter_mut().enumerate() {
                if *parent == usize::MAX && residual[u][v] > 0 {
                    *parent = u;
                    queue.push_back(v);
                }
            }
        }
        if parents[sink] == usize::MAX {
            return total;
        }
        let mut amount = u128::MAX;
        let mut v = sink;
        while v != 0 {
            let u = parents[v];
            amount = amount.min(residual[u][v]);
            v = u;
        }
        v = sink;
        while v != 0 {
            let u = parents[v];
            residual[u][v] -= amount;
            residual[v][u] += amount;
            v = u;
        }
        total += amount;
    }
}
