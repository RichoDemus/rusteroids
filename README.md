# rusteroids
Currently deployed at [richodemus.github.io/rusteroids](https://richodemus.github.io/rusteroids)

## run locally
desktop
```
cargo run (--release)
```
web
```
cargo build --release --target wasm32-unknown-unknown && wasm-bindgen --out-name rusteroids --out-dir target/rusteroids-web --target web target/wasm32-unknown-unknown/release/rusteroids.wasm  && cp index.html target/rusteroids-web/ && basic-http-server target/rusteroids-web/
```

## build and deploy
built automatically by github actions
