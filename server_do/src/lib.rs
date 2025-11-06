use worker::*;

#[durable_object]
pub struct MatchDO {
    #[allow(dead_code)]
    state: State,
    #[allow(dead_code)]
    env: Env,
    #[allow(dead_code)]
    initialized: bool,
}

impl DurableObject for MatchDO {
    fn new(state: State, env: Env) -> Self {
        Self {
            state,
            env,
            initialized: true,
        }
    }

    async fn fetch(&self, _req: Request) -> Result<Response> {
        Response::ok("Match Durable Object - TODO: WebSocket support")
    }
}
