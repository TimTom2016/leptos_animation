use full::Full;
use leptos::prelude::*;
use leptos_animation::*;
use leptos_router::components::Route;
use leptos_router::components::Router;
use leptos_router::components::Routes;
use leptos_router::path;
use simple::Simple;
use std::panic;
use text::Text;

extern crate console_error_panic_hook;

mod full;
mod simple;
mod text;

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    wasm_logger::init(wasm_logger::Config::default());

    mount_to_body(|| {
        // Initialize a single AnimationContext for all three demo's
        AnimationContext::provide();

        view! {
            <Router>
                <h1>"Animation Demo"</h1>
                <nav>
                    <a href="/leptos_animation/">Full</a>
                    <a href="/leptos_animation/simple">Simple</a>
                    <a href="/leptos_animation/text">Text</a>
                </nav>
                <Routes fallback=|| "Not found">

                    <Route
                        path=path!("/leptos_animation/")
                        view=|| {
                            view! { <Full/> }
                        }
                    />
                    <Route
                        path=path!("/leptos_animation/simple")
                        view=|| {
                            view! { <Simple/> }
                        }
                    />

                    <Route
                        path=path!("/leptos_animation/text")
                        view=|| {
                            view! { <Text/> }
                        }
                    />

                </Routes>
            </Router>
        }
    })
}
