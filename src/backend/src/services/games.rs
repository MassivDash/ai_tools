use crate::api::games;
use actix_web::web;

pub fn configure_games_services(cfg: &mut web::ServiceConfig) {
    cfg.configure(games::config);
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    /// Registration is checked by confirming each route resolves to *something*
    /// other than a 404. The chat route needs app data that is deliberately not
    /// provided, and the websocket routes reject a plain request that carries no
    /// upgrade headers - so no handler body actually runs.
    #[actix_web::test]
    async fn test_configure_games_services_registers_every_route() {
        let app = test::init_service(App::new().configure(configure_games_services)).await;

        let req = test::TestRequest::post()
            .uri("/api/games/chat/stream")
            .to_request();
        assert_ne!(test::call_service(&app, req).await.status().as_u16(), 404);

        for path in ["/api/games/ws", "/api/games/1-z-10/ws"] {
            let req = test::TestRequest::get().uri(path).to_request();
            assert_ne!(
                test::call_service(&app, req).await.status().as_u16(),
                404,
                "{} should be registered",
                path
            );
        }
    }

    #[actix_web::test]
    async fn test_unregistered_games_paths_are_not_found() {
        let app = test::init_service(App::new().configure(configure_games_services)).await;

        let req = test::TestRequest::get().uri("/api/games/nope").to_request();
        assert_eq!(test::call_service(&app, req).await.status().as_u16(), 404);
    }
}
