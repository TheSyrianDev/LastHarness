//! The vector side of search. Vectors live in the `embeddings` table, one per page and model, and are
//! tied to the page version they were made from, so an edited page is re-embedded and a stale vector
//! is never used. The whole set is read into memory for a brute-force scan (spike 4: 70 ms at
//! 100,000 pages) and dropped again when the core goes idle.

use std::sync::Arc;

use crate::embed::{EmbedError, Embedder};
use crate::{excerpt, Error, Memory, Result, State, Trust};

pub(crate) struct VectorCache {
    model: String,
    ids: Vec<String>,
    dim: usize,
    mat: Vec<f32>,
}

fn to_blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}
fn from_blob(b: &[u8]) -> Vec<f32> {
    b.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect()
}

impl From<EmbedError> for Error {
    fn from(e: EmbedError) -> Self {
        Error::Invalid(e.to_string())
    }
}

impl Memory {
    /// Embed pages that have no current vector. Sensitive pages are skipped unless the embedder
    /// runs on this machine. Returns how many pages were embedded.
    pub async fn index_pending<E: Embedder>(&self, e: &E) -> Result<usize> {
        let model = e.model_id().to_string();
        let todo: Vec<(String, String, String)> = self.store.with(|c| {
            let mut q = c.prepare(
                "select p.id, p.title, p.body from pages p left join embeddings e on e.page_id = p.id and e.model = ?1
                 where p.archived_at is null and (e.page_id is null or e.version <> p.version) and (p.privacy = 'normal' or ?2 = 1)",
            )?;
            let rows = q.query_map(lh_store::rusqlite::params![model, (e.trust() == Trust::Local) as i64], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
            rows.collect()
        })?;
        let mut done = 0;
        for batch in todo.chunks(16) {
            let texts: Vec<String> = batch.iter().map(|(_, t, b)| format!("{t}\n{}", excerpt(b, 2000))).collect();
            let vecs = e.embed_documents(&texts).await?;
            for ((id, _, _), v) in batch.iter().zip(vecs) {
                // the version is read when storing, so a page edited meanwhile is picked up next time
                self.store.with(|c| {
                    c.execute(
                        "insert into embeddings(page_id, model, version, vec) select id, ?2, version, ?3 from pages where id = ?1 on conflict(page_id, model) do update set version = excluded.version, vec = excluded.vec",
                        lh_store::rusqlite::params![id, model, to_blob(&v)],
                    )
                    .map(|_| ())
                })?;
                done += 1;
            }
        }
        if done > 0 {
            *self.cache.lock().unwrap() = None;
        }
        Ok(done)
    }

    fn load_cache(&self, model: &str) -> Result<Arc<VectorCache>> {
        if let Some(c) = self.cache.lock().unwrap().as_ref().filter(|c| c.model == model) {
            return Ok(c.clone());
        }
        let rows: Vec<(String, Vec<u8>)> = self.store.with(|c| {
            let mut q = c.prepare("select p.id, e.vec from embeddings e join pages p on p.id = e.page_id where e.model = ?1 and e.version = p.version and p.archived_at is null order by p.id")?;
            let rows = q.query_map([model], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect()
        })?;
        let dim = rows.first().map_or(0, |r| r.1.len() / 4);
        let mut cache = VectorCache { model: model.into(), ids: vec![], dim, mat: Vec::with_capacity(rows.len() * dim) };
        for (id, blob) in rows {
            if blob.len() / 4 == dim {
                cache.ids.push(id);
                cache.mat.extend(from_blob(&blob));
            }
        }
        let c = Arc::new(cache);
        *self.cache.lock().unwrap() = Some(c.clone());
        Ok(c)
    }

    /// Pages closest in meaning to `query`: (page id, cosine similarity), best first. Archived pages
    /// are not included. Pages that have no current vector simply do not appear.
    pub async fn semantic_search<E: Embedder>(&self, e: &E, query: &str, limit: usize) -> Result<Vec<(String, f64)>> {
        let q = e.embed_query(query).await?;
        let c = self.load_cache(e.model_id())?;
        if c.dim == 0 || q.len() != c.dim {
            return Ok(vec![]);
        }
        let mut scored: Vec<(usize, f32)> = c.mat.chunks_exact(c.dim).enumerate().map(|(i, row)| (i, row.iter().zip(&q).map(|(a, b)| a * b).sum())).collect();
        scored.sort_by(|a, b| b.1.total_cmp(&a.1));
        Ok(scored.into_iter().take(limit).map(|(i, s)| (c.ids[i].clone(), f64::from(s))).collect())
    }

    /// Free the in-memory vectors (idle mode).
    pub fn release_vectors(&self) {
        *self.cache.lock().unwrap() = None;
    }

    pub fn vectors_loaded(&self) -> bool {
        self.cache.lock().unwrap().is_some()
    }

    /// Pages that are live but have no usable vector for `model`.
    pub fn unindexed(&self, model: &str) -> Result<usize> {
        Ok(self.list(None, State::Live)?.len() - self.load_cache(model)?.ids.len())
    }
}
