# rust-css-loader
A simple macro to include style files in Rust using CSS Modules-like scoping.


## Example

### main.rs:
```rust
use yew::prelude::*;

import_style!("src/styles.scss");

#[function_component(App)]
fn app() -> Html {
    html! {
        <h1 class={styles.header}>{ "Hello World" }</h1>
    }
}

fn main() {
    println!("Hello, world!");
    yew::start_app::<App>();
}
```

### styles.scss
```scss
$textcolor: yellow;
$fontsize: 18px;

.header {
  color: $textcolor;
  font-size: $fontsize;
}
```

## Notes

Styles are imported relative to `CARGO_MANIFEST_DIR`, basically the directory that contains the `Cargo.toml` file. It would be nice to do relative path imports like in Javascript, however, due to a little [limitation](https://github.com/rust-lang/rust/issues/54725) in the Rust compiler, this can't be done right now.

Secondly, the same limitation prevents us from having a way to "link" the style file to the Rust source file, hence modifying the style files won't trigger a rebuild/reload. 

These both issues be addressed after this API is stabilized in the Rust compiler.

`.css` files are imported directly and `.scss` are compiled with [rsass](https://github.com/kaj/rsass) and loaded automatically.