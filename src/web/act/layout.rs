// Copyright (c) 2022-2023, Alex Badics
//
// This file is part of AJDB
//
// AJDB is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// AJDB is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with AJDB.  If not, see <http://www.gnu.org/licenses/>.

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
                    a href="/" {
                        .ajdb_logo_alternative { "AJDB" }
                    }
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
