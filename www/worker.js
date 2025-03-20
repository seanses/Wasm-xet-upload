importScripts('./pkg/read_file_with_wasm.js');

const { read_at_offset_sync } = wasm_bindgen;
const { clean_file } = wasm_bindgen;

// We compiled with `--target no-modules`, which does not create a module. The generated bindings
// can be loaded in web workers in all modern browsers.

async function run_in_worker() {
    // Load the Wasm file by awaiting the Promise returned by `wasm_bindgen`
    await wasm_bindgen('./pkg/read_file_with_wasm_bg.wasm');
    console.log("worker.js has loaded Wasm file → Functions defined with Rust are now available");
}

run_in_worker();


onmessage = async function (e) {
    console.log("onmessage inside worker.js runs");
    // let workerResult = read_at_offset_sync(
    //     e.data.file,
    //     e.data.offset,
    // );
    let workerResult = clean_file(
        e.data.file,
    );
    postMessage(workerResult);
};
