use std::str::FromStr;

use arch_program::rune::RuneAmount;
use arch_program::{rune::RuneId, utxo::UtxoMeta};
use bitcoin::Txid;
use saturn_collections::generic::fixed_set::{FixedCapacitySet, FixedSetError};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::utxo_info::UtxoInfo;

#[cfg(feature = "utxo-consolidation")]
use crate::utxo_info::FixedOptionF64;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct UtxoInfoJson {
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub txid: Txid,
    pub vout: u32,
    #[serde(
        serialize_with = "crate::serde::serialize_u64",
        deserialize_with = "crate::serde::deserialize_u64"
    )]
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub value: u64,
    pub runes: Vec<RuneAmountJson>,
    pub needs_consolidation: NeedsConsolidation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuneAmountJson {
    #[cfg_attr(feature = "utoipa", schema(
        value_type = String,
        format = "token",
        example = "1234:5678"
    ))]
    pub id: RuneId, // 12 bytes
    #[serde(
        serialize_with = "crate::serde::serialize_u128",
        deserialize_with = "crate::serde::deserialize_u128"
    )]
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub amount: u128,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type", content = "value")]
#[repr(C)]
pub enum NeedsConsolidation {
    #[default]
    #[serde(rename = "no")]
    No,
    /// Internal f64 represents mempool fee rate at moment of creation.
    #[serde(rename = "yes")]
    Yes(f64),
}

#[cfg(feature = "utxo-consolidation")]
impl Into<NeedsConsolidation> for FixedOptionF64 {
    fn into(self) -> NeedsConsolidation {
        match self.as_ref() {
            Some(&fee) => NeedsConsolidation::Yes(fee),
            None => NeedsConsolidation::No,
        }
    }
}

#[cfg(feature = "utxo-consolidation")]
impl Into<FixedOptionF64> for NeedsConsolidation {
    fn into(self) -> FixedOptionF64 {
        match self {
            Self::Yes(fee) => FixedOptionF64::some(fee),
            Self::No => FixedOptionF64::none(),
        }
    }
}

impl<RuneSet: FixedCapacitySet<Item = RuneAmount>> Into<UtxoInfoJson> for &UtxoInfo<RuneSet> {
    fn into(self) -> UtxoInfoJson {
        let runes = {
            #[cfg(feature = "runes")]
            {
                self.runes
                    .as_slice()
                    .iter()
                    .map(|rune_amount| RuneAmountJson {
                        amount: rune_amount.amount,
                        id: rune_amount.id,
                    })
                    .collect::<Vec<_>>()
            }
            #[cfg(not(feature = "runes"))]
            {
                vec![]
            }
        };

        UtxoInfoJson {
            txid: Txid::from_str(&hex::encode(&self.meta.txid())).unwrap(),
            vout: self.meta.vout(),
            value: self.value,
            runes,
            #[cfg(feature = "utxo-consolidation")]
            needs_consolidation: self.needs_consolidation.into(),
            #[cfg(not(feature = "utxo-consolidation"))]
            needs_consolidation: NeedsConsolidation::No,
        }
    }
}

impl<RuneSet: FixedCapacitySet<Item = RuneAmount> + Default> TryInto<UtxoInfo<RuneSet>>
    for UtxoInfoJson
{
    type Error = FixedSetError;

    fn try_into(self) -> Result<UtxoInfo<RuneSet>, FixedSetError> {
        let runes = {
            #[cfg(feature = "runes")]
            {
                let mut rune_set = RuneSet::default();
                for rune_amount in self.runes.iter() {
                    rune_set.insert(RuneAmount {
                        amount: rune_amount.amount,
                        id: rune_amount.id,
                    })?;
                }

                rune_set
            }

            #[cfg(not(feature = "runes"))]
            {
                RuneSet::default()
            }
        };

        let mut info: UtxoInfo<RuneSet> = Default::default();
        info.meta = UtxoMeta::from_outpoint(self.txid, self.vout);
        info.value = self.value;
        #[cfg(feature = "runes")]
        {
            info.runes = runes;
        }
        #[cfg(feature = "utxo-consolidation")]
        {
            info.needs_consolidation = self.needs_consolidation.into();
        }

        Ok(info)
    }
}

impl<RuneSet: FixedCapacitySet<Item = RuneAmount>> Serialize for UtxoInfo<RuneSet> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let utxo_info_json: UtxoInfoJson = self.into();

        utxo_info_json.serialize(serializer)
    }
}

impl<'de, RuneSet: FixedCapacitySet<Item = RuneAmount> + Default> Deserialize<'de>
    for UtxoInfo<RuneSet>
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let utxo_info_json = UtxoInfoJson::deserialize(deserializer)?;

        Ok(utxo_info_json.try_into().map_err(|e: FixedSetError| {
            serde::de::Error::custom(format!(
                "Failed to convert UtxoInfoJson to UtxoInfo: {}",
                e.to_string()
            ))
        })?)
    }
}
