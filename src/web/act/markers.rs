// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use chrono::Duration;
use hun_law::structure::LastChange;
use maud::{html, Markup};

use super::context::RenderElementContext;
use crate::{
    enforcement_date_set::EnforcementDateSet,
    web::util::{act_link, anchor_string, change_snippet_link, OrToday},
};

pub fn render_changes_markers(
    context: &RenderElementContext,
    last_change: &Option<LastChange>,
) -> Option<Markup> {
    if !context.show_changes {
        return None;
    }
    let last_change = last_change.as_ref()?;
    let current_ref = context.current_ref.as_ref()?;
    let change_snippet = Some(change_snippet_link(current_ref, last_change));
    let change_url = format!(
        "{}#{}",
        act_link(current_ref.act()?, Some(last_change.date.pred())),
        anchor_string(current_ref)
    );
    // TODO: or_today is not exactly the most optimal solution for this
    //       frequently called function.
    let change_age = context.date.or_today() - last_change.date;

    Some(html!(
        a .past_change_container href=(change_url) data-snippet=[change_snippet] {
            .past_change_marker
            .new[change_age<Duration::days(365)]
            .very_new[change_age<Duration::days(100)]
            {}
        }
    ))
}

pub fn render_enforcement_date_marker(
    context: &RenderElementContext,
    enforcement_dates: Option<&EnforcementDateSet>,
) -> Option<Markup> {
    let current_ref = context.current_ref.as_ref()?;
    let enforcement_date =
        enforcement_dates?.specific_element_not_in_force(current_ref, context.date.or_today())?;
    let change_url = format!(
        "{}#{}",
        act_link(current_ref.act()?, Some(enforcement_date)),
        anchor_string(current_ref)
    );
    let snippet = enforcement_date
        .format("static:%Y. %m. %d-n lép hatályba")
        .to_string();

    Some(html!(
        a .enforcement_date_marker href=(change_url) data-snippet=(snippet) {
            "🕓︎"
        }
    ))
}
