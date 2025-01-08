on-pair_book:
    RUSTFLAGS="-C target-cpu=native" cargo build --release
    ./target/release/benchmark_individual ../data/corpus/book_reviews.json onpair16 on-pair_book