//! Developer tool: print NSFW scores for image files.
//!
//!     cargo run --release --example classify -- photo1.jpg photo2.png

use image_guard::api::safety::{classify_file, load_model};
use image_guard::api::types::SafetyOptions;

fn main() {
    let model = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../assets/models/nsfw_mobilenet_v2_140_224.nnef.tar"
    );
    load_model(std::fs::read(model).expect("model file")).expect("load model");
    for path in std::env::args().skip(1) {
        match classify_file(path.clone(), SafetyOptions::default()) {
            Ok(r) => {
                let s = r.scores;
                println!(
                    "{path}: {:?} nsfw={:.3} | drawings {:.3} hentai {:.3} neutral {:.3} porn {:.3} sexy {:.3} | {} ms",
                    r.verdict, r.nsfw_score, s.drawings, s.hentai, s.neutral, s.porn, s.sexy, r.elapsed_ms
                );
            }
            Err(e) => println!("{path}: error {e}"),
        }
    }
}
