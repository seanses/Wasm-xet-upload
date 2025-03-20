use deduplication::{Chunk, Chunker, DataAggregator, RawXorbData};
use mdb_shard::{
    file_structs::{
        FileDataSequenceEntry, FileDataSequenceHeader, FileMetadataExt, FileVerificationEntry,
        MDBFileInfo,
    },
    shard_in_memory::MDBInMemoryShard,
};
use merkledb::aggregate_hashes::file_node_hash;
use merklehash::MerkleHash;
use sha256::ShaGenerator;
use std::{
    io::{Read, Seek, SeekFrom},
    sync::Arc,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_file_reader::WebSysFile;
use web_sys::console;

mod sha256;

#[wasm_bindgen]
pub fn add_two_numbers(a: i32, b: i32) -> i32 {
    return a + b;
}

const MAX_XORB_BYTES: usize = 64 * 1024 * 1024;
const MAX_XORB_CHUNKS: usize = 8 * 1024;

#[derive(Default, Debug)]
struct CleanState {
    /// The new data here that hasn't yet been deduplicated.
    new_data: Vec<Chunk>,

    /// The amount of new data we have.
    new_data_size: usize,

    /// The current chunk hashes for this file.
    chunk_hashes: Vec<(MerkleHash, usize)>,

    /// The current file data entries.
    file_info: Vec<FileDataSequenceEntry>,

    /// The list of indices in which the file entry references the current data
    internally_referencing_entries: Vec<usize>,
}

impl CleanState {
    fn cut_new_xorb(&mut self) -> RawXorbData {
        // Cut the new xorb.
        let new_xorb = RawXorbData::from_chunks(&self.new_data[..]);

        let xorb_hash = new_xorb.hash();

        // Go through and replace all the indices in the file sequence entries with
        // the new xorb if referenced.
        for &idx in self.internally_referencing_entries.iter() {
            let fse = &mut self.file_info[idx];
            debug_assert_eq!(fse.cas_hash, MerkleHash::default());

            fse.cas_hash = xorb_hash;
        }

        #[cfg(debug_assertions)]
        {
            // For bookkeeping checks, make sure we have everything.
            for fse in self.file_info.iter() {
                debug_assert_ne!(fse.cas_hash, MerkleHash::default());
            }
        }

        // Clear out the old data.
        self.new_data.clear();
        self.new_data_size = 0;
        self.internally_referencing_entries.clear();

        new_xorb
    }

    fn finalize(
        self,
        file_hash_salt: &[u8; 32],
        metadata_ext: Option<FileMetadataExt>,
    ) -> (MerkleHash, DataAggregator) {
        log_to_browser(format!("computing file hash"));
        let file_hash = file_node_hash(&self.chunk_hashes, &file_hash_salt);
        if file_hash.is_err() {
            log_to_browser(format!("compute file hash failed: {file_hash:?}"));
        }
        let file_hash = file_hash.unwrap();

        let metadata = FileDataSequenceHeader::new(
            file_hash,
            self.file_info.len(),
            true,
            metadata_ext.is_some(),
        );

        let mut chunk_idx = 0;

        log_to_browser(format!(
            "computing file verification entries, for {} entries",
            self.file_info.len()
        ));
        // Create the file verification stamp.
        let verification = self
            .file_info
            .iter()
            .map(|entry| {
                log_to_browser(format!("{entry:?}"));
                let n_chunks = (entry.chunk_index_end - entry.chunk_index_start) as usize;
                log_to_browser(format!("{:?}", self.chunk_hashes));
                let chunk_hashes: Vec<_> = self.chunk_hashes[chunk_idx..chunk_idx + n_chunks]
                    .iter()
                    .map(|(hash, _)| *hash)
                    .collect();
                log_to_browser(format!("{chunk_hashes:?}"));
                let range_hash = range_hash_from_chunks(&chunk_hashes);
                chunk_idx += n_chunks;

                FileVerificationEntry::new(range_hash)
            })
            .collect();

        log_to_browser(format!("computing file verification entries done"));
        let fi = MDBFileInfo {
            metadata,
            segments: self.file_info,
            verification,
            metadata_ext,
        };

        let remaining_data =
            DataAggregator::new(self.new_data, fi, self.internally_referencing_entries);

        (file_hash, remaining_data)
    }
}

#[wasm_bindgen]
// Clean a file and return the MerkleHash of the file
pub fn clean_file(file: web_sys::File) -> String {
    log_to_browser(format!("clean file called"));

    // let mut sha_generator = ShaGenerator::new();
    let mut chunker = Chunker::default();
    let mut clean_state = CleanState::default();
    let mut shard = MDBInMemoryShard::default();

    let file_hash = {
        let mut wf = WebSysFile::new(file);

        loop {
            const READ_BUF_SIZE: usize = 1024 * 1024;
            let mut buf = vec![0u8; READ_BUF_SIZE];
            let read_bytes = wf.read(&mut buf);
            let Ok(read_bytes) = read_bytes else {
                log_to_browser(format!("{read_bytes:?}"));
                return "".to_owned();
            };
            if read_bytes == 0 {
                break;
            }

            log_to_browser(format!("read {read_bytes} bytes"));

            let chunks: Arc<[Chunk]> = Arc::from(chunker.next_block(&buf[0..read_bytes], false));
            // sha_generator.update(chunks.clone());
            process_chunks(&mut clean_state, &chunks, &mut shard);
            log_to_browser(format!("{} chunks processed", chunks.len()));
        }
        if let Some(chunk) = chunker.finish() {
            let chunks: Arc<[Chunk]> = Arc::new([chunk]);
            // sha_generator.update(chunks.clone());
            process_chunks(&mut clean_state, &chunks, &mut shard);
            log_to_browser(format!("{} chunks processed", chunks.len()));
        }

        log_to_browser(format!("building file info"));
        // let sha256 = sha_generator.finalize().expect("failed to gen Sha256");
        let sha256 = MerkleHash::default();
        let metadata_ext = FileMetadataExt::new(sha256);
        log_to_browser(format!("finalize cleaning"));
        let repo_salt = [0u8; 32];
        let (file_hash, remaining_file_data) = clean_state.finalize(&repo_salt, Some(metadata_ext));
        log_to_browser(format!("finalize last xorb"));
        let (final_xorb, new_files) = remaining_file_data.finalize();
        register_new_xorb(final_xorb, &mut shard);
        log_to_browser(format!("registering file info"));
        for fi in new_files {
            let ret = shard.add_file_reconstruction_info(fi);
            if ret.is_err() {
                log_to_browser(format!("add file info failed: {ret:?}"));
            }
        }

        file_hash
    };

    log_to_browser(format!("file hash: {file_hash}"));

    upload_shard(shard);

    file_hash.to_string()
}

fn process_chunks(cs: &mut CleanState, chunks: &[Chunk], shard: &mut MDBInMemoryShard) {
    let mut cur_idx = 0;
    while cur_idx < chunks.len() {
        let n_bytes = chunks[cur_idx].data.len();

        // Do we need to cut a new xorb first?
        if cs.new_data_size + n_bytes > MAX_XORB_BYTES || cs.new_data.len() + 1 > MAX_XORB_CHUNKS {
            let new_xorb = cs.cut_new_xorb();
            register_new_xorb(new_xorb, shard);
        }

        if !cs.file_info.is_empty()
            && cs.file_info.last().unwrap().cas_hash == MerkleHash::default()
            && cs.file_info.last().unwrap().chunk_index_end as usize == cs.new_data.len()
        {
            // This is the next chunk in the CAS block we're building,
            // in which case we can just modify the previous entry.
            let last_entry = cs.file_info.last_mut().unwrap();
            last_entry.unpacked_segment_bytes += n_bytes as u32;
            last_entry.chunk_index_end += 1;
        } else {
            // This block is unrelated to the previous one.
            // This chunk will get the CAS hash updated when the local CAS block
            // is full and registered.
            let file_info_len = cs.file_info.len();
            cs.internally_referencing_entries.push(file_info_len);
            let chunk_idx = cs.new_data.len();

            cs.file_info.push(FileDataSequenceEntry::new(
                MerkleHash::default(),
                n_bytes,
                chunk_idx,
                chunk_idx + 1,
            ));
        }

        let chunk = chunks[cur_idx].clone();
        cs.new_data_size += chunk.data.len();
        cs.new_data.push(chunk);

        // Next round.
        cur_idx += 1;
    }

    cs.chunk_hashes
        .extend(chunks.iter().map(|c| (c.hash, c.data.len())));
}

fn register_new_xorb(xorb: RawXorbData, shard: &mut MDBInMemoryShard) {}

fn upload_shard(shard: MDBInMemoryShard) {}

pub const VERIFICATION_KEY: [u8; 32] = [
    127, 24, 87, 214, 206, 86, 237, 102, 18, 127, 249, 19, 231, 165, 195, 243, 164, 205, 38, 213,
    181, 219, 73, 230, 65, 36, 152, 127, 40, 251, 148, 195,
];

fn range_hash_from_chunks(chunks: &[MerkleHash]) -> MerkleHash {
    log_to_browser(format!("range hash from chunks"));
    let combined: Vec<u8> = chunks
        .iter()
        .flat_map(|hash| hash.as_bytes().to_vec())
        .collect();

    // now apply hmac to hashes and return
    log_to_browser(format!("blake3 range hash"));
    let range_hash = blake3::keyed_hash(&VERIFICATION_KEY, combined.as_slice());

    MerkleHash::from(range_hash.as_bytes())
}

/// Reads one byte from the file at a given offset. Returns the read byte or 0 if the file is empty
/// See also https://github.com/Badel2/wasm-bindgen-file-reader-test
#[wasm_bindgen]
pub fn read_at_offset_sync(file: web_sys::File, offset: u64) -> u8 {
    let log_msg = format!(
        "Rust function read_at_offset_sync sees file \"{}\". Size of file in bytes: {}",
        file.name(),
        file.size()
    );
    log_to_browser(log_msg);

    let mut wf = WebSysFile::new(file);

    loop {
        const READ_BUF_SIZE: usize = 1024 * 1024;
        let mut buf = vec![0u8; READ_BUF_SIZE];
        let read_bytes = wf.read(&mut buf);
        let Ok(read_bytes) = read_bytes else {
            log_to_browser(format!("{read_bytes:?}"));
            return 0;
        };
        if read_bytes == 0 {
            break;
        }

        log_to_browser(format!("read {read_bytes} bytes"));
    }

    0
    // if file.size() == 0.0 {
    //     log_to_browser("Can't get first byte of an empty file".to_string());
    //     return 0;
    // }
    // {
    //     let mut wf = WebSysFile::new(file);

    //     // Now we can seek as if this was a real file
    //     wf.seek(SeekFrom::Start(offset))
    //         .expect("failed to seek to offset");

    //     // Use a 1-byte buffer because we only want to read one byte
    //     let mut buf = [0];

    //     // The Read API works as with real files
    //     wf.read_exact(&mut buf).expect("failed to read bytes");

    //     buf[0]
    // }
}

/// Logs a string to the browser's console
fn log_to_browser(log_msg: String) {
    console::log_1(&log_msg.into());
}
