// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use maud::{html, Markup, DOCTYPE};

pub fn document_layout(
    additional_class: &'static str,
    title: String,
    toc: Markup,
    menu: Markup,
    document_body: Markup,
) -> Markup {
    html!(
        (DOCTYPE)
        html {
            head {
                title { (title) " - AJDB" }
                link rel="stylesheet" href="/static/style_common.css";
                link rel="stylesheet" href="/static/style_app.css";
                link rel="icon" href="/static/favicon.png";
                script type="text/javascript" src="/static/jquery-3.6.1.js" {}
                script type="text/javascript" src="/static/scripts_app.js" {}
            }
            body {
                .top_left {
                    a href="/" {
                        .ajdb_logo { "AJDB" }
                    }
                    "Alex Jogi Adatbázisa"
                }
                .top_right {
                    .menu_container .(additional_class) {
                        (menu)
                    }
                }
                .bottom_left {
                    .toc { (toc) }
                }
                .bottom_right {
                    .bottom_right_scrolled {
                        .document .(additional_class) {
                            (document_body)
                        }
                    }
                }
            }
        }
    )
}
