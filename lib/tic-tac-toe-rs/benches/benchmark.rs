use criterion::{
    black_box,
    criterion_group,
    criterion_main,
    Criterion,
};
use std::time::Duration;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("minimax all", |b| {
        b.iter(|| {
            tic_tac_toe::minimax(
                black_box(tic_tac_toe::Board::new()),
                black_box(tic_tac_toe::NUM_TILES),
            )
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = criterion_benchmark
}
criterion_main!(benches);
