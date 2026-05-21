use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// One check unit: display name (path or `literal:N`) and prompt text.
#[derive(Debug, Clone)]
pub struct CheckItem {
    pub name: String,
    pub text: String,
}

/// Expand CLI arguments into check items (files, directory `.prompt` scan, or literals).
pub fn collect_check_items(args: &[String]) -> Result<Vec<CheckItem>> {
    let mut items = Vec::new();
    let mut literal_idx = 0usize;

    for arg in args {
        let path = Path::new(arg);
        if path.exists() {
            if path.is_file() {
                let text = fs::read_to_string(path)
                    .with_context(|| format!("read file {}", path.display()))?;
                items.push(CheckItem {
                    name: path.display().to_string(),
                    text,
                });
            } else if path.is_dir() {
                let mut files = Vec::new();
                collect_prompt_files(path, &mut files)?;
                if files.is_empty() {
                    bail!("no .prompt files under {}", path.display());
                }
                files.sort();
                for file in files {
                    let text = fs::read_to_string(&file)
                        .with_context(|| format!("read {}", file.display()))?;
                    items.push(CheckItem {
                        name: file.display().to_string(),
                        text,
                    });
                }
            } else {
                bail!("not a file or directory: {}", path.display());
            }
        } else {
            literal_idx += 1;
            items.push(CheckItem {
                name: format!("literal:{literal_idx}"),
                text: arg.clone(),
            });
        }
    }

    if items.is_empty() {
        bail!("no inputs: pass file paths, directories, or prompt text");
    }

    Ok(items)
}

fn collect_prompt_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("read dir {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_prompt_files(&path, out)?;
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("prompt"))
        {
            out.push(path);
        }
    }
    Ok(())
}
