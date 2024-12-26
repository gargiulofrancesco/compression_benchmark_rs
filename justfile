on-pair_book:
    RUSTFLAGS="-C target-cpu=native" cargo build --release
    ./target/release/run_single_benchmark ../data/corpus/book_reviews.json on-pair on-pair_book

fsst_book:
    RUSTFLAGS="-C target-cpu=native" cargo build --release
    ./target/release/run_single_benchmark ../data/corpus/book_reviews.json fsst fsst_book