//! The emission constants rule 123's emission-box allocation check needs.
//!
//! The node checks how a transaction that spends the emission box allocates the emission NFT and
//! the re-emission token only when it is given the network's emission constants
//! (`ReemissionRuleInputs::emission`). They are a supplied premise inside `networkRules`, with that
//! premise's origin: nothing here says a named network or a historical height used them. A
//! request that carries none is validated without the allocation check, as before.
use ergo_chain_spec::MonetaryParams;
use ergo_primitives::reader::VlqReader;
use ergo_validation::tx::reemission::EmissionRuleInputs;
use serde::{Deserialize, Serialize};

/// Monetary schedule and emission-box identity, as the node's `EmissionRuleInputs`. The schedule
/// fields are the node's `MonetaryParams` (Scala `monetary.*`); `emissionNftId` is the NFT the
/// emission box carries and `emissionTree` the emission contract, both from the network's
/// settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmissionConstants {
    pub fixed_rate: u64,
    pub fixed_rate_period: u32,
    pub epoch_length: u32,
    pub one_epoch_reduction: u64,
    pub founders_initial_reward: u64,
    pub miner_reward_delay: u32,
    pub emission_nft_id: String,
    pub emission_tree: String,
}

impl EmissionConstants {
    /// The node's inputs, after the checks the pay-to-reemission tree premise gets. An
    /// `epochLength` of zero is refused: the node divides by it.
    pub fn node(&self, activated_script_version: u8) -> Result<EmissionRuleInputs, String> {
        if self.epoch_length == 0 {
            return Err("emission epochLength must be at least 1".into());
        }
        let emission_nft_id: [u8; 32] = hex::decode(&self.emission_nft_id)
            .map_err(|e| e.to_string())?
            .try_into()
            .map_err(|_| "emission NFT id must be 32 bytes".to_string())?;
        let emission_tree = hex::decode(&self.emission_tree).map_err(|e| e.to_string())?;
        let mut reader = VlqReader::new(&emission_tree);
        let parsed = ergo_ser::ergo_tree::read_ergo_tree(&mut reader).map_err(|e| e.to_string())?;
        if !reader.is_empty() {
            return Err("trailing emission tree bytes".into());
        }
        ergo_ser::ergo_tree::check_tree_version_supported(&parsed, activated_script_version)
            .map_err(|e| e.to_string())?;
        ergo_ser::ergo_tree::check_header_size_bit(&parsed).map_err(|e| e.to_string())?;
        ergo_ser::ergo_tree::check_resolvable_methods(&parsed).map_err(|e| e.to_string())?;
        ergo_ser::ergo_tree::check_sigma_prop_root(&parsed).map_err(|e| e.to_string())?;
        Ok(EmissionRuleInputs {
            monetary: MonetaryParams {
                fixed_rate: self.fixed_rate,
                fixed_rate_period: self.fixed_rate_period,
                epoch_length: self.epoch_length,
                one_epoch_reduction: self.one_epoch_reduction,
                founders_initial_reward: self.founders_initial_reward,
                miner_reward_delay: self.miner_reward_delay,
            },
            emission_nft_id,
            emission_tree,
        })
    }
}
