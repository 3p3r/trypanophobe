use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::io::Read;
use tar::Archive;
use xz2::read::XzDecoder;

include!(concat!(env!("OUT_DIR"), "/embedded_chunks.rs"));

fn concat_chunks() -> Vec<u8> {
    let mut data = Vec::with_capacity(CHUNKS.iter().map(|c| c.len()).sum());
    for chunk in CHUNKS {
        data.extend_from_slice(chunk);
    }
    data
}

/// Decompress an xz tarball and return `(model.onnx, tokenizer.json)` bytes.
pub fn unpack_tar_xz(compressed: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut decoder = XzDecoder::new(compressed);
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
        .ok_or_else(|| anyhow!("model.onnx missing from archive"))?;
    let tokenizer = files
        .remove("tokenizer.json")
        .ok_or_else(|| anyhow!("tokenizer.json missing from archive"))?;

    Ok((model, tokenizer))
}

pub fn load_model_assets() -> Result<(Vec<u8>, Vec<u8>)> {
    unpack_tar_xz(&concat_chunks())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tar::Builder;
    use xz2::write::XzEncoder;

    fn fixture_tar_xz() -> Vec<u8> {
        let mut tar_buf = Vec::new();
        {
            let mut builder = Builder::new(&mut tar_buf);
            let mut model = tar::Header::new_gnu();
            model.set_size(5);
            model.set_cksum();
            builder
                .append_data(&mut model, "model.onnx", b"model" as &[u8])
                .unwrap();
            let mut tok = tar::Header::new_gnu();
            tok.set_size(4);
            tok.set_cksum();
            builder
                .append_data(&mut tok, "tokenizer.json", b"tok!" as &[u8])
                .unwrap();
            builder.finish().unwrap();
        }
        let mut xz_buf = Vec::new();
        let mut enc = XzEncoder::new(&mut xz_buf, 6);
        enc.write_all(&tar_buf).unwrap();
        enc.finish().unwrap();
        xz_buf
    }

    #[test]
    fn unpack_tar_xz_reads_fixture() {
        let (model, tok) = unpack_tar_xz(&fixture_tar_xz()).unwrap();
        assert_eq!(model, b"model");
        assert_eq!(tok, b"tok!");
    }

    #[test]
    fn unpack_skips_empty_tar_entry_names() {
        let mut tar_buf = Vec::new();
        {
            let mut builder = Builder::new(&mut tar_buf);
            let mut dir = tar::Header::new_gnu();
            dir.set_entry_type(tar::EntryType::Directory);
            dir.set_size(0);
            dir.set_cksum();
            builder.append(&dir, &[][..]).unwrap();
            let mut model = tar::Header::new_gnu();
            model.set_size(3);
            model.set_cksum();
            builder
                .append_data(&mut model, "model.onnx", b"abc" as &[u8])
                .unwrap();
            let mut tok = tar::Header::new_gnu();
            tok.set_size(2);
            tok.set_cksum();
            builder
                .append_data(&mut tok, "tokenizer.json", b"ok" as &[u8])
                .unwrap();
            builder.finish().unwrap();
        }
        let mut xz_buf = Vec::new();
        let mut enc = XzEncoder::new(&mut xz_buf, 6);
        enc.write_all(&tar_buf).unwrap();
        enc.finish().unwrap();
        let (model, tok) = unpack_tar_xz(&xz_buf).unwrap();
        assert_eq!(model, b"abc");
        assert_eq!(tok, b"ok");
    }

    #[test]
    fn unpack_errors_when_model_missing() {
        let mut tar_buf = Vec::new();
        {
            let mut builder = Builder::new(&mut tar_buf);
            let mut tok = tar::Header::new_gnu();
            tok.set_size(1);
            tok.set_cksum();
            builder
                .append_data(&mut tok, "tokenizer.json", b"x" as &[u8])
                .unwrap();
            builder.finish().unwrap();
        }
        let mut xz_buf = Vec::new();
        let mut enc = XzEncoder::new(&mut xz_buf, 6);
        enc.write_all(&tar_buf).unwrap();
        enc.finish().unwrap();

        let err = unpack_tar_xz(&xz_buf).unwrap_err();
        assert!(err.to_string().contains("model.onnx missing"));
    }

    #[test]
    fn embedded_chunks_are_non_empty() {
        assert!(!CHUNKS.is_empty());
        assert!(CHUNKS.iter().map(|c| c.len()).sum::<usize>() > 0);
    }
}
