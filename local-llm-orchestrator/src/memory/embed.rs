//! Embedding helpers and the built-in offline hashing embedder.

/// Deterministic, model-free text embedder based on feature hashing.
///
/// It hashes word unigrams, word bigrams and character trigrams into a
/// fixed-size signed vector, then L2-normalises it. Quality is well below a
/// neural embedding model, but it captures lexical overlap, needs no model
/// download, is stable across versions (FNV-1a, not std's randomised hasher)
/// and never sends text anywhere.
#[derive(Debug, Clone)]
pub struct HashEmbedder {
    dim: usize,
}

const STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "do", "for", "from", "has", "have",
    "i", "in", "is", "it", "its", "me", "my", "of", "on", "or", "so", "that", "the", "this", "to",
    "was", "we", "what", "with", "you", "your",
];

impl HashEmbedder {
    pub fn new(dim: usize) -> Self {
        assert!(dim > 0);
        Self { dim }
    }

    pub fn model_id(&self) -> String {
        format!("hash-{}", self.dim)
    }

    pub fn embed(&self, text: &str) -> Vec<f32> {
        let mut v = vec![0f32; self.dim];
        let tokens: Vec<String> = tokenize(text)
            .filter(|t| !STOPWORDS.contains(&t.as_str()))
            .collect();

        for t in &tokens {
            self.add(&mut v, b"w:", t.as_bytes(), 1.0);
            let chars: Vec<char> = format!("^{t}$").chars().collect();
            if chars.len() > 3 {
                for w in chars.windows(3) {
                    let tri: String = w.iter().collect();
                    self.add(&mut v, b"c:", tri.as_bytes(), 0.35);
                }
            }
        }
        for pair in tokens.windows(2) {
            let bigram = format!("{} {}", pair[0], pair[1]);
            self.add(&mut v, b"b:", bigram.as_bytes(), 0.6);
        }
        normalize(&mut v);
        v
    }

    fn add(&self, v: &mut [f32], prefix: &[u8], feature: &[u8], weight: f32) {
        let h = fnv1a(prefix, feature);
        let idx = (h % self.dim as u64) as usize;
        let sign = if h >> 63 == 0 { 1.0 } else { -1.0 };
        v[idx] += sign * weight;
    }
}

fn tokenize(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
}

fn fnv1a(prefix: &[u8], data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in prefix.iter().chain(data) {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

pub fn normalize(v: &mut [f32]) {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        v.iter_mut().for_each(|x| *x /= norm);
    }
}

/// Cosine similarity. Returns 0 for mismatched lengths or zero vectors.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0f32, 0f32, 0f32);
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}

pub fn to_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

pub fn from_bytes(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_normalised() {
        let e = HashEmbedder::new(256);
        let a = e.embed("I prefer Rust for systems programming");
        let b = e.embed("I prefer Rust for systems programming");
        assert_eq!(a, b);
        let norm: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn related_text_scores_higher() {
        let e = HashEmbedder::new(512);
        let q = e.embed("which programming language does the user prefer?");
        let related = e.embed("The user prefers the Rust programming language");
        let unrelated = e.embed("Dinner reservation tomorrow at seven near the harbour");
        assert!(cosine(&q, &related) > cosine(&q, &unrelated) + 0.1);
    }

    #[test]
    fn handles_inflection_via_trigrams() {
        let e = HashEmbedder::new(512);
        let a = e.embed("deploying containers");
        let b = e.embed("deployment of a container");
        let c = e.embed("banana smoothie");
        assert!(cosine(&a, &b) > cosine(&a, &c));
    }

    #[test]
    fn empty_text_is_zero_vector() {
        let e = HashEmbedder::new(64);
        let v = e.embed("  !!  ");
        assert!(v.iter().all(|x| *x == 0.0));
        assert_eq!(cosine(&v, &e.embed("hello")), 0.0);
    }

    #[test]
    fn byte_roundtrip() {
        let v = vec![0.5f32, -1.25, 3.0];
        assert_eq!(from_bytes(&to_bytes(&v)), v);
    }
}
