# Compost
kpw_compose-ng

# Run the example
Make sure you [have the Rust toolchain installed](https://www.rust-lang.org/learn/get-started).
```sh
# Build compost
cargo build -r

# Move binary to example site
cp target/release/compost example_site/

cd example_site

# precompile stb headers
sh lib/build.sh

# run compost
./compost
```
Built pages will be in example_site/out.
