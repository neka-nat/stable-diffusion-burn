use stablediffusion_wgpu::{
    model::stablediffusion::*,
    model_download::download_model,
    tokenizer::SimpleTokenizer,
};

use burn::{module::Module, tensor::backend::Backend};

use burn_wgpu::{AutoGraphicsApi, Wgpu, WgpuDevice};

use std::process;
use tokio;

use burn::record::{self, BinFileRecorder, FullPrecisionSettings, Recorder};

fn load_stable_diffusion_model_file<B: Backend>(
    filename: &str,
) -> Result<StableDiffusion<B>, record::RecorderError> {
    BinFileRecorder::<FullPrecisionSettings>::new()
        .load(filename.into())
        .map(|record| StableDiffusionConfig::new().init().load_record(record))
}

#[tokio::main]
async fn main() {
    type Backend = Wgpu<AutoGraphicsApi, f32, i32>;
    let device = WgpuDevice::BestAvailable;

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        eprintln!("Usage: {} <unconditional_guidance_scale> <n_diffusion_steps> <prompt> <output_image_name>", args[0]);
        process::exit(1);
    }

    let unconditional_guidance_scale: f64 = args[1].parse().unwrap_or_else(|_| {
        eprintln!("Error: Invalid unconditional guidance scale.");
        process::exit(1);
    });
    let n_steps: usize = args[2].parse().unwrap_or_else(|_| {
        eprintln!("Error: Invalid number of diffusion steps.");
        process::exit(1);
    });
    let prompt = &args[3];
    let output_image_name = &args[4];

    println!("Downloading model...");
    let model_name = download_model().await.unwrap_or_else(|err| {
        eprintln!("Error downloading model: {}", err);
        process::exit(1);
    });

    println!("Loading tokenizer...");
    let tokenizer = SimpleTokenizer::new().unwrap();
    println!("Loading model...");
    let sd: StableDiffusion<Backend> = load_stable_diffusion_model_file(model_name.as_str())
        .unwrap_or_else(|err| {
            eprintln!("Error loading model: {}", err);
            process::exit(1);
        });

    let sd = sd.to_device(&device);

    let unconditional_context = sd.unconditional_context(&tokenizer);
    let context = sd.context(&tokenizer, prompt).unsqueeze::<3>(); //.repeat(0, 2); // generate 2 samples

    println!("Sampling image...");
    let images = sd.sample_image(
        context,
        unconditional_context,
        unconditional_guidance_scale,
        n_steps,
    );
    save_images(&images, output_image_name, 512, 512).unwrap_or_else(|err| {
        eprintln!("Error saving image: {}", err);
        process::exit(1);
    });
}

use image::{self, ColorType::Rgb8, ImageResult};

fn save_images(images: &Vec<Vec<u8>>, basepath: &str, width: u32, height: u32) -> ImageResult<()> {
    for (index, img_data) in images.iter().enumerate() {
        let path = format!("{}{}.png", basepath, index);
        image::save_buffer(path, &img_data[..], width, height, Rgb8)?;
    }

    Ok(())
}
