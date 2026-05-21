use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::io::Read;
use tar::Archive;
use xz2::read::XzDecoder;

include!(concat!(env!("OUT_DIR"), "/embedded_chunks.rs"));

fn get_compressed_archive() -> Vec<u8> {
    let mut data = Vec::with_capacity(CHUNKS.iter().map(|c| c.len()).sum());
    for chunk in CHUNKS {
        data.extend_from_slice(chunk);
    }
    data
}

pub fn load_model_assets() -> Result<(Vec<u8>, Vec<u8>)> {
    let compressed = get_compressed_archive();
    let mut decoder = XzDecoder::new(compressed.as_slice());
    let mut tar_bytes = Vec::new();
    decoder
        .read_to_end(&mut tar_bytes)
        .context("xz decompress")?;

    let mut archive = Archive::new(tar_bytes.as_slice());
    let mut files: HashMap<String, Vec<u8>> = HashMap::new();

    for entry in archive.entries().context("read tar entries")? {
        let mut entry = entry.context("tar entry")?;
        let path = entry.path().context("entry path")?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).context("read entry")?;
        files.insert(name, buf);
    }

    let model = files
        .remove("model.onnx")
        .ok_or_else(|| anyhow!("model.onnx missing from embedded archive"))?;
    let tokenizer = files
        .remove("tokenizer.json")
        .ok_or_else(|| anyhow!("tokenizer.json missing from embedded archive"))?;

    Ok((model, tokenizer))
}
