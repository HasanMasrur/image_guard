//! Dev tool: converts the ONNX model to tract's NNEF format, which the app
//! loads at runtime (no ONNX parser in the shipped binary).
//!
//!     cargo run --release --example onnx_to_nnef -- in.onnx out.nnef.tar

use tract_onnx::prelude::*;

fn main() -> TractResult<()> {
    let args: Vec<String> = std::env::args().collect();
    let (input, output) = (&args[1], &args[2]);
    let model = tract_onnx::onnx()
        .model_for_path(input)?
        .with_input_fact(0, f32::fact([1, 224, 224, 3]).into())?
        .into_typed()?
        .into_decluttered()?;
    let file = std::fs::File::create(output)?;
    tract_nnef::nnef().write_to_tar(&model, file)?;
    println!("wrote {output}");
    Ok(())
}
