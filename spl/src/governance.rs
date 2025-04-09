/// A macro to embed the program ID and provide a wrapper for the SPL governance program's VoterWeightRecord type.
#[macro_export]
macro_rules! vote_weight_record {
    ($id:expr) => {
        /// Anchor wrapper for the SPL governance program's VoterWeightRecord type.
        #[derive(Clone)]
        pub struct VoterWeightRecord(spl_governance_addin_api::voter_weight::VoterWeightRecord);

        impl anchor_lang::AccountDeserialize for VoterWeightRecord {
            /// Tries to deserialize the account data into a VoterWeightRecord.
            fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
                let data: &[u8] = buf;
                let vwr = spl_governance_addin_api::voter_weight::VoterWeightRecord::deserialize(&mut data)
                    .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?;
                
                if !spl_governance_addin_api::voter_weight::VoterWeightRecord::is_initialized(&vwr) {
                    return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
                }

                Ok(VoterWeightRecord(vwr))
            }

            /// Tries to deserialize the account data without checking initialization.
            fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
                let data: &[u8] = buf;
                let vwr = spl_governance_addin_api::voter_weight::VoterWeightRecord::deserialize(&mut data)
                    .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?;
                
                Ok(VoterWeightRecord(vwr))
            }
        }

        impl anchor_lang::AccountSerialize for VoterWeightRecord {
            /// Serializes the VoterWeightRecord account data.
            fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> anchor_lang::Result<()> {
                self.0
                    .serialize(writer)
                    .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotSerialize)?;
                Ok(())
            }
        }

        impl anchor_lang::Owner for VoterWeightRecord {
            /// Returns the program ID as the owner of the account.
            fn owner() -> Pubkey {
                $id
            }
        }

        impl std::ops::Deref for VoterWeightRecord {
            type Target = spl_governance_addin_api::voter_weight::VoterWeightRecord;

            /// Provides immutable access to the inner VoterWeightRecord.
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for VoterWeightRecord {
            /// Provides mutable access to the inner VoterWeightRecord.
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        #[cfg(feature = "idl-build")]
        impl anchor_lang::IdlBuild for VoterWeightRecord {}

        #[cfg(feature = "idl-build")]
        impl anchor_lang::Discriminator for VoterWeightRecord {
            /// Returns an empty discriminator, since none is needed here.
            const DISCRIMINATOR: &'static [u8] = &[];
        }
    };
}
