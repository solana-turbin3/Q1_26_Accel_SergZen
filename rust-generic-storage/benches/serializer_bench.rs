use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};

use rust_generic_storage::{
    person::Person,
    serializer::{
        borsh::BorshSerializer,
        serde::JsonSerializer,
        wincode::WincodeSerializer,
    },
    storage::Storage,
};

fn make_person() -> Person {
    Person {
        name: "Alice".to_string(),
        age: 30,
    }
}

fn bench_borsh_save(c: &mut Criterion) {
    let person = make_person();
    c.bench_function("borsh_save", |b| {
        b.iter(|| {
            let mut storage = Storage::new(BorshSerializer);
            storage.save(black_box(&person)).unwrap();
        })
    });
}

fn bench_json_save(c: &mut Criterion) {
    let person = make_person();
    c.bench_function("json_save", |b| {
        b.iter(|| {
            let mut storage = Storage::new(JsonSerializer);
            storage.save(black_box(&person)).unwrap();
        })
    });
}

fn bench_wincode_save(c: &mut Criterion) {
    let person = make_person();
    c.bench_function("wincode_save", |b| {
        b.iter(|| {
            let mut storage = Storage::new(WincodeSerializer);
            storage.save(black_box(&person)).unwrap();
        })
    });
}

fn bench_borsh_load(c: &mut Criterion) {
    let person = make_person();
    let mut storage = Storage::new(BorshSerializer);
    storage.save(&person).unwrap();

    c.bench_function("borsh_load", |b| {
        b.iter(|| {
            let _: Person = black_box(storage.load().unwrap());
        })
    });
}

fn bench_json_load(c: &mut Criterion) {
    let person = make_person();
    let mut storage = Storage::new(JsonSerializer);
    storage.save(&person).unwrap();

    c.bench_function("json_load", |b| {
        b.iter(|| {
            let _: Person = black_box(storage.load().unwrap());
        })
    });
}

fn bench_wincode_load(c: &mut Criterion) {
    let person = make_person();
    let mut storage = Storage::new(WincodeSerializer);
    storage.save(&person).unwrap();

    c.bench_function("wincode_load", |b| {
        b.iter(|| {
            let _: Person = black_box(storage.load().unwrap());
        })
    });
}

fn bench_borsh_roundtrip(c: &mut Criterion) {
    let person = make_person();
    c.bench_function("borsh_roundtrip", |b| {
        b.iter(|| {
            let mut storage = Storage::new(BorshSerializer);
            storage.save(black_box(&person)).unwrap();
            let _: Person = black_box(storage.load().unwrap());
        })
    });
}

fn bench_json_roundtrip(c: &mut Criterion) {
    let person = make_person();
    c.bench_function("json_roundtrip", |b| {
        b.iter(|| {
            let mut storage = Storage::new(JsonSerializer);
            storage.save(black_box(&person)).unwrap();
            let _: Person = black_box(storage.load().unwrap());
        })
    });
}

fn bench_wincode_roundtrip(c: &mut Criterion) {
    let person = make_person();
    c.bench_function("wincode_roundtrip", |b| {
        b.iter(|| {
            let mut storage = Storage::new(WincodeSerializer);
            storage.save(black_box(&person)).unwrap();
            let _: Person = black_box(storage.load().unwrap());
        })
    });
}

fn bench_save_comparison(c: &mut Criterion) {
    let person = make_person();
    let mut group = c.benchmark_group("save_comparison");

    group.bench_function("borsh", |b| {
        b.iter(|| {
            let mut s = Storage::new(BorshSerializer);
            s.save(black_box(&person)).unwrap();
        })
    });
    group.bench_function("json", |b| {
        b.iter(|| {
            let mut s = Storage::new(JsonSerializer);
            s.save(black_box(&person)).unwrap();
        })
    });
    group.bench_function("wincode", |b| {
        b.iter(|| {
            let mut s = Storage::new(WincodeSerializer);
            s.save(black_box(&person)).unwrap();
        })
    });

    group.finish();
}

fn bench_roundtrip_comparison(c: &mut Criterion) {
    let person = make_person();
    let mut group = c.benchmark_group("roundtrip_comparison");

    group.bench_function("borsh", |b| {
        b.iter(|| {
            let mut s = Storage::new(BorshSerializer);
            s.save(black_box(&person)).unwrap();
            let _: Person = black_box(s.load().unwrap());
        })
    });
    group.bench_function("json", |b| {
        b.iter(|| {
            let mut s = Storage::new(JsonSerializer);
            s.save(black_box(&person)).unwrap();
            let _: Person = black_box(s.load().unwrap());
        })
    });
    group.bench_function("wincode", |b| {
        b.iter(|| {
            let mut s = Storage::new(WincodeSerializer);
            s.save(black_box(&person)).unwrap();
            let _: Person = black_box(s.load().unwrap());
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_borsh_save,
    bench_json_save,
    bench_wincode_save,
    bench_borsh_load,
    bench_json_load,
    bench_wincode_load,
    bench_borsh_roundtrip,
    bench_json_roundtrip,
    bench_wincode_roundtrip,
    bench_save_comparison,
    bench_roundtrip_comparison,
);
criterion_main!(benches);