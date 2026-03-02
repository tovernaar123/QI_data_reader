function stubgen {
    $env:CARGO_TARGET_DIR = "$PWD\target_stub"
    cargo run --bin stub_gen
}

function devbuild {
    $env:CARGO_TARGET_DIR = "$PWD\target_maturin"
    maturin develop
}

function relbuild {
    $env:CARGO_TARGET_DIR = "$PWD\target_maturin"
    maturin develop --release
}