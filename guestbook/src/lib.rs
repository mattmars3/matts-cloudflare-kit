use serde::{Deserialize, Serialize};
use worker::*;

const GUESTBOOK_SIGNATURE_LIMIT: usize = 20;

#[event(fetch, respond_with_errors)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    let router = Router::new();

    router
        .post_async("/api/guestbook/sign", |req, ctx| async move {
            let form = req.clone()?.form_data().await?;
            let Some(name) = form.get_field("name") else {
                return Response::from_html("<p>Must provide the name post field</p>");
            };
            let Some(message) = form.get_field("message") else {
                return Response::from_html("<p>Must provide the message post field</p>");
            };

            // no XSS allowed here
            let name = htmlescape::encode_minimal(&name);
            let message = htmlescape::encode_minimal(&message);

            let sign_req = Signature {
                name: name.clone(),
                message: message.clone(),
            };

            let _res = sign_guestbook(ctx, sign_req).await;

            let sign_req_timestamped = SignatureWithTime {
                name,
                message,
                created_at: "Just Now".to_string(),
            };

            let htmx_response = generate_htmx_component(vec![sign_req_timestamped], 0);

            Response::from_html(htmx_response)
        })
        .get_async("/api/guestbook", |req, ctx| async move {
            console_log!("get guestbook request");

            let query: QueryParams = req.query()?;
            let page_to_return = query.page.unwrap_or(0);

            console_log!("getting guestbook signatures");
            let signatures: Vec<SignatureWithTime> =
                get_guestbook_signatures(ctx, page_to_return).await?;

            console_log!("generating htmx");
            let htmx_response = generate_htmx_component(signatures, page_to_return);

            console_log!("done");
            Response::from_html(&htmx_response)
        })
        .run(req, env)
        .await
}

#[derive(Deserialize, Serialize)]
pub struct Signature {
    name: String,
    message: String,
}

#[derive(Deserialize, Serialize)]
pub struct SignatureWithTime {
    name: String,
    message: String,
    created_at: String,
}

#[derive(Deserialize)]
struct QueryParams {
    page: Option<usize>,
}

pub fn generate_htmx_component(signatures: Vec<SignatureWithTime>, current_page: usize) -> String {
    let mut html = String::new();

    for sig in signatures.iter() {
        // create formatted datetime
        html.push_str(&format!(
            r#"
            <div class="guestbook-entry">
                <p class="guestbook-entry-name">{}:</p>
                <p class="guestbook-entry-message">{}</p>
                <p class="guestbook-entry-timestamp">{}</p>
            </div>
            "#,
            sig.name, sig.message, sig.created_at
        ));
    }

    if signatures.len() == GUESTBOOK_SIGNATURE_LIMIT {
        let next_page = current_page + 1;

        html.push_str(&format!(
            r#"
            <div
              hx-get="/api/guestbook?page={}"
              hx-trigger="revealed"
              hx-swap="beforeend"
              hx-target=".guestbook-entries">
            </div>
            "#,
            next_page
        ));
    }

    html
}

pub async fn sign_guestbook(ctx: worker::RouteContext<()>, sign_req: Signature) -> Result<(), ()> {
    // create handle to d1
    let Ok(guestbook_db) = ctx.d1("guestbook") else {
        return Err(());
    };

    let _res = guestbook_db
        .prepare("INSERT INTO guestbook (name, message) VALUES (?1, ?2)")
        .bind(&[sign_req.name.into(), sign_req.message.into()])
        .expect("Unable to prepare and bind query")
        .run()
        .await;

    Ok(())
}

pub async fn get_guestbook_signatures(
    ctx: worker::RouteContext<()>,
    offset: usize,
) -> worker::Result<Vec<SignatureWithTime>> {
    let guestbook_db = ctx.d1("guestbook")?;

    let signature_results = guestbook_db
        .prepare(
            "
            SELECT name, message, created_at
            FROM guestbook
            ORDER BY created_at DESC
            LIMIT ?1
            OFFSET ?2;
        ",
        )
        .bind(&[GUESTBOOK_SIGNATURE_LIMIT.into(), offset.into()])?
        .all()
        .await?;

    Ok(signature_results.results()?)
}

pub async fn root() -> &'static str {
    "Hello Axum!"
}
