use alloy::primitives::Address;
use async_trait::async_trait;
use std::collections::HashSet;
use crate::{amms::{amm::{AMM, AutomatedMarketMaker}, error::AMMError}, state_space::filters::{AMMFilter, FilterStage}};

#[derive(Debug, Clone)]
pub struct CheckpointFilter {
    pub pools: HashSet<Address>,
}

impl CheckpointFilter {
    pub fn new(pools: Vec<Address>) -> Self {
        Self {
            pools: pools.into_iter().collect(),
        }
    }
}

#[async_trait]
impl AMMFilter for CheckpointFilter {
    /// Only filter pools that are in the checkpoint
    async fn filter(&self, amms: Vec<AMM>) -> Result<Vec<AMM>, AMMError> {
        Ok(amms
            .into_iter()
            .filter(|amm| self.pools.contains(&amm.address()))
            .collect())
    }

    fn stage(&self) -> FilterStage {
        FilterStage::Discovery
    }
}

