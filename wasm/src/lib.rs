use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct JsonPath {
    path: sql_json_path::JsonPath,
}

#[wasm_bindgen]
impl JsonPath {
    #[wasm_bindgen(constructor)]
    pub fn compile(path: &str) -> Result<JsonPath, JsValue> {
        let path = sql_json_path::JsonPath::new(path)
            .map_err(|e| JsValue::from_str(&format!("Compile error: {}", e)))?;
        Ok(Self { path })
    }

    #[wasm_bindgen]
    pub fn query(&self, json_str: &str) -> Result<String, JsValue> {
        let json: serde_json::Value = json_str
            .parse()
            .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;
        let results = self
            .path
            .query(&json)
            .map_err(|e| JsValue::from_str(&format!("Query error: {}", e)))?;
        let result =
            serde_json::Value::Array(results.into_iter().map(|v| v.into_owned()).collect());
        Ok(serde_json::to_string_pretty(&result).unwrap())
    }
}

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
