// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

function toggle_on(event, element_id) {
    e = document.getElementById(element_id);
    e.classList.toggle("show_temporarily");
    event.stopPropagation();
}

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
