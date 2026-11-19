use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

pub(crate) struct SyncIdentity {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub fingerprint: String,
}

fn tls_dir(user_data: &Path) -> PathBuf {
    user_data.join("mobile_sync").join("tls")
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, contents)
        .map_err(|e| format!("could not write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("could not rename {}: {e}", path.display()))
}

pub(crate) fn ensure_identity(user_data: &Path) -> Result<SyncIdentity, String> {
    let dir = tls_dir(user_data);
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create the sync tls dir: {e}"))?;
    let cert_path = dir.join("cert.pem");
    let key_path = dir.join("key.pem");
    let fingerprint_path = dir.join("fingerprint");
    if cert_path.is_file() && key_path.is_file() && fingerprint_path.is_file() {
        if let Ok(fingerprint) = std::fs::read_to_string(&fingerprint_path) {
            let fingerprint = fingerprint.trim().to_string();
            if fingerprint.len() == 64 && fingerprint.chars().all(|c| c.is_ascii_hexdigit()) {
                return Ok(SyncIdentity {
                    cert_path,
                    key_path,
                    fingerprint,
                });
            }
        }
    }
    let certified = rcgen::generate_simple_self_signed(vec!["vynl-mobile-sync".to_string()])
        .map_err(|e| format!("could not generate the sync certificate: {e}"))?;
    let fingerprint = hex::encode(Sha256::digest(certified.cert.der().as_ref()));
    atomic_write(&cert_path, certified.cert.pem().as_bytes())?;
    atomic_write(&key_path, certified.signing_key.serialize_pem().as_bytes())?;
    atomic_write(&fingerprint_path, fingerprint.as_bytes())?;
    Ok(SyncIdentity {
        cert_path,
        key_path,
        fingerprint,
    })
}

pub(crate) fn fingerprint(user_data: &Path) -> Option<String> {
    ensure_identity(user_data)
        .ok()
        .map(|identity| identity.fingerprint)
}
