use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};

use anyhow::Result;
use lazy_static::lazy_static;
use sha1::{Digest, Sha1};
use git2::{Oid, ObjectType};

type BlobId = Vec<u8>;

fn git_hash(input: &[u8]) -> Result<BlobId> {
    let result = Oid::hash_object(ObjectType::Blob, input)?;
    Ok(result.as_bytes().to_vec())
}

fn rust_sha1_hash(input: &[u8]) -> Result<BlobId> {
    let mut h = Sha1::new();
    h.update(b"blob ");
    let len_str: String = input.len().to_string();
    h.update(len_str);
    h.update(b"\0");
    h.update(input);
    let result = h.finalize();
    Ok(result.to_vec())
}

lazy_static! {
    static ref BLOB_HASHER: Sha1 = {
        let mut h = Sha1::new();
        h.update(b"blob ");
        h
    };
}

fn rust_sha1_hash_2(input: &[u8]) -> Result<BlobId> {
    let mut h = BLOB_HASHER.clone();
    let len_str: String = input.len().to_string();
    h.update(len_str);
    h.update(b"\0");
    h.update(input);
    let result = h.finalize();
    Ok(result.to_vec())
}

fn rust_sha1_hash_3(input: &[u8]) -> Result<BlobId> {
    use std::io::Write;

    let mut h = BLOB_HASHER.clone();
    write!(&mut h, "{}\0", input.len())?;
    h.update(input);
    let result = h.finalize();
    Ok(result.to_vec())
}

fn rust_sha1_hash_4(input: &[u8]) -> Result<BlobId> {
    use std::io::Write;

    let mut h = Sha1::new();
    write!(&mut h, "blob {}\0", input.len())?;
    h.update(input);
    let result = h.finalize();
    Ok(result.to_vec())
}

fn openssl_hash(input: &[u8]) -> Result<BlobId> {
    use openssl::sha::sha1;
    Ok(sha1(input).to_vec())
}

pub fn blob_hash_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Blob ID");

    let mut len: usize = 1024;
    loop {
        if len > 32 * 1024 * 1024 {
            break;
        }

        group.throughput(Throughput::Bytes(len as u64));

        let input: &[u8] = &vec![42; len];

        group.bench_with_input(BenchmarkId::new("git2", len), &input,
            |b, input| b.iter(|| git_hash(*input)));

        /*
        group.bench_with_input(BenchmarkId::new("rust_sha1 1", len), &input,
            |b, input| b.iter(|| rust_sha1_hash(*input)));

        group.bench_with_input(BenchmarkId::new("rust_sha1 2", len), &input,
            |b, input| b.iter(|| rust_sha1_hash_2(*input)));

        group.bench_with_input(BenchmarkId::new("rust_sha1 3", len), &input,
            |b, input| b.iter(|| rust_sha1_hash_3(*input)));
        */

        group.bench_with_input(BenchmarkId::new("rust_sha1 4", len), &input,
            |b, input| b.iter(|| rust_sha1_hash_4(*input)));

        group.bench_with_input(BenchmarkId::new("openssl", len), &input,
            |b, input| b.iter(|| openssl_hash(*input)));

        len *= 4;
    }
    group.finish();
}

criterion_group!(microbenchmarks, blob_hash_benchmark);
criterion_main!(microbenchmarks);
