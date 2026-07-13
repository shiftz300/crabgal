use std::hint::black_box;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crabgal_loader::mount_hexz;

const OPEN_ITERATIONS: usize = 5;
const LOOKUP_ITERATIONS: usize = 200_000;
const DIRECTORY_ITERATIONS: usize = 20_000;
const RANGE_ITERATIONS: usize = 4_000;

fn main() {
    let package = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/streaming-test-project.hxz")
        });
    if !package.is_file() {
        eprintln!("missing benchmark package: {}", package.display());
        std::process::exit(2);
    }

    let open = timed(OPEN_ITERATIONS, || {
        black_box(mount_hexz(&package).unwrap());
    });
    let archive = mount_hexz(&package).unwrap();
    let hit = Path::new("assets/background/bg.webp");
    let miss = Path::new("assets/background/missing.webp");
    let directory = Path::new("assets/background");

    let lookup = timed(LOOKUP_ITERATIONS, || {
        black_box(archive.contains_file(hit));
        black_box(archive.contains_file(miss));
    });
    let directory_lookup = timed(DIRECTORY_ITERATIONS, || {
        black_box(archive.is_directory(directory));
        black_box(archive.read_directory(directory));
    });
    let file = archive.open_file(hit).unwrap();
    let mut buffer = vec![0_u8; 4096];
    let mut raw_offset = 0;
    let range = timed(RANGE_ITERATIONS, || {
        if raw_offset + buffer.len() > file.len() {
            raw_offset = 0;
        }
        let read = file.read_range_into(raw_offset, &mut buffer).unwrap();
        raw_offset += read;
        black_box(read);
    });
    let mut cursor = file.cursor();
    let buffered = timed(RANGE_ITERATIONS, || {
        if cursor.position() + buffer.len() > cursor.len() {
            cursor.seek(SeekFrom::Start(0)).unwrap();
        }
        cursor.read_exact(&mut buffer).unwrap();
        black_box(&buffer);
    });

    println!("Hexz loader benchmark: {}", package.display());
    print_result("open + index", open, OPEN_ITERATIONS);
    print_result("hit + miss lookup", lookup, LOOKUP_ITERATIONS * 2);
    print_result(
        "directory lookup + list",
        directory_lookup,
        DIRECTORY_ITERATIONS * 2,
    );
    print_result("raw 4 KiB range read", range, RANGE_ITERATIONS);
    print_result("buffered 4 KiB read", buffered, RANGE_ITERATIONS);
}

fn timed(iterations: usize, mut operation: impl FnMut()) -> Duration {
    let start = Instant::now();
    for _ in 0..iterations {
        operation();
    }
    start.elapsed()
}

fn print_result(name: &str, duration: Duration, operations: usize) {
    let nanos = duration.as_nanos() as f64 / operations as f64;
    println!(
        "  {name:24} {:>9.3} ms total  {:>10.1} ns/op",
        duration.as_secs_f64() * 1_000.0,
        nanos,
    );
}
