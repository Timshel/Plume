use crate::routes::{errors::ErrorPage, Page};
use crate::template_utils::{IntoContext, Ructe};
use plume_models::{db_conn::DbConn, posts::Post, PlumeRocket};

#[get("/tag/<name>?<page>")]
pub fn tag(
    name: String,
    page: Option<Page>,
    mut conn: DbConn,
    rockets: PlumeRocket,
) -> Result<Ructe, ErrorPage> {
    let page = page.unwrap_or_default();
    let posts = Post::list_by_tag(&mut conn, name.clone(), page.limits())?;
    let page_total = Page::total(Post::count_for_tag(&mut conn, name.clone())? as i32);
    Ok(render!(tags::index_html(
        &(&mut conn, &rockets).to_context(),
        name,
        posts,
        page.0,
        page_total
    )))
}
