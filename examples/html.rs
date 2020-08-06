use metrics::{counter, gauge, register_counter, Key, Recorder};
use metrics_exporter_statsd::{HtmlExporter, MetricsBuilder};
use warp::Filter;

#[tokio::main]
async fn main() {
    let collector = MetricsBuilder::new().statsd(false).install().unwrap();
    let recorder = collector.recorder();

    let c0 = recorder.register_counter(Key::from_name("spam"), None);
    register_counter!("ham");

    std::thread::spawn(move || {
        use rand::distributions::Uniform;
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let counter = Uniform::from(1..=5);
        let gauge = Uniform::new_inclusive(0., 100.);

        loop {
            recorder.increment_counter(c0, rng.sample(counter));
            counter!("ham", rng.sample(counter));
            gauge!("eggs", rng.sample(gauge));

            std::thread::sleep(std::time::Duration::from_secs(10));
        }
    });

    let index = warp::get()
        .and(warp::path::end())
        .map(|| warp::reply::html(HtmlExporter::INDEX));
    let js = warp::path("graph.js").map(|| {
        warp::reply::with_header(HtmlExporter::JS, "content-type", "application/javascript")
    });
    let html = collector.html();
    let json = warp::path("data.json").map(move || warp::reply::json(&html.json_snapshot()));

    let routes = index.or(js).or(json);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
