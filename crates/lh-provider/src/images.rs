//! Image generation through the OpenAI-style `/images/generations` endpoint, which OpenRouter-style
//! services and local servers such as stable-diffusion.cpp use. Video has no common endpoint yet and
//! is not covered.

use base64::Engine;
use serde_json::{json, Value};

use crate::client::Error;
use crate::Provider;

#[derive(Clone, Debug, PartialEq)]
pub struct GeneratedImage {
    pub bytes: Vec<u8>,
    /// File extension from the image's own header: `png`, `jpg` or `webp`; `bin` if unknown.
    pub ext: &'static str,
}

pub fn sniff(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "png"
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        "jpg"
    } else if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "webp"
    } else {
        "bin"
    }
}

const MAX_IMAGE: usize = 25 * 1024 * 1024;

impl Provider {
    pub async fn generate_images(&self, model: &str, prompt: &str, size: Option<&str>, n: u32) -> Result<Vec<GeneratedImage>, Error> {
        let mut body = json!({"model": model, "prompt": prompt, "n": n, "response_format": "b64_json"});
        if let Some(s) = size {
            body["size"] = json!(s);
        }
        let v: Value = self.post_json("/images/generations", &body).await?;
        let rows = v["data"].as_array().ok_or_else(|| Error::Format("no `data` in the image reply".into()))?;
        let mut out = Vec::new();
        for r in rows {
            let bytes = if let Some(b64) = r["b64_json"].as_str() {
                base64::engine::general_purpose::STANDARD.decode(b64).map_err(|e| Error::Format(format!("bad image data: {e}")))?
            } else if let Some(url) = r["url"].as_str() {
                self.download(url).await?
            } else {
                return Err(Error::Format("an image had neither data nor an address".into()));
            };
            if bytes.len() > MAX_IMAGE {
                return Err(Error::Format("the image is larger than 25 MB".into()));
            }
            out.push(GeneratedImage { ext: sniff(&bytes), bytes });
        }
        if out.is_empty() {
            return Err(Error::Format("the server returned no images".into()));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nrest-of-file";

    async fn serve(body: String) -> String {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        tokio::spawn(async move {
            loop {
                let (mut s, _) = l.accept().await.unwrap();
                let body = body.clone();
                tokio::spawn(async move {
                    let mut b = vec![0u8; 8192];
                    let n = s.read(&mut b).await.unwrap_or(0);
                    let req = String::from_utf8_lossy(&b[..n]).to_string();
                    let (ct, payload) = if req.starts_with("GET /file.png") { ("image/png", PNG.to_vec()) } else { ("application/json", body.into_bytes()) };
                    let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: {ct}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", payload.len()).as_bytes()).await;
                    let _ = s.write_all(&payload).await;
                    let _ = s.shutdown().await;
                });
            }
        });
        base
    }

    #[test]
    fn sniffs_formats() {
        assert_eq!(sniff(PNG), "png");
        assert_eq!(sniff(&[0xff, 0xd8, 0xff, 0xe0]), "jpg");
        assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), "webp");
        assert_eq!(sniff(b"<html>"), "bin");
    }

    #[tokio::test]
    async fn reads_inline_data_and_downloaded_addresses() {
        let b64 = base64::engine::general_purpose::STANDARD.encode(PNG);
        let base = serve(format!(r#"{{"data":[{{"b64_json":"{b64}"}}]}}"#)).await;
        let imgs = Provider::new(format!("{base}/v1"), None).generate_images("m", "a cat", Some("512x512"), 1).await.unwrap();
        assert_eq!((imgs.len(), imgs[0].ext, imgs[0].bytes.as_slice()), (1, "png", PNG));

        let base = serve(String::new()).await;
        let by_url = serve(format!(r#"{{"data":[{{"url":"{base}/file.png"}}]}}"#)).await;
        let imgs = Provider::new(format!("{by_url}/v1"), None).generate_images("m", "a cat", None, 1).await.unwrap();
        assert_eq!(imgs[0].bytes, PNG);
    }

    #[tokio::test]
    async fn odd_replies_are_clear_errors() {
        for body in [r#"{"data":[]}"#, r#"{"oops":1}"#, r#"{"data":[{"x":1}]}"#, r#"{"data":[{"b64_json":"!!!"}]}"#] {
            let base = serve(body.to_string()).await;
            assert!(Provider::new(format!("{base}/v1"), None).generate_images("m", "p", None, 1).await.is_err(), "{body}");
        }
    }
}
