compress:
    RUSTFLAGS="-C target-cpu=native" cargo build --release
    ./target/release/compress ../data/corpus/book_reviews.json

perf_bench:
    RUSTFLAGS="-C target-cpu=native" cargo build --release
    perf stat -D 500 -e cycles,branches,branch-misses,L1-dcache-loads,L1-dcache-load-misses,LLC-loads,LLC-load-misses  ./target/release/lpm_benchmark book_reviews

perf_record_bench:
    RUSTFLAGS="-C target-cpu=native" cargo build --release
    perf record -D 500 -e cycles,branches,branch-misses,L1-dcache-loads,L1-dcache-load-misses,LLC-loads,LLC-load-misses  ./target/release/lpm_benchmark book_reviews

perf_stalls_bench:
    RUSTFLAGS="-C target-cpu=native" cargo build --release
    perf stat -D 500 -e CYCLE_ACTIVITY.STALLS_L1D_MISS,CYCLE_ACTIVITY.STALLS_L2_MISS,CYCLE_ACTIVITY.STALLS_L3_MISS,CYCLE_ACTIVITY.STALLS_MEM_ANY,RESOURCE_STALLS.ANY ./target/release/lpm_benchmark book_reviews

perf:
    RUSTFLAGS="-C target-cpu=native" cargo build --release
    perf stat -D 1500 -e cycles,branches,branch-misses,L1-dcache-loads,L1-dcache-load-misses,LLC-loads,LLC-load-misses  ./target/release/compress ../data/corpus/book_reviews.json

perf_stalls:
    RUSTFLAGS="-C target-cpu=native" cargo build --release
    perf stat -D 1500 -e CYCLE_ACTIVITY.STALLS_L1D_MISS,CYCLE_ACTIVITY.STALLS_L2_MISS,CYCLE_ACTIVITY.STALLS_L3_MISS,CYCLE_ACTIVITY.STALLS_MEM_ANY,RESOURCE_STALLS.ANY  ./target/release/compress ../data/corpus/book_reviews.json

missed_opt:
    RUSTFLAGS='-Copt-level=3 -Ctarget-cpu=native -Cforce-frame-pointers=yes -Cremark=all -Cdebuginfo=2 --emit asm --emit llvm-ir' cargo build --release

on-pair_book:
    RUSTFLAGS="-C target-cpu=native" cargo build --release
    ./target/release/benchmark_individual ../data/corpus/book_reviews.json onpair16 on-pair_book

fsst_book:
    RUSTFLAGS="-C target-cpu=native" cargo build --release
    ./target/release/run_single_benchmark ../data/corpus/book_reviews.json fsst fsst_book