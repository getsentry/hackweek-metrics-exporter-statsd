use std::sync::Arc;

use metrics::{Key, Recorder};
use metrics_exporter_statsd::{html, Registry, StatsdBuilder};
use warp::Filter;

#[tokio::main]
async fn main() {
    let registry = Arc::new(Registry::new());

    let recorder = StatsdBuilder::new()
        .build_with_registry(registry.clone())
        .unwrap();

    let c0 = recorder.register_counter(Key::from_name("spam"), None);
    let c1 = recorder.register_counter(Key::from_name("ham"), None);
    let g0 = recorder.register_gauge(Key::from_name("eggs"), None);

    std::thread::spawn(move || {
        use rand::distributions::Uniform;
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let counter = Uniform::from(1..=5);
        let gauge = Uniform::new_inclusive(0., 100.);

        loop {
            recorder.increment_counter(c0, rng.sample(counter));
            recorder.increment_counter(c1, rng.sample(counter));
            recorder.update_gauge(g0, rng.sample(gauge));

            std::thread::sleep(std::time::Duration::from_secs(10));
        }
    });

    let index = warp::get()
        .and(warp::path::end())
        .map(|| warp::reply::html(html::INDEX));
    let js = warp::path("graph.js")
        .map(|| warp::reply::with_header(html::JS, "content-type", "application/javascript"));
    let json =
        warp::path("data.json").map(move || warp::reply::json(&html::metrics_json(&registry)));

    let routes = index.or(js).or(json);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
