use crate::project_root;
use std::{path::Path, process::Command};

pub fn launch(schema_path: &Path) -> Result<(), String> {
    let root = project_root(schema_path);

    // 1. Try to find lite-orm-studio executable next to the current exe
    if let Ok(current_exe) = std::env::current_exe()
        && let Some(bin_dir) = current_exe.parent()
    {
        let studio_bin = if cfg!(windows) {
            bin_dir.join("lite-orm-studio.exe")
        } else {
            bin_dir.join("lite-orm-studio")
        };
        if studio_bin.exists() {
            println!(
                "Launching LiteORM Studio desktop app ({})...",
                studio_bin.display()
            );
            let status = Command::new(&studio_bin)
                .current_dir(root)
                .status()
                .map_err(|e| format!("Failed to launch {}: {e}", studio_bin.display()))?;
            if !status.success() {
                return Err(format!("LiteORM Studio exited with {status}"));
            }
            return Ok(());
        }
    }

    // 2. Try running `lite-orm-studio` from PATH
    if let Ok(status) = Command::new("lite-orm-studio").current_dir(root).status() {
        if !status.success() {
            return Err(format!("LiteORM Studio exited with {status}"));
        }
        return Ok(());
    }

    // 3. Fallback: try `cargo run -p lite-orm-studio` in the workspace root
    println!("Launching LiteORM Studio desktop app via cargo...");
    let status = Command::new("cargo")
        .args(["run", "-p", "lite-orm-studio"])
        .current_dir(root)
        .status()
        .map_err(|e| format!("Failed to run cargo: {e}"))?;

    if !status.success() {
        return Err(format!("LiteORM Studio exited with {status}"));
    }

    Ok(())
}

#[cfg(test)]
fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[cfg(test)]
fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let (chunks, rem) = input.as_chunks::<3>();
    for chunk in chunks {
        let b = ((chunk[0] as usize) << 16) | ((chunk[1] as usize) << 8) | (chunk[2] as usize);
        out.push(TABLE[(b >> 18) & 63] as char);
        out.push(TABLE[(b >> 12) & 63] as char);
        out.push(TABLE[(b >> 6) & 63] as char);
        out.push(TABLE[b & 63] as char);
    }
    match rem.len() {
        1 => {
            let b = (rem[0] as usize) << 16;
            out.push(TABLE[(b >> 18) & 63] as char);
            out.push(TABLE[(b >> 12) & 63] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let b = ((rem[0] as usize) << 16) | ((rem[1] as usize) << 8);
            out.push(TABLE[(b >> 18) & 63] as char);
            out.push(TABLE[(b >> 12) & 63] as char);
            out.push(TABLE[(b >> 6) & 63] as char);
            out.push('=');
        }
        _ => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_postgres_identifiers() {
        assert_eq!(quote("odd\"name"), "\"odd\"\"name\"");
    }

    #[test]
    fn encodes_base64_without_padding_errors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
    }
}
