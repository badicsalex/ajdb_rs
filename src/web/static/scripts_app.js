// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.
'use strict';
function snippet_hover_new($snippeted_link, $parent){
    var url = $snippeted_link.data('snippet');
    var $snippet_container = $('<div class="snippet_container">Előnézet betöltése...</div>');

    var offset = $snippeted_link.offset();
    var pane_offset = $('.bottom_right_scrolled').offset()
    offset.left -= pane_offset.left;
    offset.top -= pane_offset.top;
    offset.left -= 50;
    offset.top += $snippeted_link.height();
    var right_border = $('.bottom_right_scrolled').width() - 20;

    $snippet_container.html("Előnézet betöltése...")
    $snippet_container.load(url, function( response, status, xhr ) {
        if ( status == "error" ) {
            $snippet_container.html("Előnézet nem elérhető.")
        } else {
            add_snippet_handlers($snippet_container);
        }
        $snippet_container.css({'left': 0});
        if (offset.left + $snippet_container.outerWidth() > right_border){
            offset.left = right_border - $snippet_container.outerWidth()
        }
        $snippet_container.css(offset);
    });
    $snippet_container.css(offset);
    $snippet_container.data('tooltip-parent', $parent);
    snippet_hover_start($snippet_container);
    /* Cancel fadeOut if mouse enters the snippet_container itself */
    /* XXX: This is a hack, basically we use the animation as a way to store state for some time */
    $snippet_container.hover(
        function(){snippet_hover_start($snippet_container)},
        function(){snippet_hover_end(null)},
    );
    $snippet_container.appendTo('.bottom_right_scrolled');
}

function snippet_hover_start($element){
    /* stops hiding this and its parents */
    if ($element) {
        $element.stop();
        $element.fadeIn(200);
    }
    var $parent = $element.data('tooltip-parent');
    if ($parent){
        snippet_hover_start($parent);
    }
}

function clear_tooltip_timeout($element){
    var tooltipTimeout = $element.data('tooltip-timeout');
    if (tooltipTimeout) {
        clearTimeout(tooltipTimeout);
        $element.data('tooltip-timeout', null);
    }
}

function snippet_hover_end($dont_hide_element){
    /* hides all snippet containers */
    $('.snippet_container').stop();
    $('.snippet_container').fadeOut(
        200,
        function(){
            $(this).remove();
            /* The hover stop handler is not called: the timeout might still be active */
            clear_tooltip_timeout($(this));
        }
    );
    if ($dont_hide_element){
        /* XXX: This is an even bigger hack: cancel parents fadeout, since this is a link. */
        snippet_hover_start($dont_hide_element);
    }
}


function add_snippet_handlers($parent) {
    $parent.find("[data-snippet]").each(function() {
        var $snippeted_link = $(this)
        $snippeted_link.hover(
            function(){
                var tooltipTimeout=setTimeout(function(){
                    $parent.data('tooltip-timeout', null);
                    snippet_hover_new($snippeted_link, $parent);
                }, 500);
                $parent.data('tooltip-timeout', tooltipTimeout);
            },
            function(){
                snippet_hover_end($parent);
                clear_tooltip_timeout($parent);
            }
        );
    })
}

function scroll_to_hash(){
    var element_id = window.location.hash.slice(1);
    if (!element_id){
        return;
    }
    var element = document.getElementById(element_id);
    if (!element){
        return;
    }
    element.scrollIntoView({block: "center"})
}

function set_up_hash_change_scrolling() {
    $( window ).on('hashchange', scroll_to_hash)
}

function toggle_on(event, element_id) {
    // TODO: JQuerify
    var e = document.getElementById(element_id);
    e.classList.toggle("show_temporarily");
    event.stopPropagation();
}

// TODO: JQuerify
window.onclick = function(event) {
    /* close everything */
    document
        .querySelectorAll(".show_temporarily")
        .forEach(
            function(elem){
                e.classList.remove("show_temporarily")
            }
        );
}

$(function() {
    add_snippet_handlers($('.document'));
    set_up_hash_change_scrolling();
    scroll_to_hash();
})
