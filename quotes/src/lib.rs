use serde::{Deserialize, Serialize};
use worker::*;

#[event(fetch, respond_with_errors)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    let router = Router::new();

    router
        .get_async("/api/quote", |_req, ctx| async move {
            let Ok(random_quote) = get_random_quote(ctx).await else {
                return Response::empty();
            };

            let htmx_component = substitute_into_template(random_quote);

            Response::from_html(htmx_component)
        })
        .run(req, env)
        .await
}

async fn get_random_quote(ctx: worker::RouteContext<()>) -> worker::Result<Quote> {
    let quotes_db = ctx.d1("quotes")?;

    let random_quote = quotes_db
        .prepare(
            "
            SELECT body, writer, source_media
            FROM quotes
            ORDER BY RANDOM()
            LIMIT 1
        ",
        )
        .first::<Quote>(None)
        .await?
        .expect("Failed to find a quote, check you have populated the database with quotes");

    Ok(random_quote)
}

fn substitute_into_template(quote: Quote) -> String {
    console_debug!("{:?}", quote);

    let mut template = include_str!("quote_component.html").to_string();

    template = template.replace("{{body}}", &quote.body);

    let attribution = [&quote.source_media, &quote.writer]
        .iter()
        .filter_map(|s| empty_string_to_none(s))
        .collect::<Vec<&str>>()
        .join(" - ");

    let htmx_component = template.replace("{{attribution}}", &attribution);

    htmx_component
}

fn empty_string_to_none(s: &str) -> Option<&str> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Quote {
    pub body: String,

    /// the name of the author, band, artist, etc
    pub writer: String, // sometimes this is NULL, so it will be set to ""

    /// the title of the song, book, etc
    pub source_media: String, // sometimes this is NULL, so it will be set to ""
}
