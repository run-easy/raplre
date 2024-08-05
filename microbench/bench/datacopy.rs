use std::{alloc::Layout, env::var};

use criterion::{black_box, criterion_group, criterion_main, Criterion};

const BATCH_SIZE: usize = 32;
const ELEM_SIZE: usize = 512;

pub fn b(c: &mut Criterion) {
    c.bench_function("seperate_copy", |b| {
        let src = unsafe {
            std::alloc::alloc(Layout::from_size_align(BATCH_SIZE * ELEM_SIZE, 64).unwrap())
        };
        let dst = unsafe {
            std::alloc::alloc(Layout::from_size_align(BATCH_SIZE * ELEM_SIZE, 64).unwrap())
        };
        b.iter(|| {
            for i in 0..BATCH_SIZE {
                unsafe {
                    std::ptr::copy(src.add(i * ELEM_SIZE), dst.add(i * ELEM_SIZE), ELEM_SIZE);
                }
            }
        });
    });

    c.bench_function("batch_copy", |b| {
        let src = unsafe {
            std::alloc::alloc(Layout::from_size_align(BATCH_SIZE * ELEM_SIZE, 64).unwrap())
        };
        let dst = unsafe {
            std::alloc::alloc(Layout::from_size_align(BATCH_SIZE * ELEM_SIZE, 64).unwrap())
        };
        b.iter(|| {
            unsafe { std::ptr::copy(src, dst, BATCH_SIZE * ELEM_SIZE) };
        });
    });

    c.bench_function("scatter_seperate_copy", |b| {
        let mut batches = vec![];
        for i in 0..BATCH_SIZE {
            let src = unsafe { std::alloc::alloc(Layout::from_size_align(2048, 64).unwrap()) };
            let dst = unsafe { std::alloc::alloc(Layout::from_size_align(2048, 64).unwrap()) };
            batches.push((src, dst));
        }
        b.iter(|| {
            for i in 0..BATCH_SIZE {
                unsafe {
                    std::ptr::copy(batches[i].0, batches[i].1, ELEM_SIZE);
                }
            }
        });
    });
}

criterion_group!(benches, b);
criterion_main!(benches);
