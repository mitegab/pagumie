//! Conversion benchmarks.
//!
//! The interesting number is not "how fast is a date conversion" (it is a
//! handful of integer divisions) but whether anything in the API accidentally
//! costs an allocation or a division that could be a multiply. Run with:
//!
//! ```sh
//! cargo bench -p ethiopic-calendar
//! ```

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion, Throughput};

use ethiopic_calendar::bahire_hasab::{BahireHasab, MovableFeast};
use ethiopic_calendar::format::{format_date, Locale};
use ethiopic_calendar::{EthiopianDate, GregorianDate, JulianDate, RataDie};

fn sample_rds() -> Vec<i64> {
    // A deterministic spread across ~4000 years, so the branch predictor gets
    // no help from monotone input.
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    (0..1024)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % 1_460_000) as i64 - 200_000
        })
        .collect()
}

fn conversions(c: &mut Criterion) {
    let rds = sample_rds();
    let dates: Vec<EthiopianDate> = rds
        .iter()
        .map(|&rd| EthiopianDate::from_rata_die(RataDie::new(rd)).unwrap())
        .collect();

    let mut group = c.benchmark_group("conversion");
    group.throughput(Throughput::Elements(rds.len() as u64));

    group.bench_function("ethiopic_from_rata_die", |b| {
        b.iter(|| {
            for &rd in &rds {
                black_box(EthiopianDate::from_rata_die(RataDie::new(black_box(rd))).unwrap());
            }
        });
    });

    group.bench_function("ethiopic_to_rata_die", |b| {
        b.iter(|| {
            for d in &dates {
                black_box(black_box(d).to_rata_die());
            }
        });
    });

    group.bench_function("ethiopic_to_gregorian", |b| {
        b.iter(|| {
            for d in &dates {
                black_box(black_box(d).to_gregorian());
            }
        });
    });

    group.bench_function("gregorian_to_ethiopic", |b| {
        let gregs: Vec<GregorianDate> = dates.iter().map(|d| d.to_gregorian()).collect();
        b.iter(|| {
            for g in &gregs {
                black_box(EthiopianDate::from_gregorian(black_box(*g)).unwrap());
            }
        });
    });

    group.bench_function("julian_from_rata_die", |b| {
        b.iter(|| {
            for &rd in &rds {
                black_box(JulianDate::from_rata_die(RataDie::new(black_box(rd))).unwrap());
            }
        });
    });

    group.finish();
}

fn computus(c: &mut Criterion) {
    let mut group = c.benchmark_group("computus");
    group.bench_function("bahire_hasab_new", |b| {
        b.iter_batched(
            || 2017i32,
            |year| black_box(BahireHasab::new(black_box(year)).unwrap()),
            BatchSize::SmallInput,
        );
    });
    group.bench_function("fasika", |b| {
        let bh = BahireHasab::new(2017).unwrap();
        b.iter(|| black_box(bh.feast(black_box(MovableFeast::Tinsae)).unwrap()));
    });
    group.bench_function("all_feasts", |b| {
        let bh = BahireHasab::new(2017).unwrap();
        b.iter(|| black_box(bh.feasts().unwrap()));
    });
    group.finish();
}

fn formatting(c: &mut Criterion) {
    let d = EthiopianDate::from_ymd(2017, 13, 5).unwrap();
    let mut group = c.benchmark_group("format");
    group.bench_function("iso", |b| {
        b.iter(|| black_box(format_date(black_box(d), "%Y-%m-%d", Locale::Amharic)));
    });
    group.bench_function("long_amharic", |b| {
        b.iter(|| {
            black_box(format_date(
                black_box(d),
                "%A %-d %B %Y %N",
                Locale::Amharic,
            ))
        });
    });
    group.bench_function("geez_numeral_year", |b| {
        b.iter(|| black_box(format_date(black_box(d), "%G", Locale::Amharic)));
    });
    group.finish();
}

criterion_group!(benches, conversions, computus, formatting);
criterion_main!(benches);
