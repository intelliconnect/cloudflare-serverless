use serde_json::json;
use worker::*;

mod utils;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_log!(
        "{} {}, located at: {:?}, within: {}",
        req.method().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );

    log_request(&req);

    // Optionally, get more helpful error messages written to the console in the case of a panic.
    utils::set_panic_hook();

    // Optionally, use the Router to handle matching endpoints, use ":name" placeholders, or "*name"
    // catch-alls to match on specific patterns. Alternatively, use `Router::with_data(D)` to
    // provide arbitrary data that will be accessible in each route via the `ctx.data()` method.
    let router = Router::new();

    // Add as many routes as your Worker needs! Each route will get a `Request` for handling HTTP
    // functionality and a `RouteContext` which you can use to  and get route parameters and
    // Environment bindings like KV Stores, Durable Objects, Secrets, and Variables.
    router
        .get("/", |_, _| Response::ok("Hello from Workers!"))
        //get data from mongo db using DATA api
        .get_async("/mongo", |_, ctx| async move {
            let body = json!({
                "dataSource": "Cluster0",
                "database": "task-manager-api",
                "collection": "tasks"
            });
            let body_str = body.to_string();
            let res = utils::mongo_request(Method::Post, &ctx, body_str.as_str(), "action", "find")
                .await?;
            let res_json = serde_json::from_str::<serde_json::Value>(res.as_str()).unwrap();
            Response::from_json(&res_json)
        })
        // post data to mongo db
        .post_async("/mongo_post", |mut req, ctx| async move {
            let r = req.json::<serde_json::Value>().await?;
            let body = json!({
                "dataSource": "Cluster0",
                "database": "task-manager-api",
                "collection": "tasks",
                "document": r
            });
            let body_str = body.to_string();
            let res =
                utils::mongo_request(Method::Post, &ctx, body_str.as_str(), "action", "insertOne")
                    .await?;
            let res_json = serde_json::from_str::<serde_json::Value>(res.as_str()).unwrap();
            Response::from_json(&res_json)
        })
        // create key:value pair in cloudflare worker KV
        .post_async("/create_user/:name", |mut req, ctx| async move {
            let body = req.json::<serde_json::Value>().await?;
            let rust_kv = ctx.kv("KV_FROM_RUST")?;
            if let Some(name) = ctx.param("name") {
                rust_kv.put(name, body)?.execute().await?;
                Response::ok("created kv pair")
            } else {
                Response::error("an error occured while adding kv pair", 500)
            }
        })
        // get Key:Value pair from cloudflare worker KV
        .get_async("/user/:name", |_, ctx| async move {
            let rust_kv = ctx.kv("KV_FROM_RUST")?;
            if let Some(name) = ctx.param("name") {
                Response::from_json(
                    &json!({"user": rust_kv.get(name).json::<serde_json::Value>().await?.unwrap()}),
                )
            } else {
                Response::error("an error occured getting kv", 500)
            }
        })
        //example of operation with form data
        .post_async("/form/:field", |mut req, ctx| async move {
            if let Some(name) = ctx.param("field") {
                let form = req.form_data().await?;
                match form.get(name) {
                    Some(FormEntry::Field(value)) => {
                        return Response::from_json(&json!({ name: value }))
                    }
                    Some(FormEntry::File(_)) => {
                        return Response::error("`field` param in form shouldn't be a File", 422);
                    }
                    None => return Response::error("Bad Request", 400),
                }
            }

            Response::error("Bad Request", 400)
        })
        // get the worker version
        .get("/worker-version", |_, ctx| {
            let version = ctx.var("WORKERS_RS_VERSION")?.to_string();
            Response::ok(version)
        })
        .run(req, env)
        .await
}
