use {
    super::history::{HistoricalIdlVersion, IdlHistorySource},
    anyhow::{anyhow, Result},
    std::{
        collections::BTreeMap,
        path::{Path, PathBuf},
    },
};

// Writes each recovered historical IDL using its slot as the output filename.
pub(super) fn save_historical_idls(
    idls: &[HistoricalIdlVersion],
    out_dir: Option<PathBuf>,
) -> Result<()> {
    let output_names = build_output_names(idls);

    for idl in idls {
        // The merged history path can emit multiple sources for the same slot, so always resolve
        // through the precomputed filename map before writing.
        let file_name = output_names
            .get(&(idl.slot, idl.signature.clone(), idl.source))
            .ok_or_else(|| {
                anyhow!(
                    "missing historical output name for slot {}, signature {}, source {:?}",
                    idl.slot,
                    idl.signature,
                    idl.source
                )
            })?;
        write_idl_file(&idl.idl_data, &PathBuf::from(file_name), out_dir.as_deref())?;
    }

    Ok(())
}

fn build_output_names(
    entries: &[HistoricalIdlVersion],
) -> BTreeMap<(u64, String, IdlHistorySource), String> {
    let mut counts = BTreeMap::<u64, usize>::new();
    for entry in entries {
        *counts.entry(entry.slot).or_default() += 1;
    }

    entries
        .iter()
        .map(|entry| {
            // Only add a source suffix when slot-based filenames would collide on disk.
            let file_name = if counts.get(&entry.slot).copied().unwrap_or_default() > 1 {
                format!("idl_{}_{}.json", entry.slot, entry.source.as_suffix())
            } else {
                format!("idl_{}.json", entry.slot)
            };
            (
                (entry.slot, entry.signature.clone(), entry.source),
                file_name,
            )
        })
        .collect()
}

// Writes one IDL file, creating the output directory only when an explicit base path is provided.
pub(super) fn write_idl_file(
    idl_data: &[u8],
    relative_path: &Path,
    out_dir: Option<&Path>,
) -> Result<()> {
    let path = match out_dir {
        Some(out_dir) => {
            std::fs::create_dir_all(out_dir)?;
            out_dir.join(relative_path)
        }
        None => relative_path.to_path_buf(),
    };
    std::fs::write(&path, idl_data)?;
    println!("Saved IDL to: {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_names_suffix_same_slot_conflicts() {
        let entries = vec![
            HistoricalIdlVersion {
                slot: 42,
                signature: "a".into(),
                source: IdlHistorySource::Legacy,
                idl_data: b"{\"a\":1}".to_vec(),
            },
            HistoricalIdlVersion {
                slot: 42,
                signature: "b".into(),
                source: IdlHistorySource::Pmp,
                idl_data: b"{\"a\":2}".to_vec(),
            },
        ];
        let names = build_output_names(&entries);
        assert_eq!(
            names
                .get(&(42, "a".into(), IdlHistorySource::Legacy))
                .unwrap(),
            "idl_42_legacy.json"
        );
        assert_eq!(
            names.get(&(42, "b".into(), IdlHistorySource::Pmp)).unwrap(),
            "idl_42_pmp.json"
        );
    }
}
