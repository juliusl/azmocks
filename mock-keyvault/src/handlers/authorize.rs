use poem::{get, handler, Route, RouteMethod};

pub struct Handler;

impl Handler {
    pub fn install(&self, route: Route) -> Route {
        route.at("/authorize", self.get())
    }

    fn get(&self) -> RouteMethod {
        get(get_token_test)
    }
}

#[handler]
pub fn get_token_test() -> &'static str {
    println!("called get token handler");
    "ok"
}
