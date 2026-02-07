
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    fn setTimeout(closure: &Closure<dyn FnMut()>, time: u32) -> i32;
}

#[cfg(target_arch = "wasm32")]
pub async fn sleep(ms: u64) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let window = web_sys::window().unwrap();
        window.set_timeout_with_callback_and_timeout_and_arguments_0(
            &resolve,
            ms as i32
        ).unwrap();
    });
    
    wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep(ms: u64) {
    // This might not be used if interpreter uses tokio directly, 
    // but good to have fallback.
    tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
}
