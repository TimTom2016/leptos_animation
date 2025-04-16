use leptos::prelude::*;
use leptos_animation::*;
use leptos_meta::*;
use leptos_router::{
    components::{Route, Router, Routes, A},
    *,
};

extern crate console_error_panic_hook;

use std::panic;

mod full;
mod simple;
mod text;

use full::Full;
use simple::Simple;
use text::Text;

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    mount_to_body(|| view! {<App/>})
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    AnimationContext::provide();
    view! {
        <Html attr:lang="en" attr:dir="ltr" attr:data-theme="dark" />

        // sets the document title
        <Title text="Welcome to Leptos CSR" />

        // injects metadata in the <head> of the page
        <Meta charset="UTF-8" />
        <Meta name="viewport" content="width=device-width, initial-scale=1.0" />

        <Router base="/leptos_animation">
            <h1>"Animation Demo"</h1>
            <nav>
                <A href="">"Full"</A>
                <A href="simple">"Simple"</A>
                <A href="text">"Text"</A>
            </nav>
            <Routes fallback=move || {
                view! { <div>"Not Found"</div> }
            }>
                <Route
                    path=path!("")
                    view=Full
                />

                <Route
                    path=path!("simple")
                    view=Simple
                />

                <Route
                    path=path!("text")
                    view=Text
                />

            </Routes>
        </Router>
    }
}
