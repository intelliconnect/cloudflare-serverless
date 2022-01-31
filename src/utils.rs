use cfg_if::cfg_if;
use worker::{wasm_bindgen::JsValue, *};

cfg_if! {
    // https://github.com/rustwasm/console_error_panic_hook#readme
    if #[cfg(feature = "console_error_panic_hook")] {
        extern crate console_error_panic_hook;
        pub use self::console_error_panic_hook::set_once as set_panic_hook;
    } else {
        #[inline]
        pub fn set_panic_hook() {}
    }
}

pub async fn mongo_request(
    method: Method,
    ctx: &RouteContext<()>,
    body: &str,
    query_type: &str,
    query: &str,
) -> Result<String> {
    let url = format!(
        "https://data.mongodb-api.com/app/data-bkmnk/endpoint/data/beta/{}/{}",
        query_type, query
    );
    let mut headers = Headers::new();
    headers.set("Access-Control-Request-Headers", "*")?;
    headers.set("Content-Type", "application/json")?;
    headers.set(
        "api-key",
        ctx.secret("mongo_data_api_key")?.to_string().as_str(),
    )?;
    let mut request = RequestInit::new();
    request
        .with_method(method)
        .with_headers(headers)
        .with_body(Some(JsValue::from_str(body)));
    let mut response = Fetch::Request(Request::new_with_init(&url, &request)?)
        .send()
        .await?;
    response.text().await
}
