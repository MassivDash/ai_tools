use actix_web::web::ServiceConfig;

use crate::api::pdf_to_markdown::post::convert_pdf_to_markdown;
use crate::api::text_to_tokens::post::convert_text_to_tokens;
use crate::api::url_to_markdown::post::convert_url_to_markdown;

/// Configures all converter related endpoints
pub fn configure_converter_services(cfg: &mut ServiceConfig) {
    cfg.service(convert_url_to_markdown)
        .service(convert_pdf_to_markdown)
        .service(convert_text_to_tokens);
}

