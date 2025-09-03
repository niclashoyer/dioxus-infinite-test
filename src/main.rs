use std::time::Duration;

use dioxus::{logger::tracing::debug, prelude::*};
use itertools::Itertools;
use lipsum::lipsum_words;
use uuid::Uuid;

const APP_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: APP_CSS }
        div {
            class: "",
            Infinite {}
        }
    }
}

#[component]
fn Message(text: String, scroll_to: Option<bool>) -> Element {
    rsx! {
        div {
            class: "text-lg bg-blue-300 p-4",
            onmounted: move |e| async move {
                if let Some(true) = scroll_to {
                    let _ = e.data.scroll_to(ScrollBehavior::Instant).await;
                    dioxus_time::sleep(Duration::from_millis(100)).await;
                    let _ = e.data.scroll_to(ScrollBehavior::Instant).await;
                }
            },
            p {
                "{text}"
            }
        }
    }
}

fn new_message() -> (Uuid, String) {
    let id = Uuid::new_v4();
    let num = rand::random::<u8>() % 45 + 5;
    let msg = lipsum_words(num.into());
    (id, msg)
}

async fn scroll_to(id: &Uuid, pos: f32) {
    let _ = dioxus::document::eval(&format!(
        r#"
        let el = document.getElementById("{id}");
        let el2 = document.getElementById("{id}-offset")
        let scrollTop = {pos};
        el.scrollTop = scrollTop;
        // if the scroll top has not changed, the element is not scrollable
        // try again later, maybe the page is not fully rendered yet?
        if (scrollTop > 0 && el.scrollTop == 0) {{
            let start = performance.now();
            let int = setInterval(function() {{
                let diff = performance.now() - start;
                if (diff > 200 || el.scrollTop > 0) {{
                    clearInterval(int);
                }}
                el.scrollTop = scrollTop;
            }}, 10);
        }}
    "#
    ))
    .await;
}

async fn scroll_min(id: &Uuid, min: f32) -> f32 {
    let mut eval = dioxus::document::eval(&format!(
        r#"
        let el = document.getElementById("{id}");
        let min = {min};
        if (el.scrollTop <= min) {{
            el.scrollTop = min;
            console.log(min, el.scrollTop);
        }}
        dioxus.send(el.scrollTop);
    "#
    ));
    eval.recv().await.unwrap()
}

async fn scroll_height(id: &Uuid) -> f32 {
    let mut eval = dioxus::document::eval(&format!(
        r#"
        let el = document.getElementById("{id}");
        dioxus.send(el.scrollHeight);
    "#
    ));
    eval.recv().await.unwrap()
}

#[component]
fn Infinite() -> Element {
    let mut el = use_signal(|| None);
    let mut pos = use_signal(|| 0.0);
    let mut scroll_height_before = use_signal(|| 0.0);
    let mut msgs: Signal<Vec<(Uuid, String)>> = use_signal(|| vec![]);
    let id = use_hook(|| Uuid::new_v4());
    use_hook(move || {
        for _ in 0..5 {
            msgs.push(new_message());
        }
    });
    use_effect(move || {
        let scroll_height_before = scroll_height_before();
        spawn(async move {
            let pos = *pos.peek();
            let scroll_height_now = scroll_height(&id).await;
            let new_pos = pos + (scroll_height_now - scroll_height_before);
            debug!("pos: {}", pos);
            debug!("scroll height before: {}", scroll_height_before);
            debug!("scroll height now: {}", scroll_height_now);
            debug!("new_pos: {}", new_pos);
            scroll_to(&id, new_pos).await;
        });
    });
    rsx! {
        div {
            id: "{id}",
            onmounted: move |e| async move {
                el.set(Some(e.data));
                // scroll_to(&id, 42).await;
            },
            onscroll: move |_| async move {
                let top = scroll_min(&id, 1.0).await;
                if top <= 50.0 {
                    scroll_height_before.set(scroll_height(&id).await);
                    for _ in 0..5 {
                        msgs.insert(0, new_message());
                    }
                }
                pos.set(top);
            },
            class: "max-w-128 max-h-128 bg-gray-100 overflow-y-scroll",
            div {
                id: "{id}-offset",
                class: "h-[1px]",
            }
            div {
                class: "flex flex-col gap-4 p-4",
                for (pos, (id, msg)) in msgs().iter().with_position() {
                    Message {
                        key: "{id}",
                        text: msg,
                        scroll_to: pos == itertools::Position::Last
                    }
                }
            }
        }
        div {
            class: "py-4 flex gap-4",
            button {
                class: "text-lg bg-gray-200 p-2",
                onclick: move |_| async move {
                    scroll_height_before.set(scroll_height(&id).await);
                    for _ in 0..5 {
                        msgs.insert(0, new_message());
                    }
                },
                "Insert Top"
           }
            button {
                class: "text-lg bg-gray-200 p-2",
                onclick: move |_| async move {
                    for _ in 0..5 {
                        msgs.push(new_message());
                    }
                },
                "Insert Bottom"
            }
            button {
                class: "text-lg bg-gray-200 p-2",
                onclick: move |_| async move {
                    scroll_to(&id, 1.0).await;
                },
                "Scroll to top"
            }
            div {
                class: "text-lg bg-gray-100 p-2",
                "{pos}"
            }
        }
    }
}
