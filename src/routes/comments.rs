use crate::template_utils::Ructe;
use activitystreams::object::Note;
use rocket::{
    form::Form,
    response::{Flash, Redirect},
};
use validator::Validate;

use std::time::Duration;

use crate::routes::errors::ErrorPage;
use crate::template_utils::IntoContext;
use plume_common::{
    activity_pub::{broadcast, ActivityStream, ApRequest},
    utils,
};
use plume_models::{
    blogs::Blog, comments::*, db_conn::DbConn, inbox::inbox, instance::Instance, medias::Media,
    mentions::Mention, posts::Post, safe_string::SafeString, tags::Tag, users::User, Error,
    PlumeRocket, CONFIG,
};

#[derive(Default, FromForm, Debug, Validate)]
pub struct NewCommentForm {
    pub responding_to: Option<i32>,
    #[validate(length(min = 1, message = "Your comment can't be empty"))]
    pub content: String,
    pub warning: String,
}

#[post("/~/<blog_name>/<slug>/comment", data = "<form>")]
pub fn create(
    blog_name: String,
    slug: String,
    form: Form<NewCommentForm>,
    user: User,
    mut conn: DbConn,
    rockets: PlumeRocket,
) -> Result<Flash<Redirect>, Ructe> {
    let blog = Blog::find_by_fqn(&mut conn, &blog_name).expect("comments::create: blog error");
    let post = Post::find_by_slug(&mut conn, &slug, blog.id).expect("comments::create: post error");
    form.validate()
        .map(|_| {
            let (html, mentions, _hashtags) = utils::md_to_html(
                form.content.as_ref(),
                Some(
                    &Instance::get_local()
                        .expect("comments::create: local instance error")
                        .public_domain,
                ),
                true,
                Some(Media::get_media_processor(&mut conn, vec![&user])),
            );
            let comm = Comment::insert(
                &mut conn,
                NewComment {
                    content: SafeString::new(html.as_ref()),
                    in_response_to_id: form.responding_to,
                    post_id: post.id,
                    author_id: user.id,
                    ap_url: None,
                    sensitive: !form.warning.is_empty(),
                    spoiler_text: form.warning.clone(),
                    public_visibility: true,
                },
            )
            .expect("comments::create: insert error");
            let new_comment = comm
                .create_activity(&mut conn)
                .expect("comments::create: activity error");

            // save mentions
            for ment in mentions {
                let activity = &Mention::build_activity(&mut conn, &ment)
                    .expect("comments::create: build mention error");

                Mention::from_activity(
                    &mut conn,
                    activity,
                    comm.id,
                    false,
                    true,
                )
                .expect("comments::create: mention save error");
            }

            comm.notify(&mut conn).expect("comments::create: notify error");

            // federate
            let dest = User::one_by_instance(&mut conn).expect("comments::create: dest error");
            let user_clone = user.clone();
            rockets.worker.execute(move || {
                broadcast(&user_clone, new_comment, dest, CONFIG.proxy().cloned())
            });

            Flash::success(
                Redirect::to(uri!(
                    super::posts::details(blog = blog_name,slug = slug,responding_to = _)
                )),
                i18n!(&rockets.intl.catalog, "Your comment has been posted."),
            )
        })
        .map_err(|errors| {
            // TODO: de-duplicate this code
            let comments = CommentTree::from_post(&mut conn, &post, Some(&user))
                .expect("comments::create: comments error");

            let previous = form.responding_to.and_then(|r| Comment::get(&mut conn, r).ok());

            let tags = Tag::for_post(&mut conn, post.id).expect("comments::create: tags error");
            let likes = post.count_likes(&mut conn).expect("comments::create: count likes error");
            let counts = post.count_reshares(&mut conn).expect("comments::create: count reshares error");
            let liked = user.has_liked(&mut conn, &post).expect("comments::create: liked error");
            let reshared = user.has_reshared(&mut conn, &post).expect("comments::create: reshared error");

            let author = post.get_authors(&mut conn).expect("comments::create: authors error").swap_remove(0);
            let following = user.is_following(&mut conn, author.id).expect("comments::create: following error");

            render!(posts::details_html(
                &(&mut conn, &rockets).to_context(),
                post.clone(),
                blog,
                &*form,
                errors,
                tags,
                comments,
                previous,
                likes,
                counts,
                liked,
                reshared,
                following,
                author
            ))
        })
}

#[post("/~/<blog>/<slug>/comment/<id>/delete")]
pub fn delete(
    blog: String,
    slug: String,
    id: i32,
    user: User,
    mut conn: DbConn,
    rockets: PlumeRocket,
) -> Result<Flash<Redirect>, ErrorPage> {
    if let Ok(comment) = Comment::get(&mut conn, id) {
        if comment.author_id == user.id {
            let dest = User::one_by_instance(&mut conn)?;
            let delete_activity = comment.build_delete(&mut conn)?;
            inbox(
                &mut conn,
                serde_json::to_value(&delete_activity).map_err(Error::from)?,
            )?;

            let user_c = user.clone();
            rockets.worker.execute(move || {
                broadcast(&user_c, delete_activity, dest, CONFIG.proxy().cloned())
            });
            rockets
                .worker
                .execute_after(Duration::from_secs(10 * 60), move || {
                    user.rotate_keypair(&mut conn)
                        .expect("Failed to rotate keypair");
                });
        }
    }
    Ok(Flash::success(
        Redirect::to(uri!(
            super::posts::details(blog = blog,
            slug = slug,
            responding_to = _
        ))),
        i18n!(&rockets.intl.catalog, "Your comment has been deleted."),
    ))
}

#[get("/~/<_blog>/<_slug>/comment/<id>")]
pub fn activity_pub(
    _blog: String,
    _slug: String,
    id: i32,
    _ap: ApRequest,
    mut conn: DbConn,
) -> Option<ActivityStream<Note>> {
    Comment::get(&mut conn, id)
        .and_then(|c| c.to_activity(&mut conn))
        .ok()
        .map(ActivityStream::new)
}
