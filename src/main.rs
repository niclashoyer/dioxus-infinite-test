use std::time::Duration;

use dioxus::{logger::tracing::debug, prelude::*};
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
fn Message(text: String) -> Element {
    rsx! {
        div {
            class: "text-lg bg-blue-300 p-4",
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

#[derive(Debug, Clone, Copy)]
struct ElementValues {
    scroll_top: f32,
    scroll_height: f32,
    offset_height: f32,
}

async fn scroll_values(id: &Uuid) -> ElementValues {
    let mut eval = dioxus::document::eval(&format!(
        r#"
        let el = document.getElementById("{id}");
        dioxus.send(el.scrollTop);
        dioxus.send(el.scrollHeight);
        dioxus.send(el.offsetHeight);
    "#
    ));
    let scroll_top = eval.recv().await.unwrap();
    let scroll_height = eval.recv().await.unwrap();
    let offset_height = eval.recv().await.unwrap();
    ElementValues {
        scroll_top,
        scroll_height,
        offset_height,
    }
}

#[derive(Debug, Clone, Copy)]
enum ScrollHeightBeforeMutation {
    None,
    TopAdd(f32),
    BottomAdd,
    TopRemove(f32),
    BottomRemove,
}

#[component]
fn Infinite(scroll_margin: Option<f32>) -> Element {
    let mut el = use_signal(|| None);
    let mut pos = use_signal(|| 0.0);
    let mut scroll_height_before = use_signal(|| ScrollHeightBeforeMutation::None);
    let mut msgs: Signal<Vec<(Uuid, String)>> = use_signal(|| vec![]);
    let id = use_hook(|| Uuid::new_v4());
    use_hook(move || {
        for _ in 0..15 {
            msgs.push(new_message());
        }
    });
    use_effect(move || {
        use ScrollHeightBeforeMutation::*;
        let height_before = scroll_height_before();
        // if let ScrollHeightBeforeMutation::None = scroll_height_before {
        //     return;
        // }
        spawn(async move {
            let scroll_margin = scroll_margin.unwrap_or(50.0);
            let current_pos = *pos.peek();
            let values = scroll_values(&id).await;
            let scroll_height_now = values.scroll_height;
            let mut new_pos = match height_before {
                TopAdd(scroll_height_before) => {
                    current_pos + (values.scroll_height - scroll_height_before)
                }
                TopRemove(scroll_height_before) => {
                    current_pos - (scroll_height_before - values.scroll_height)
                }
                BottomAdd | BottomRemove => current_pos,
                None => {
                    // scroll into the middle
                    // FIXME: this magic delay is needed on page load, otherwise the values are not correct,
                    // as the css is still not fully loaded
                    dioxus_time::sleep(Duration::from_millis(100)).await;
                    let values = scroll_values(&id).await;
                    (values.scroll_height - values.offset_height) / 2.0
                }
            };

            // if we added elements at top or bottom in the last run,
            // remove the elements on the opposite side
            // FIXME: extract this into some kind of callback
            if let TopAdd(_) = height_before {
                scroll_height_before.set(ScrollHeightBeforeMutation::BottomRemove);
                let mut msgs = msgs.write();
                let len = msgs.len().saturating_sub(5);
                msgs.drain(len..);
            } else if let BottomAdd = height_before {
                scroll_height_before
                    .set(ScrollHeightBeforeMutation::TopRemove(values.scroll_height));
                msgs.write().drain(0..5);
            }

            debug!("pos: {}, values: {:?}", current_pos, values);
            debug!("scroll height before: {:?}", height_before);
            debug!("scroll height now: {}", scroll_height_now);
            debug!("new_pos: {}", new_pos);
            if (new_pos <= scroll_margin) {
                new_pos = scroll_margin + 1.0;
            }
            if let TopRemove(_) | BottomRemove | None = height_before {
                scroll_to(&id, new_pos).await;
            } else {
                pos.set(new_pos);
            }
        });
    });
    rsx! {
        div {
            id: "{id}",
            onmounted: move |e| async move {
                el.set(Some(e.data));
            },
            onscroll: move |_| async move {
                use ScrollHeightBeforeMutation::*;
                let scroll_margin = scroll_margin.unwrap_or(50.0);
                let values = scroll_values(&id).await;
                if values.scroll_top <= scroll_margin {
                    // insert top
                    scroll_height_before.set(TopAdd(values.scroll_height));
                    for _ in 0..5 {
                        msgs.insert(0, new_message());
                    }
                } else if values.scroll_top >= values.scroll_height - values.offset_height - scroll_margin {
                    // insert bottom
                    scroll_height_before.set(BottomAdd);
                    for _ in 0..5 {
                        msgs.push(new_message());
                    }
                }
                pos.set(values.scroll_top);
            },
            class: "max-w-128 max-h-128 bg-gray-100 overflow-y-scroll",
            div {
                id: "{id}-offset",
                class: "h-[1px]",
            }
            div {
                class: "flex flex-col gap-4 p-4",
                for (id, msg) in msgs() {
                    Message {
                        key: "{id}",
                        text: msg,
                    }
                }
            }
        }
        div {
            class: "p-4 flex gap-4",
            button {
                class: "text-lg bg-gray-200 p-2",
                onclick: move |_| async move {
                    scroll_height_before.set(ScrollHeightBeforeMutation::TopAdd(scroll_values(&id).await.scroll_height));
                    for _ in 0..5 {
                        msgs.insert(0, new_message());
                    }
                },
                "Insert Top"
           }
            button {
                class: "text-lg bg-gray-200 p-2",
                onclick: move |_| async move {
                    scroll_height_before.set(ScrollHeightBeforeMutation::BottomAdd);
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
