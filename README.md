# Compost (kpw_compose-ng)
Static site generator with infamous `<c>` tags.

## Run the example
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

## Constructs
`<c>` tags are valid in `content/*.md` files, as well as templates themselves.
The contents of each `<c>` block will be run in a C function, and the stdout will replace the `<c>` tag itself.
The prelude in the initial configuration example includes `stb_image.h`, `stb_image_write.h` for image processing capabilities, as well as `kpw_web_utils.h`.
Check for `do_c` in `src/main.rs` for more information.

## TyperTags
You can extend the markup syntax with custom TyperTags in `content/*.md`, their syntax is as follows:
```
[%TAG_NAME%] {
%TAG_CONTENT%
}
```
Insert your custom routine in `do_typer_tags` in `src/main.rs`.

A simple example where `%TAG_NAME% = note` is given, where it renders `%TAG_CONTENT%` from Markdown into HTML, then substitutes the TyperTag with `<div class="note">%TAG_CONTENT%</div>`.
