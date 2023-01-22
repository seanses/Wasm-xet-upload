# Read a file with Wasm
This example web application is a combination of [wasm_worker_interaction](https://github.com/sgasse/wasm_worker_interaction) and [wasm-bindgen-file-reader-test](https://github.com/Badel2/wasm-bindgen-file-reader-test).

The application lets users choose a local file using [`input type="file"`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input/file). A function defined in `src/lib.rs` reads the first byte of the file and passes that byte to `www/index.js` which logs it to the browser console.

# Build and run
* Executing `build.sh` builds the application, calling `wasm-pack build --out-dir www/pkg --target no-modules`.
* To run the application, start a web server in the directory `www`, using [`http`](https://crates.io/crates/https) for example.
* Point your browser to the app: `firefox --new-window localhost:8000`

# Browser support
Tested successfully with Firefox 109.0 and Chromium 109.0.5414.74.
