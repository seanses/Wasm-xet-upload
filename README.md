# Read a file with Wasm
This example web application is a combination of [wasm_worker_interaction](https://github.com/sgasse/wasm_worker_interaction) and [wasm-bindgen-file-reader-test](https://github.com/Badel2/wasm-bindgen-file-reader-test).

The application lets users choose a local file using [`input type="file"`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input/file). A function defined in [`src/lib.rs`](./src/lib.rs) reads the first byte of the file and passes that byte to [`www/index.js`](./www/index.js) which logs it to the browser console.

# Build and run
I used [`wasm-pack`](https://github.com/rustwasm/wasm-pack) version 0.10.3 and stable [`rustc`](https://www.rust-lang.org/tools/install) 1.66.1 to build the application on Arch Linux 6.1.6.

* Execute `build.sh` to build the application, this calls `wasm-pack`.
* To run the application, start a web server like [`http`](https://crates.io/crates/https) in the directory `www/`.
* Fetch and run the app with your browser: `firefox --new-window localhost:8000`

# Browser support
Tested successfully with Firefox 109.0 and Chromium 109.0.5414.74.

