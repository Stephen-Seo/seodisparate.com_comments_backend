"use strict";

// This is some example javascript that loads comments for a particular blog_id
// and sets up buttons to add/edit/delete comments.
// It expects a <div id="blog_id"></div> somewhere on the page.

async function load_blog_comments(blog_id, blog_url, base_url) {
  let blog_url_encoded = encodeURIComponent(blog_url);

  let comment_div = document.getElementById(blog_id);
  comment_div.innerHTML = "";
  const response = await fetch(base_url + "/get_comments?blog_id=" + blog_id);
  if (!response.ok) {
    comment_div.innerText = "ERROR: Failed to fetch comments!";
    return;
  }
  try {
    let json_arr = JSON.parse(await response.text());
    if (json_arr.length == 0) {
      comment_div.innerText = "There are no comments.";
    } else {
      for (let idx = 0; idx < json_arr.length; ++idx) {
        let br_elem = document.createElement("br");
        comment_div.appendChild(br_elem);
        let img_elem = document.createElement("img");
        img_elem.setAttribute("width", "64");
        img_elem.setAttribute("height", "64");
        img_elem.setAttribute("src", json_arr[idx].useravatar);
        comment_div.appendChild(img_elem);
        let bold_elem = document.createElement("b");
        bold_elem.innerText = json_arr[idx].username;
        comment_div.appendChild(bold_elem);
        let link_elem = document.createElement("a");
        link_elem.setAttribute("href", json_arr[idx].userurl);
        link_elem.innerText = "(User Profile)";
        comment_div.appendChild(link_elem);
        let create_time_bold = document.createElement("b");
        let create_time_obj = new Date(json_arr[idx].create_date);
        let edit_time_obj = new Date(json_arr[idx].edit_date);
        create_time_bold.innerText = "Created: " + create_time_obj.toString();
        create_time_bold.innerText += ", Edited: " + edit_time_obj.toString();
        comment_div.appendChild(document.createElement("br"));
        comment_div.appendChild(create_time_bold);
        let comment_text = document.createElement("div");
        comment_text.innerText = json_arr[idx].comment;
        comment_div.appendChild(comment_text);
        let edit_button = document.createElement("button");
        edit_button.innerText = "Edit";
        edit_button.onclick = (e) => {
          window.location = base_url + "/edit_comment?comment_id=" + json_arr[idx].comment_id + "&blog_url=" + blog_url_encoded;
        };
        comment_div.appendChild(edit_button);
        let delete_button = document.createElement("button");
        delete_button.innerText = "Delete";
        delete_button.onclick = (e) => {
          window.location = base_url + "/del_comment?comment_id=" + json_arr[idx].comment_id + "&blog_url=" + blog_url_encoded;
        };
        comment_div.appendChild(delete_button);
      }
    }
  } catch (error) {
    comment_div.innerText = "ERROR: Failed to parse fetched comments!";
    console.error(error);
    return;
  }

  comment_div.appendChild(document.createElement("br"));
  comment_div.appendChild(document.createElement("br"));
  let new_comment_button = document.createElement("button");
  new_comment_button.onclick = (e) => {
    window.location = base_url + "/do_comment?blog_id=" + blog_id + "&blog_url=" + blog_url_encoded;
  };
  new_comment_button.innerText = "Submit a New Comment";
  comment_div.appendChild(new_comment_button);
}
