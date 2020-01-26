// directives.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

use std::io::Read;

/**************************/
/***** Main Directive *****/
/**************************/

/// Minimum required size of tar archive/calculated chunks
const MIN_ARCHIVE_SIZE: u64 = 1024;
/// Minimum user-provided chunks number
const MIN_NUM_CHUNKS: u32 = 2;

#[derive(Debug)]
pub struct TarsplitDirectiveArgs {
    pub chunk_size: Option<u64>,
    pub num_chunks: Option<u32>,
    pub prefix: String,
    pub source: String,
    pub target: String,
}

impl<'a> From<&clap::ArgMatches<'a>> for TarsplitDirectiveArgs {
    fn from(matches: &clap::ArgMatches<'a>) -> TarsplitDirectiveArgs {
        // Parse chunk size argument
        let chunk_size = match matches.value_of("CHUNK_SIZE") {
            None => None,
            Some(chunk_size) => {
                let chunk_size = chunk_size.parse::<u64>().unwrap();
                if chunk_size < MIN_ARCHIVE_SIZE {
                    panic!("Chunk size must be at least {}", MIN_ARCHIVE_SIZE);
                }
                Some(chunk_size)
            },
        };
        // Parse number of chunks argument
        let num_chunks = match matches.value_of("NUM_CHUNKS") {
            None => {
                if chunk_size == None {
                    panic!("Must provide either chunk size or number of chunks");
                } else { None }
            },
            Some(num_chunks) => {
                let num_chunks = num_chunks.parse::<u32>().unwrap();
                if num_chunks == MIN_NUM_CHUNKS {
                    panic!("Number of chunks must be greater than {}", MIN_NUM_CHUNKS);
                }
                Some(num_chunks)
            },
        };
        // Parse prefix argument
        let prefix = match matches.value_of("PREFIX") {
            None => String::from("split"),
            Some(prefix) => String::from(prefix),
        };
        // Parse source argument
        let source = matches.value_of("SOURCE").unwrap().to_string();
        // Parse target path argument
        let target  = matches.value_of("TARGET").unwrap().to_string();

        TarsplitDirectiveArgs {
            chunk_size,
            num_chunks,
            prefix,
            source,
            target,
        }
    }
}

#[doc(hidden)]
fn gen_chunk_size(chunk_size: &Option<u64>, num_chunks: &Option<u32>, source_size: &u64) -> u64 {
    // Calculate output chunks (maximum) size
    match chunk_size {
        // If num_chunks specified, calculate maximum size of each chunk
        // as source_size / num_chunks
        None => {
            let max_chunk_size = ((*source_size as f64) / (num_chunks.unwrap() as f64)).round() as u64;
            // Panic if chunk size is zero
            if max_chunk_size < MIN_ARCHIVE_SIZE {
                panic!("Calculated chunk size must be at least {} bytes, try providing \
                       a lower number of chunks (<{})", MIN_ARCHIVE_SIZE, num_chunks.unwrap());
            }
            max_chunk_size
        },
        // Otherwise use the user-provided chunk size
        Some(max_chunk_size) => {
            // Panic if chunk_size greater than size of source archive
            if max_chunk_size >= source_size {
                panic!(
                    "Chunk size must be less than source archive size ({} >= {})",
                    max_chunk_size,
                    source_size
                );
            }
            *max_chunk_size
        },
    }
}

#[doc(hidden)]
fn gen_chunk_filename(prefix: &str, filename_base: &str, chunk_count: u32) -> String {
    format!("{}_{}_{}.tar", prefix, filename_base, chunk_count)
}

#[doc(hidden)]
fn gen_chunk_archive(
    target: &std::path::Path,
    prefix: &str,
    filename_base: &str,
    chunk_count: u32
) -> tar::Builder<std::io::BufWriter<std::fs::File>> {
    let filepath = gen_chunk_filename(prefix, filename_base, chunk_count);
    let filepath = target.join(&filepath);
    tar::Builder::new(
        std::io::BufWriter::new(
            std::fs::File::create(filepath.as_path()).unwrap()
        )
    )
}

pub fn tarsplit(args: TarsplitDirectiveArgs) {
    // Ensure source is file and exists
    let source = std::path::Path::new(&args.source);
    if !source.is_file() {
        panic!("Source must point to an existing archive");
    }

    // Ensure target is existing directory
    let target = std::path::Path::new(&args.target);
    if !target.is_dir() {
        panic!("Target must point to an existing directory");
    }

    // Read size of source archive
    let source_size = source.metadata().unwrap().len();
    // Panic if source is less than 
    if source_size < MIN_ARCHIVE_SIZE { panic!("::: ERROR: Source archive is less than {} bytes", MIN_ARCHIVE_SIZE); }
    println!("::: INFO: Source archive is {} bytes", source_size);

    // Calculate output chunks (maximum) size
    let chunk_maximum_size = gen_chunk_size(&args.chunk_size, &args.num_chunks, &source_size);
    println!("::: INFO: Maximum chunk size will be {} bytes", chunk_maximum_size);

    // Generate output archives base filename from source archive file stem
    let chunk_filename_base = source.file_stem().unwrap().to_str().unwrap();

    // Read source as TAR archive
    let source = std::fs::File::open(source).unwrap();
    let mut source = tar::Archive::new(source);

    // Initialize loop variable state
    let mut current_chunk_size: u64 = 0;
    let mut chunk_count: u32 = 0;
    let mut archive_chunk = gen_chunk_archive(
        &target,
        &args.prefix,
        chunk_filename_base,
        chunk_count
    );

    // For each entry in the source archive
    for entry in source.entries().unwrap() {
        // Unwrap archive entry
        let mut entry = entry.unwrap();
        // Copy header and check entry size
        let mut entry_header = entry.header().clone();
        let entry_size = entry_header.entry_size().unwrap();

        // If adding entry would make chunk large than maximum chunk size
        // TODO: If entry_size itself is larger than chunk_maximum_size,
        //       write entry alone to separate tar file.
        if current_chunk_size + entry_size > chunk_maximum_size {
            println!("::: INFO: Reached chunk boundary, writing chunk {}", chunk_count);
            // Flush current chunk to disk
            archive_chunk.finish().unwrap();
            // Increment chunk count
            chunk_count = chunk_count + 1;
            // Generate new archive
            archive_chunk = gen_chunk_archive(
                &target,
                &args.prefix,
                chunk_filename_base,
                chunk_count
            );
            // Reset current chunk size
            current_chunk_size = 0;
        }

        // Extract entry path
        let entry_path  = entry.path().unwrap().to_path_buf();
        // Add entry to archive chunk
        archive_chunk.append_data(
            &mut entry_header,
            entry_path,
            entry.by_ref()
        ).unwrap();

        // Increment current chunk size by size of header plus entry size
        // (each aligned to 512 bytes).
        current_chunk_size = current_chunk_size + 512 + (if entry_size > 512 {entry_size} else {512});
    }

    // Flush final chunk to disk
    println!("::: INFO: Writing final chunk");
    archive_chunk.finish().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use galvanic_assert::assert_that;
    use galvanic_test::test_suite;

    test_suite! {
        name tarsplit_test_suite;

// Panics:
//  1) Calculated chunk size rounds to less than MIN_ARCHIVE_SIZE
//  2) User-provided chunk size greater than source archive size
        fixture fixture_gen_chunk_size(
            expected: u64,
            should_panic: bool,
            chunk_size: Option<u64>,
            num_chunks: Option<u32>,
            source_size: u64
        ) -> () {
            params {
                vec![
                    (0, true, None, Some(3), 1024),
                    (0, true, Some(2048), None, 1024),
                    (15044193919, false, Some(15044193919), None, 16634133390),
                    (37629135304, false, Some(37629135304), None, 46228510722),
                    (24684425755, false, Some(24684425755), None, 29434577089),
                    (17016020886, false, Some(17016020886), None, 51155793224),
                    (3538808281, false, None, Some(10), 35388082811),
                    (2302398318, false, Some(2302398318), None, 3528792383),
                    (7119293162, false, Some(7119293162), None, 9264872871),
                    (7199108631, false, None, Some(2), 14398217261),
                    (515418526, false, None, Some(12), 6185022310),
                    (122409328, false, Some(122409328), None, 4963699455),
                    (9151011641, false, None, Some(5), 45755058207),
                    (13559357808, false, Some(13559357808), None, 24230229168),
                    (2966266890, false, None, Some(10), 29662668896),
                    (32775332, false, None, Some(17), 557180652),
                    (8565004352, false, Some(8565004352), None, 22489160338),
                    (26025676716, false, Some(26025676716), None, 37848167206),
                    (16166691602, false, Some(16166691602), None, 18969499869),
                    (1233997522, false, None, Some(18), 22211955399),
                    (7216833632, false, Some(7216833632), None, 26792632238),
                    (15195086225, false, Some(15195086225), None, 47609272141),
                ].into_iter()
            }
            setup(&mut self) { () }
        }

        test test_gen_chunk_size(fixture_gen_chunk_size) {
            if *fixture_gen_chunk_size.params.should_panic {
                assert_that!(crate::directives::gen_chunk_size(
                    &fixture_gen_chunk_size.params.chunk_size,
                    &fixture_gen_chunk_size.params.num_chunks,
                    &fixture_gen_chunk_size.params.source_size,
                ), panics);
            } else {
                assert_eq!(*fixture_gen_chunk_size.params.expected, crate::directives::gen_chunk_size(
                    &fixture_gen_chunk_size.params.chunk_size,
                    &fixture_gen_chunk_size.params.num_chunks,
                    &fixture_gen_chunk_size.params.source_size,
                ));
            }
        }
    }
}
