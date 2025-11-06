use worker::*;

// Export the Durable Object from server_do
pub use server_do::MatchDO;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    let router = Router::new();

    router
        .get("/", |_, _| Response::ok("ISO Game Server"))
        .get_async("/create", handle_create)
        .get_async("/join/:code", handle_join)
        .run(req, env)
        .await
}

async fn handle_create(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    // TODO: Generate match code and create Durable Object
    Response::ok("Create endpoint - TODO")
}

async fn handle_join(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // TODO: Join match by code
    let code = ctx.param("code").map_or("unknown", |v| v);
    Response::ok(format!("Join endpoint - code: {}", code))
}
