# Compost (kpw_compose-ng)
Static site generator with infamous `<c>` tags.

## Features other than `<c>`
- Thumbnail generator (ImageMagick)

## Run the example
Make sure you [have the Rust toolchain installed](https://www.rust-lang.org/learn/get-started) and `gcc` and `convert` (ImageMagick) are available.
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

## Configuration
Look at example_site/compost.toml for an example. `compost` will look for `compost.toml` in the same directory by default. You can also provide a different config to `compost` via its first command-line argument.

```toml
cc = "gcc"              # CC needs to be GCC-compatible (e.g. clang)
im = "convert"          # ImageMagick
lib_dir = "./lib/"
include_dir = "./include/"
c_dir = "./bin/"
content_dir = "./content/"
output_dir = "./out/"
copy_year = 2025
root_url = "https://lachrymal.net/"
thumbnails_dir = "thumbnails/"
template_fn = "template.html"
default_thumb = "default.png"
prelude_path = "prelude.c"
font_fn = "lmroman10-regular.otf"
```

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
