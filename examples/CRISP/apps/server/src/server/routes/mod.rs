mod rounds;
mod state;
mod voting;

use actix_web::web;

pub fn setup_routes(config: &mut web::ServiceConfig) {
    state::setup_routes(config);
    voting::setup_routes(config);
    rounds::setup_routes(config);
}
