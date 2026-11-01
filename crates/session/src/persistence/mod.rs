//! Validated records and durable storage are separate from live frontend/race ownership.
pub mod disk;
pub mod paths;
pub mod preferences;
use crate::{Result, career::Career, catalog::Catalog};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
pub const SAVE_SCHEMA: u8 = 2;
pub const MAX_BYTES: usize = 65536;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentPack {
    pub id: String,
    pub version: u32,
    pub sha256: String,
    pub simulation: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requirements {
    pub packs: Vec<ContentPack>,
}
impl Requirements {
    pub fn base(catalog: &Catalog) -> Self {
        Self {
            packs: vec![ContentPack {
                id: "rr:base".into(),
                version: 1,
                sha256: if catalog.source_verified {
                    rr_formats::pe::EXE_SHA256.into()
                } else {
                    format!("{:x}", Sha256::digest(b"rr-authored-base-1"))
                },
                simulation: true,
            }],
        }
    }
    pub fn validate(&self) -> Result<()> {
        let mut ids = std::collections::BTreeSet::new();
        if self.packs.is_empty() || self.packs.len() > 32 {
            return Err("save content list must contain 1..32 packs".into());
        }
        for p in &self.packs {
            if p.id.is_empty()
                || p.id.len() > 80
                || !p
                    .id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b":._-".contains(&c))
                || p.version == 0
                || p.sha256.len() != 64
                || !p.sha256.bytes().all(|c| c.is_ascii_hexdigit())
                || !ids.insert(&p.id)
            {
                return Err("invalid/duplicate saved content identity".into());
            }
        }
        if !self.packs.iter().any(|p| p.id == "rr:base" && p.simulation) {
            return Err("save is missing its base content identity".into());
        }
        Ok(())
    }
    pub fn compatible(&self, active: &Self) -> Result<Vec<String>> {
        self.validate()?;
        active.validate()?;
        let mut warnings = vec![];
        for p in &self.packs {
            if !active.packs.iter().any(|a| a == p) {
                if p.simulation {
                    return Err(format!(
                        "Required content unavailable or changed: {} version {} ({}). Restore this exact pack before loading; active career was not changed.",
                        p.id, p.version, p.sha256
                    ));
                }
                warnings.push(format!(
                    "Presentation pack {} unavailable; native appearance will be used.",
                    p.id
                ));
            }
        }
        // An extra simulation pack would also change the recorded career contract.
        for p in active.packs.iter().filter(|p| p.simulation) {
            if !self.packs.iter().any(|s| s == p) {
                return Err(format!(
                    "Active simulation pack {} was not recorded in this save; disable it before loading.",
                    p.id
                ));
            }
        }
        Ok(warnings)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Body {
    career: Career,
    requirements: Requirements,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Save {
    schema_version: u8,
    body: Body,
    sha256: String,
}
#[derive(Clone, Debug)]
pub struct Loaded {
    pub career: Career,
    pub requirements: Requirements,
    pub migrations: Vec<String>,
    pub warnings: Vec<String>,
}
pub fn encode(career: &Career, catalog: &Catalog, requirements: &Requirements) -> Result<Vec<u8>> {
    career.validate(catalog)?;
    requirements.validate()?;
    let body = Body {
        career: career.clone(),
        requirements: requirements.clone(),
    };
    let hash = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&body).map_err(|e| e.to_string())?)
    );
    let bytes = serde_json::to_vec_pretty(&Save {
        schema_version: SAVE_SCHEMA,
        body,
        sha256: hash,
    })
    .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_BYTES {
        return Err("career record exceeds 64 KiB".into());
    }
    Ok(bytes)
}
pub fn decode(bytes: &[u8], catalog: &Catalog, active: &Requirements) -> Result<Loaded> {
    if bytes.len() > MAX_BYTES {
        return Err("career record exceeds 64 KiB".into());
    }
    if bytes.get(4..8) == Some(&[0xad, 0xde, 0xfe, 0xca]) {
        return Err("Original binary Road Rash saves are not supported; import scope is documented in SAVES.md. No active career was changed.".into());
    }
    let header: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("Invalid/truncated career JSON: {e}"))?;
    let version = header
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .ok_or("Missing career schema version")?;
    if version == 1 {
        let career = Career::decode(bytes, catalog)?;
        active.validate()?;
        return Ok(Loaded{career,requirements:active.clone(),migrations:vec!["career v1 -> v2 (validated IDs; content identity attached on next save)".into()],warnings:vec!["This older snapshot did not record content hashes. Its original bytes are preserved until an explicit save; a backup is retained during upgrade.".into()]});
    }
    if version != u64::from(SAVE_SCHEMA) {
        return Err(format!(
            "Unsupported career schema {version}; this build reads versions 1 and 2."
        ));
    }
    let save: Save = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let actual = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&save.body).map_err(|e| e.to_string())?)
    );
    if save.sha256 != actual {
        return Err("Career integrity check failed; use Restore Backup to inspect/recover the previous valid save.".into());
    }
    save.body.career.validate(catalog)?;
    let warnings = save.body.requirements.compatible(active)?;
    Ok(Loaded {
        career: save.body.career,
        requirements: save.body.requirements,
        migrations: vec![],
        warnings,
    })
}
