================================================================================

SEODISPARATE.COM's COMMENT SYSTEM

================================================================================

A backend that uses Github API to authenticate users such that only users with
Github accounts can create comments.


================================================================================

Setting It Up

================================================================================

Create an OAuth app on Github (visible under Developer Settings).

Install a MySQL server (MariaDB recommended), and set up a database and user.

Check the "example.config" and edit the fields accordingly.

Set up your webserver to point towards this backend server:

NGINX EXAMPLE:

location /comment_api {
    rewrite /comment_api(.*) $1 break;

    proxy_pass 127.0.0.1:9090;
}

Ensure your "base_url" config is set to the front-facing webserver's url with
"/comment_api" appended to it.

base_url=https://example.com/comment_api

Note that "allowed_bid" in the config can be specified multiple times for each
possible "blog_id". This is a whitelist. If a comment is submitted to a blog_id
that isn't one of these, then it will be rejected.


================================================================================

Client API

================================================================================

Get a single comment's text:

comment_id = id string

{BASE_URL}/get_comment?comment_id=123456789012345678901234567890123456


Get all comments for a given blog_id:

blog_id = blog post id

{BASE_URL}/get_comments?blog_id=my_other_blog_post

This returns JSON:

[
    {
        "comment_id": "string",
        "username": "string",
        "userurl": "profile url string",
        "useravatar": "profile image url string",
        "create_date": "formatted date string",
        "edit_date": "formatted date string",
        "comment": "full comment text string"
    },
    ...
]

Note that if the blog_id has no comments, then the returned JSON is an empty
array.


================================================================================

Client Endpoints

================================================================================

These endpoints must be accessed by the user directly, and not with a headless
http/https request.


Submit a comment:

blog_id = blog post id (must be allowed in config)
blog_url = url to return to after submitting a comment

{BASE_URL}/do_comment?blog_id=my_blog_post&blog_url=https://example.com/my_blog


Edit a comment:

comment_id = id of the comment to edit
blog_url = url to return to after editing a comment

{BASE_URL}/edit_comment?comment_id=1234567890&blog_url=https://example.com/my_blog

Note that only the user who made the comment can edit it.


Delete a comment:

comment_id = id of the comment to delete
blog_url = url to return to after deleting the comment

{BASE_URL}/del_comment?comment_id=1234567890&blog_url=https://example.com/my_blog

Note that only the user who made the comment can delete it.
