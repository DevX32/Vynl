use std::path::{Path, PathBuf};
use std::sync::Mutex;

use base64::Engine;
use sha2::{Digest, Sha256};

pub(crate) struct SyncIdentity {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub fingerprint: String,
}

static IDENTITY_LOCK: Mutex<()> = Mutex::new(());

fn tls_dir(user_data: &Path) -> PathBuf {
    user_data.join("mobile_sync").join("tls")
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension(format!(
        "tmp.{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&tmp, contents)
        .map_err(|e| format!("could not write {}: {e}", tmp.display()))?;
    restrict_permissions(&tmp);
    std::fs::rename(&tmp, path).map_err(|e| format!("could not rename {}: {e}", path.display()))
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) {}

fn fingerprint_from_cert_pem(pem: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(pem).ok()?;
    let start = text.find("-----BEGIN CERTIFICATE-----")? + "-----BEGIN CERTIFICATE-----".len();
    let end = text.find("-----END CERTIFICATE-----")?;
    let body: String = text[start..end]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let der = base64::engine::general_purpose::STANDARD
        .decode(body.as_bytes())
        .ok()?;
    Some(hex::encode(Sha256::digest(&der)))
}

pub(crate) fn ensure_identity(user_data: &Path) -> Result<SyncIdentity, String> {
    let _guard = IDENTITY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tls_dir(user_data);
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create the sync tls dir: {e}"))?;
    let cert_path = dir.join("cert.pem");
    let key_path = dir.join("key.pem");

    if cert_path.is_file() && key_path.is_file() {
        if let Ok(pem) = std::fs::read(&cert_path)
            && let Some(fingerprint) = fingerprint_from_cert_pem(&pem)
        {
            return Ok(SyncIdentity {
                cert_path,
                key_path,
                fingerprint,
            });
        }
        let _ = std::fs::remove_file(&cert_path);
        let _ = std::fs::remove_file(&key_path);
    }

    let certified = rcgen::generate_simple_self_signed(vec!["vynl-mobile-sync".to_string()])
        .map_err(|e| format!("could not generate the sync certificate: {e}"))?;
    let fingerprint = hex::encode(Sha256::digest(certified.cert.der().as_ref()));
    atomic_write(&key_path, certified.signing_key.serialize_pem().as_bytes())?;
    restrict_permissions(&key_path);
    atomic_write(&cert_path, certified.cert.pem().as_bytes())?;
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
