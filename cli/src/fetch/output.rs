use {
    anyhow::Result,
    solana_rpc_client_api::response::RpcConfirmedTransactionStatusWithSignature,
    std::path::{Path, PathBuf},
};

// Writes each recovered historical IDL using its slot as the output filename.
pub(super) fn save_historical_idls(
    idls: Vec<(RpcConfirmedTransactionStatusWithSignature, Vec<u8>)>,
    out_dir: Option<PathBuf>,
) -> Result<()> {
    for (sig, idl_data) in idls.iter() {
        write_idl_file(
            idl_data,
            &PathBuf::from(format!("idl_{}.json", sig.slot)),
            out_dir.as_deref(),
        )?;
    }

    Ok(())
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
