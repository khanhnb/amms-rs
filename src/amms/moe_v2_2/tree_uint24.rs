use alloy::primitives::U256;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct TreeUint24 {
    pub level_0: U256,
    pub level_1: HashMap<U256, U256>,
    pub level_2: HashMap<U256, U256>,
}

impl TreeUint24 {
    pub fn add(&mut self, id: u32) -> bool {
        let key2 = U256::from(id) >> 8;
        let leaves = *self.level_2.entry(key2).or_insert(U256::ZERO);
        let new_leaves = leaves | (U256::ONE << (id & u8::MAX as u32));
        if new_leaves == leaves {
            return false;
        }
        self.level_2
            .entry(key2)
            .and_modify(|leaves| *leaves = new_leaves);
        if leaves == U256::ZERO {
            let key1 = key2 >> 8;
            let leaves = *self.level_1.entry(key1).or_insert(U256::ZERO);
            self.level_1
                .entry(key1)
                .and_modify(|f| *f = leaves | (U256::ONE << (key2 & U256::from(u8::MAX))));
            if leaves == U256::ZERO {
                self.level_0 |= U256::ONE << (key1 & U256::from(u8::MAX));
            }
        }
        true
    }

    // function contains(TreeUint24 storage tree, uint24 id) internal view returns (bool) {
    //     bytes32 leaf2 = bytes32(uint256(id) >> 8);
    //
    //     return tree.level2[leaf2] & bytes32(1 << (id & type(uint8).max)) != 0;
    // }
    //
    pub fn contains(&self, id: u32) -> bool {
        let leaf2 = U256::from(id) >> 8;
        self.level_2.get(&leaf2).unwrap_or(&U256::ZERO) & (U256::ONE << (id & u8::MAX as u32))
            != U256::ZERO
    }
}
