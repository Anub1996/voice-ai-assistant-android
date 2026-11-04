//! Internet-facing HTTP (Hugging Face, GitHub releases): TLS with the OS
//! certificate store (plus an optional extra CA for corporate proxies),
//! system/env proxy support, and a resumable, verified file downloader.

use std::path::Path;
use std::sync::Once;
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use futures::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::jobs::Job;

static TLS: Once = Once::new();

/// Install the process-wide rustls crypto provider (ring). Idempotent.
pub fn init_tls() {
    TLS.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

pub const USER_AGENT: &str = concat!("llmo/", env!("CARGO_PKG_VERSION"));

/// Client for internet downloads. Unlike the runtime client it honours
/// system and `HTTPS_PROXY` settings.
pub fn internet_client(ca_bundle: Option<&str>) -> Result<reqwest::Client> {
    init_tls();
    let mut b = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(60))
        .redirect(reqwest::redirect::Policy::limited(10));
    if let Some(path) = ca_bundle {
        let pem = std::fs::read(crate::config::expand_tilde(path))
            .with_context(|| format!("reading CA bundle {path}"))?;
        for cert in reqwest::Certificate::from_pem_bundle(&pem).context("parsing CA bundle")? {
            b = b.add_root_certificate(cert);
        }
    }
    Ok(b.build()?)
}

pub fn free_space(dir: &Path) -> Option<u64> {
    let mut d = dir;
    loop {
        if d.exists() {
            return fs4::available_space(d).ok();
        }
        d = d.parent()?;
    }
}

/// Download `url` to `dest` with resume (HTTP Range on `dest.part`),
/// retries with backoff, progress reporting, cancellation, optional SHA-256
/// verification, and an atomic rename on success.
pub async fn download(
    http: &reqwest::Client,
    url: &str,
    bearer: Option<&str>,
    dest: &Path,
    expected_size: Option<u64>,
    expected_sha256: Option<&str>,
    job: &Job,
) -> Result<()> {
    let dir = dest.parent().context("destination has no parent")?;
    tokio::fs::create_dir_all(dir)
        .await
        .with_context(|| format!("creating {}", dir.display()))?;
    let part = dest.with_extension(format!(
        "{}.part",
        dest.extension().and_then(|e| e.to_str()).unwrap_or("dl")
    ));

    if let (Some(size), Some(free)) = (expected_size, free_space(dir)) {
        let have = tokio::fs::metadata(&part)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        let need = size.saturating_sub(have);
        ensure!(
            free > need + 512 * 1024 * 1024,
            "not enough disk space in {}: need {:.1} GB, {:.1} GB free",
            dir.display(),
            need as f64 / 1e9,
            free as f64 / 1e9
        );
    }

    // Bytes of this file already credited to the job's progress. Callers
    // add the expected size to `job.total` up front; unknown sizes are
    // added from Content-Length once.
    let mut acct = Acct {
        counted: 0,
        total_added: expected_size.is_some(),
    };
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        match download_once(http, url, bearer, &part, expected_size, job, &mut acct).await {
            Ok(()) => break,
            Err(e) if job.cancelled() => return Err(e),
            Err(e) if attempt < 5 && is_retryable(&e) => {
                let wait = Duration::from_secs(2u64.pow(attempt));
                tracing::warn!(%url, attempt, error = %format!("{e:#}"), "download failed; retrying");
                job.stage(format!("network error, retrying in {}s", wait.as_secs()));
                tokio::time::sleep(wait).await;
            }
            Err(e) => return Err(e),
        }
    }

    if let Some(want) = expected_sha256 {
        job.stage("verifying checksum");
        let got = sha256_file(&part).await?;
        if !got.eq_ignore_ascii_case(want) {
            let _ = tokio::fs::remove_file(&part).await;
            bail!(
                "checksum mismatch for {}: expected {want}, got {got}; the partial file was deleted",
                dest.display()
            );
        }
    }
    tokio::fs::rename(&part, dest)
        .await
        .with_context(|| format!("moving into {}", dest.display()))?;
    Ok(())
}

#[derive(Debug)]
struct HttpStatus(u16, String);
impl std::fmt::Display for HttpStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HTTP {}: {}", self.0, self.1)
    }
}
impl std::error::Error for HttpStatus {}

fn is_retryable(e: &anyhow::Error) -> bool {
    if let Some(s) = e.downcast_ref::<HttpStatus>() {
        return s.0 == 429 || s.0 >= 500;
    }
    e.downcast_ref::<reqwest::Error>().is_some() || e.downcast_ref::<std::io::Error>().is_some()
}

struct Acct {
    counted: u64,
    total_added: bool,
}

impl Acct {
    /// Make the job's progress reflect exactly `on_disk` bytes of this file.
    fn set(&mut self, job: &Job, on_disk: u64) {
        use std::sync::atomic::Ordering::Relaxed;
        if on_disk >= self.counted {
            job.progress(on_disk - self.counted);
        } else {
            job.done.fetch_sub(self.counted - on_disk, Relaxed);
        }
        self.counted = on_disk;
    }
}

async fn download_once(
    http: &reqwest::Client,
    url: &str,
    bearer: Option<&str>,
    part: &Path,
    expected_size: Option<u64>,
    job: &Job,
    acct: &mut Acct,
) -> Result<()> {
    let mut have = tokio::fs::metadata(part)
        .await
        .map(|m| m.len())
        .unwrap_or(0);
    if let Some(size) = expected_size {
        if have > size {
            tokio::fs::remove_file(part).await?;
            have = 0;
        }
        if have == size {
            acct.set(job, have);
            return Ok(());
        }
    }
    acct.set(job, have);
    let mut req = http.get(url);
    if let Some(t) = bearer {
        req = req.bearer_auth(t);
    }
    if have > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
    }
    let resp = req.send().await?;
    let status = resp.status();
    if status.as_u16() == 416 && have > 0 && expected_size.is_none() {
        return Ok(()); // server says we already have everything
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        let hint = match status.as_u16() {
            401 | 403 => {
                " (gated or private: accept the license on huggingface.co and set an HF token)"
            }
            404 => " (not found)",
            _ => "",
        };
        return Err(HttpStatus(
            status.as_u16(),
            format!("{}{hint}", body.chars().take(200).collect::<String>()),
        )
        .into());
    }
    let resumed = status.as_u16() == 206;
    let mut file = if resumed {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(part)
            .await?
    } else {
        // Server ignored the range (or fresh start): begin from zero.
        acct.set(job, 0);
        tokio::fs::File::create(part).await?
    };
    let mut written = if resumed { have } else { 0 };
    if !acct.total_added
        && let Some(len) = resp.content_length()
    {
        job.add_total(len + written);
        acct.total_added = true;
    }
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        if job.cancelled() {
            file.flush().await?;
            bail!("cancelled");
        }
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        written += chunk.len() as u64;
        acct.set(job, written);
    }
    file.flush().await?;
    file.sync_all().await?;
    if let Some(size) = expected_size
        && written != size
    {
        // Connection closed early: retryable, and the next attempt resumes.
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            format!("incomplete download: got {written} of {size} bytes"),
        )
        .into());
    }
    Ok(())
}

pub async fn sha256_file(path: &Path) -> Result<String> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || -> Result<String> {
        use std::io::Read;
        let mut f = std::fs::File::open(&path)?;
        let mut h = Sha256::new();
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
        }
        Ok(crate::auth::to_hex(&h.finalize()))
    })
    .await?
}
