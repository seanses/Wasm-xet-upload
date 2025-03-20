use std::sync::Arc;

use deduplication::Chunk;
use merklehash::MerkleHash;
use sha2::{Digest, Sha256};
use thread::{JoinHandle, Result};
use wasm_thread as thread;

#[derive(Debug)]
pub struct ShaGenerator {
    hasher: Option<JoinHandle<Result<Sha256>>>,
}

impl ShaGenerator {
    pub fn new() -> Self {
        Self { hasher: None }
    }

    /// Update the generator with some bytes.
    pub fn update(&mut self, new_chunks: Arc<[Chunk]>) {
        let current_state = self.hasher.take();

        // The previous task returns the hasher; we consume that and pass it on.
        self.hasher = Some(thread::spawn(move || {
            let mut hasher = match current_state {
                Some(jh) => jh.join()??,
                None => Sha256::default(),
            };

            for chunk in new_chunks.iter() {
                hasher.update(&chunk.data);
            }

            Ok(hasher)
        }));
    }

    // For testing purposes
    pub fn update_with_bytes(&mut self, new_bytes: &[u8]) {
        let new_chunk = Chunk {
            hash: MerkleHash::default(), // not used
            data: Arc::from(Vec::from(new_bytes)),
        };

        self.update(Arc::new([new_chunk]));
    }

    /// Generates a sha256 from the current state of the variant.
    pub fn finalize(mut self) -> Result<MerkleHash> {
        let current_state = self.hasher.take();

        let hasher = match current_state {
            Some(jh) => jh.join()??,
            None => return Ok(MerkleHash::default()),
        };

        let sha256 = hasher.finalize();
        let hex_str = format!("{sha256:x}");
        Ok(MerkleHash::from_hex(&hex_str).expect("Converting sha256 to merklehash."))
    }
}
