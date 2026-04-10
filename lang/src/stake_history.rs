//! Stake history sysvar: wrapped so Anchor can implement account loading and
//! [`pinocchio::sysvars::Sysvar`] without hitting the orphan rules on foreign types.

pub use solana_stake_interface::{
    stake_history::{StakeHistoryEntry, MAX_ENTRIES},
    sysvar::stake_history::{check_id, ID},
};
use {
    serde::{Deserialize, Serialize},
    solana_stake_interface::stake_history::StakeHistory as StakeHistoryInner,
    std::ops::Deref,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct StakeHistory(pub StakeHistoryInner);

impl Deref for StakeHistory {
    type Target = StakeHistoryInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for StakeHistory {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StakeHistory {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self(StakeHistoryInner::deserialize(deserializer)?))
    }
}

impl solana_sysvar::Sysvar for StakeHistory {}

impl solana_sysvar_id::SysvarId for StakeHistory {
    fn id() -> crate::Pubkey {
        ID
    }

    fn check_id(pubkey: &crate::Pubkey) -> bool {
        check_id(pubkey)
    }
}

impl solana_sysvar::SysvarSerialize for StakeHistory {
    fn size_of() -> usize {
        <StakeHistoryInner as solana_sysvar::SysvarSerialize>::size_of()
    }
}

impl pinocchio::sysvars::Sysvar for StakeHistory {
    fn get() -> Result<Self, pinocchio::error::ProgramError> {
        Err(pinocchio::error::ProgramError::UnsupportedSysvar)
    }
}
