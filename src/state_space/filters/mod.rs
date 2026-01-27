pub mod blacklist;
pub mod value;
pub mod whitelist;

use async_trait::async_trait;

use crate::amms::{amm::AMM, error::AMMError};
use std::fmt::Debug;

#[async_trait]
pub trait AMMFilter: Send + Sync + AMMFilterClone {
    async fn filter(&self, amms: Vec<AMM>) -> Result<Vec<AMM>, AMMError>;
    fn stage(&self) -> FilterStage;
}

impl Debug for Box<dyn AMMFilter> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PoolFilter").finish()
    }
}

impl Clone for Box<dyn AMMFilter> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub trait AMMFilterClone {
    fn clone_box(&self) -> Box<dyn AMMFilter>;
}

impl<T> AMMFilterClone for T
where
    T: AMMFilter + Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn AMMFilter> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterStage {
    Discovery,
    Sync,
}

