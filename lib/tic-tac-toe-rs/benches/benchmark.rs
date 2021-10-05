use criterion::{
    black_box,
    criterion_group,
    criterion_main,
    Criterion,
};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("minimax all", |b| {
        b.iter(|| {
            tic_tac_toe::minimax(
                black_box(tic_tac_toe::Board::new()),
                black_box(tic_tac_toe::NUM_TILES),
                -100,
                100
            )
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
