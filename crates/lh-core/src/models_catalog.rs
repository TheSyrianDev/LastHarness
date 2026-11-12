//! Models data layer: types and pure helpers for the Models page.
//!
//! No network here. The Hub adapter ([`HfdCatalog`], feature `hf-catalog`) implements
//! [`CatalogSource`] on top of hfd-core. Tests use [`FakeCatalog`].

use std::fmt;
use std::path::PathBuf;

/// One repo from a Hub search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoHit {
    pub repo: String,
    pub name: String,
    pub downloads: u64,
    pub has_gguf: bool,
}

/// One file inside a repo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogFile {
    pub path: String,
    pub size_bytes: u64,
    /// `(index, total)` for a split GGUF file, e.g. `(2, 5)` for `-00002-of-00005.gguf`.
    pub shard: Option<(u32, u32)>,
    /// True for a vision projector file (name contains `mmproj`).
    pub mmproj: bool,
}

impl CatalogFile {
    /// Build a file entry, reading shard and mmproj info from the path.
    pub fn new(path: &str, size_bytes: u64) -> Self {
        Self {
            path: path.to_string(),
            size_bytes,
            shard: parse_shard(path),
            mmproj: mmproj(path),
        }
    }
}

/// All shards and files of one quantisation, for one repo.
#[derive(Debug, Clone, PartialEq)]
pub struct QuantGroup {
    pub quant: String,
    pub bits_per_weight: Option<f32>,
    pub files: Vec<CatalogFile>,
    pub total_bytes: u64,
    pub shards: u32,
}

/// Whether a model fits in memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    /// Fits in free VRAM.
    Fits,
    /// Does not fit in VRAM but fits in free RAM plus VRAM.
    Tight,
    /// Does not fit even in RAM plus VRAM.
    TooBig,
    /// Size is not known.
    Unknown,
}

/// Progress state of one download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadState {
    Queued,
    Running { downloaded: u64, total: u64, bytes_per_sec: u64 },
    Done { local_path: String },
    Failed { message: String },
    Cancelled,
}

/// One download job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadJob {
    pub id: String,
    pub repo: String,
    pub path: String,
    pub dest_dir: String,
    pub state: DownloadState,
}

/// Settings for the downloader. Only carries the option for now; nothing writes the log yet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DownloadConfig {
    /// Where the downloader should write its JSON download log, if anywhere.
    pub download_log: Option<PathBuf>,
}

/// Error from a catalog source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    Message(String),
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CatalogError::Message(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for CatalogError {}

/// Where repo and file data comes from. Sync on purpose; the adapter decides how to block.
pub trait CatalogSource {
    /// Search repos by text. With `gguf_only`, only repos that hold a GGUF file.
    fn search(&self, query: &str, gguf_only: bool) -> Result<Vec<RepoHit>, CatalogError>;
    /// List the files of one repo.
    fn files(&self, repo: &str) -> Result<Vec<CatalogFile>, CatalogError>;
}

/// Fixed-data catalog for tests and the UI before the real adapter exists.
#[derive(Debug, Clone, Default)]
pub struct FakeCatalog {
    pub repos: Vec<(RepoHit, Vec<CatalogFile>)>,
}

impl CatalogSource for FakeCatalog {
    fn search(&self, query: &str, gguf_only: bool) -> Result<Vec<RepoHit>, CatalogError> {
        let q = query.to_lowercase();
        Ok(self
            .repos
            .iter()
            .map(|(hit, _)| hit)
            .filter(|hit| !gguf_only || hit.has_gguf)
            .filter(|hit| {
                q.is_empty()
                    || hit.repo.to_lowercase().contains(&q)
                    || hit.name.to_lowercase().contains(&q)
            })
            .cloned()
            .collect())
    }

    fn files(&self, repo: &str) -> Result<Vec<CatalogFile>, CatalogError> {
        self.repos
            .iter()
            .find(|(hit, _)| hit.repo == repo)
            .map(|(_, files)| files.clone())
            .ok_or_else(|| CatalogError::Message(format!("unknown repo: {repo}")))
    }
}

/// Known quant names with approximate bits per weight (from llama.cpp quant tables).
const QUANT_BITS: &[(&str, f32)] = &[
    ("Q2_K", 2.63),
    ("Q3_K_S", 3.5),
    ("Q3_K_M", 3.91),
    ("Q3_K_L", 4.27),
    ("Q4_0", 4.5),
    ("Q4_K_S", 4.58),
    ("Q4_K_M", 4.85),
    ("Q5_0", 5.5),
    ("Q5_K_S", 5.52),
    ("Q5_K_M", 5.69),
    ("Q6_K", 6.56),
    ("Q8_0", 8.5),
    ("BF16", 16.0),
    ("F16", 16.0),
    ("F32", 32.0),
];

/// Group a repo's GGUF files by quant, biggest first.
///
/// Shards of one quant merge into one entry. mmproj files and non-GGUF files are left out
/// of the groups, so they do not count toward quant sizes. Use [`mmproj_files`] for those.
pub fn group_by_quant(files: &[CatalogFile]) -> Vec<QuantGroup> {
    let mut groups: Vec<QuantGroup> = Vec::new();
    for file in files.iter().filter(|f| !f.mmproj && is_gguf(&f.path)) {
        let quant = quant_of(&file.path);
        match groups.iter_mut().find(|g| g.quant == quant) {
            Some(group) => {
                group.files.push(file.clone());
                group.total_bytes += file.size_bytes;
                group.shards += 1;
            }
            None => groups.push(QuantGroup {
                bits_per_weight: bits_for(&quant),
                quant,
                files: vec![file.clone()],
                total_bytes: file.size_bytes,
                shards: 1,
            }),
        }
    }
    groups.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes).then(a.quant.cmp(&b.quant)));
    groups
}

/// The mmproj (vision projector) files of a repo, kept apart from the quant groups.
pub fn mmproj_files(files: &[CatalogFile]) -> Vec<CatalogFile> {
    files.iter().filter(|f| f.mmproj).cloned().collect()
}

/// Whether a model fits in memory, using the registry rule (min_free_ram_factor 1.0).
///
/// - `Fits` when free RAM is at least the GGUF size.
/// - `Tight` when the size is up to 10 percent over free RAM.
/// - `TooBig` otherwise. A size of 0 means unknown.
/// - `free_vram` is reserved for the "GPU offload possible" hint. This rule does not use it yet.
///
/// Temporary: hfd-core's `fit_for` (80 percent of RAM plus VRAM) replaces this once hf-dl
/// aligns its rule with the registry rule. Keep one rule. Until then, do not switch.
pub fn fit_for(total_bytes: u64, free_ram: u64, free_vram: u64) -> Fit {
    // Reserved for the GPU offload hint; kept in the signature on purpose.
    let _ = free_vram;
    if total_bytes == 0 {
        return Fit::Unknown;
    }
    if total_bytes <= free_ram {
        Fit::Fits
    } else if u128::from(total_bytes) * 100 <= u128::from(free_ram) * 110 {
        Fit::Tight
    } else {
        Fit::TooBig
    }
}

/// Read `(index, total)` from a split file name like `model-00002-of-00005.gguf`.
/// Returns None for a name that is not a shard.
pub fn parse_shard(path: &str) -> Option<(u32, u32)> {
    let name = path.rsplit('/').next()?;
    let stem = name.strip_suffix(".gguf")?;
    let (head, total) = stem.rsplit_once("-of-")?;
    let (_, index) = head.rsplit_once('-')?;
    let index = parse_digits(index)?;
    let total = parse_digits(total)?;
    if index == 0 || total == 0 || index > total {
        return None;
    }
    Some((index, total))
}

/// True when the file name contains `mmproj` (any case).
pub fn mmproj(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .unwrap_or(path)
        .to_lowercase()
        .contains("mmproj")
}

fn is_gguf(path: &str) -> bool {
    path.to_lowercase().ends_with(".gguf")
}

fn parse_digits(s: &str) -> Option<u32> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// Quant name from a file path: the last `-` or `.` token that looks like a quant.
/// Shard suffixes are removed first. Falls back to `unknown`.
fn quant_of(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    let stem = name.strip_suffix(".gguf").unwrap_or(name);
    let stem = match stem.rsplit_once("-of-") {
        Some((head, _)) => match head.rsplit_once('-') {
            Some((rest, idx)) if parse_digits(idx).is_some() => rest,
            _ => stem,
        },
        None => stem,
    };
    stem.split(['-', '.'])
        .filter(|tok| looks_like_quant(tok))
        .last()
        .unwrap_or("unknown")
        .to_uppercase()
}

fn looks_like_quant(tok: &str) -> bool {
    let up = tok.to_uppercase();
    let known = ["Q", "IQ", "BF16", "F16", "F32", "MXFP"]
        .iter()
        .any(|p| up.starts_with(p) && up.len() > p.len());
    known && up.chars().any(|c| c.is_ascii_digit())
}

fn bits_for(quant: &str) -> Option<f32> {
    QUANT_BITS
        .iter()
        .find(|(name, _)| *name == quant)
        .map(|(_, bits)| *bits)
}

/// Settings for a download into `dest_dir`, mapped to hfd-core's options.
///
/// `dest_dir` comes from the settings, never a fixed path. `download_log` maps to
/// `DownloadOptions::download_log`. The cache dir is only used when `local_dir` is unset.
#[cfg(feature = "hf-catalog")]
pub fn download_options(config: &DownloadConfig, dest_dir: &std::path::Path) -> hfd_core::DownloadOptions {
    hfd_core::DownloadOptions {
        cache_dir: hfd_core::Config::from_env().cache_dir,
        local_dir: Some(dest_dir.to_path_buf()),
        download_log: config.download_log.clone(),
        ..Default::default()
    }
}

/// Hub adapter over hfd-core. Feature `hf-catalog`.
///
/// The trait is sync, so each call blocks the calling thread on a small Tokio runtime.
/// Do not call it from inside an async task; use `spawn_blocking`.
#[cfg(feature = "hf-catalog")]
pub struct HfdCatalog {
    hub: hfd_core::HubClient,
    rt: tokio::runtime::Runtime,
}

#[cfg(feature = "hf-catalog")]
fn hf_err(error: hfd_core::Error) -> CatalogError {
    CatalogError::Message(error.to_string())
}

#[cfg(feature = "hf-catalog")]
impl HfdCatalog {
    /// Build from the environment (`HF_HOME`, `HF_ENDPOINT`, token from the usual places).
    pub fn from_env() -> Result<Self, CatalogError> {
        let config = hfd_core::Config::from_env();
        let token = hfd_core::auth::resolve_token(None, &config);
        let hub = hfd_core::HubClient::new(config, token).map_err(hf_err)?;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| CatalogError::Message(error.to_string()))?;
        Ok(Self { hub, rt })
    }

    /// Quant groups of one model repo, biggest first, with shards merged.
    ///
    /// Uses hfd-core's `group_quants`. mmproj files are left out; use [`mmproj_files`].
    pub fn quant_groups(&self, repo: &str) -> Result<Vec<QuantGroup>, CatalogError> {
        let entries = self.tree(repo)?;
        let quants = hfd_core::group_quants(&entries);
        let size_for = |path: &str| -> u64 {
            entries
                .iter()
                .find(|e| e.path == path)
                .and_then(|e| e.size)
                .unwrap_or(0)
        };
        let mut groups: Vec<QuantGroup> = quants
            .options
            .iter()
            .map(|opt| QuantGroup {
                quant: opt.quant.unwrap_or("unknown").to_string(),
                bits_per_weight: opt.bits_per_weight,
                files: opt
                    .files
                    .iter()
                    .map(|p| CatalogFile::new(p, size_for(p)))
                    .collect(),
                total_bytes: opt.size,
                shards: opt.files.len() as u32,
            })
            .collect();
        groups.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes).then(a.quant.cmp(&b.quant)));
        Ok(groups)
    }

    fn tree(&self, repo: &str) -> Result<Vec<hfd_core::TreeEntry>, CatalogError> {
        let repo_ref = hfd_core::RepoRef::new(hfd_core::RepoType::Model, repo);
        self.rt
            .block_on(self.hub.list_tree(&repo_ref, "main"))
            .map_err(hf_err)
    }
}

#[cfg(feature = "hf-catalog")]
impl CatalogSource for HfdCatalog {
    fn search(&self, query: &str, gguf_only: bool) -> Result<Vec<RepoHit>, CatalogError> {
        let q = hfd_core::SearchQuery {
            text: query.to_string(),
            gguf_only,
            sort: hfd_core::SearchSort::Downloads,
            limit: 20,
        };
        let hits = self
            .rt
            .block_on(self.hub.search_models(&q))
            .map_err(hf_err)?;
        Ok(hits
            .into_iter()
            .map(|r| {
                let name = r.id.rsplit('/').next().unwrap_or(&r.id).to_string();
                RepoHit {
                    has_gguf: r.tags.iter().any(|t| t.eq_ignore_ascii_case("gguf")),
                    downloads: r.downloads,
                    name,
                    repo: r.id,
                }
            })
            .collect())
    }

    fn files(&self, repo: &str) -> Result<Vec<CatalogFile>, CatalogError> {
        Ok(self
            .tree(repo)?
            .iter()
            .filter(|e| e.kind == "file")
            .map(|e| CatalogFile::new(&e.path, e.size.unwrap_or(0)))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, size: u64) -> CatalogFile {
        CatalogFile::new(path, size)
    }

    #[test]
    fn group_by_quant_merges_three_shards_into_one_entry() {
        let files = vec![
            file("model-Q4_K_M-00001-of-00003.gguf", 1000),
            file("model-Q4_K_M-00002-of-00003.gguf", 1000),
            file("model-Q4_K_M-00003-of-00003.gguf", 1000),
            file("model-Q8_0.gguf", 2500),
        ];
        let groups = group_by_quant(&files);
        assert_eq!(groups.len(), 2);
        // Biggest first: Q4_K_M (3000 across shards) before Q8_0 (2500).
        assert_eq!(groups[0].quant, "Q4_K_M");
        assert_eq!(groups[0].total_bytes, 3000);
        assert_eq!(groups[0].shards, 3);
        assert_eq!(groups[0].files.len(), 3);
        assert_eq!(groups[0].bits_per_weight, Some(4.85));
        assert_eq!(groups[1].quant, "Q8_0");
        assert_eq!(groups[1].total_bytes, 2500);
        assert_eq!(groups[1].shards, 1);
    }

    #[test]
    fn mmproj_is_excluded_from_quant_sizes() {
        let files = vec![
            file("model-Q4_K_M.gguf", 1000),
            file("mmproj-model-f16.gguf", 500),
        ];
        let groups = group_by_quant(&files);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].total_bytes, 1000);
        let mm = mmproj_files(&files);
        assert_eq!(mm.len(), 1);
        assert_eq!(mm[0].path, "mmproj-model-f16.gguf");
    }

    #[test]
    fn groups_are_sorted_by_size_then_name() {
        let files = vec![
            file("a-Q2_K.gguf", 10),
            file("a-Q6_K.gguf", 50),
            file("a-Q3_K_M.gguf", 10),
        ];
        let names: Vec<String> = group_by_quant(&files).into_iter().map(|g| g.quant).collect();
        assert_eq!(names, vec!["Q6_K", "Q2_K", "Q3_K_M"]);
    }

    #[test]
    fn fit_for_boundaries() {
        // Free RAM 1000. VRAM is ignored by this rule.
        let ram = 1000;
        assert_eq!(fit_for(1000, ram, 0), Fit::Fits); // exact free RAM
        assert_eq!(fit_for(999, ram, 0), Fit::Fits);
        assert_eq!(fit_for(1001, ram, 0), Fit::Tight);
        assert_eq!(fit_for(1100, ram, 0), Fit::Tight); // 10 percent over
        assert_eq!(fit_for(1101, ram, 0), Fit::TooBig); // just above 10 percent
        assert_eq!(fit_for(0, ram, 0), Fit::Unknown);
        assert_eq!(fit_for(100, ram, 9999), Fit::Fits); // VRAM does not change the result
    }

    #[test]
    fn parse_shard_reads_split_names() {
        assert_eq!(parse_shard("model-00002-of-00005.gguf"), Some((2, 5)));
        assert_eq!(parse_shard("Q4/model-00001-of-00002.gguf"), Some((1, 2)));
        assert_eq!(parse_shard("model-Q4_K_M.gguf"), None);
        assert_eq!(parse_shard("model-00006-of-00005.gguf"), None);
        assert_eq!(parse_shard("model-00001-of-00005.bin"), None);
    }

    #[test]
    fn mmproj_detects_name() {
        assert!(mmproj("gemma-mmproj-f16.gguf"));
        assert!(mmproj("MMPROJ-model.gguf"));
        assert!(!mmproj("model-Q4_K_M.gguf"));
    }

    fn fake() -> FakeCatalog {
        FakeCatalog {
            repos: vec![
                (
                    RepoHit {
                        repo: "org/Llama-GGUF".into(),
                        name: "Llama-GGUF".into(),
                        downloads: 900,
                        has_gguf: true,
                    },
                    vec![file("llama-Q4_K_M.gguf", 4000)],
                ),
                (
                    RepoHit {
                        repo: "org/Llama-safetensors".into(),
                        name: "Llama-safetensors".into(),
                        downloads: 300,
                        has_gguf: false,
                    },
                    vec![file("model.safetensors", 8000)],
                ),
            ],
        }
    }

    #[test]
    fn fake_catalog_search_filters() {
        let cat = fake();
        let all = cat.search("llama", false).unwrap();
        assert_eq!(all.len(), 2);
        let gguf = cat.search("LLAMA", true).unwrap();
        assert_eq!(gguf.len(), 1);
        assert_eq!(gguf[0].repo, "org/Llama-GGUF");
        assert!(cat.search("nothing-here", false).unwrap().is_empty());
    }

    #[test]
    fn fake_catalog_files() {
        let cat = fake();
        let files = cat.files("org/Llama-GGUF").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].size_bytes, 4000);
        assert!(cat.files("org/missing").is_err());
    }
}
