const {add_two_numbers} = wasm_bindgen;

async function run_wasm() {
    // Load the Wasm file by awaiting the Promise returned by `wasm_bindgen`
    // `wasm_bindgen` was imported in `index.html`
    await wasm_bindgen('./pkg/read_file_with_wasm_bg.wasm');

    console.log("index.js has loaded Wasm file → Functions defined with Rust are now available");

    console.log("Rust calculates: 2 + 4 =", add_two_numbers(2, 4));

    // Create a worker in JS. The worker also uses Rust functions
    var myWorker = new Worker('./worker.js');

    document.getElementById("file_picker").addEventListener(
        "change",
        function() {

            console.log("eventListener 'change' inside index.js runs");
            let file = this.files[0];
            myWorker.postMessage({ file: file, offset: BigInt(0) });
            myWorker.onmessage = function(e) {
                console.log("First byte of file is: 0x" + e.data.toString(16));
            };
        },
        false
    );
}

run_wasm();
