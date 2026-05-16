//! The actual runtime library.
use clap::Clap;
use color::palette;
use dither::prelude::*;

fn main() {
    let opts = Opt::parse();
    if let Err(err) = _main(&opts) {
        eprintln!("{}", err);

        std::process::exit(1)
    } else {
        std::process::exit(0)
    }
}

pub fn _main(opts: &Opt) -> Result<()> {
    let (input, output) = (&opts.input, opts.output_path()?);
    if opts.verbose {
        eprintln!(
            concat!(
                "running dither in VERBOSE mode:\n\t",
                "INPUT: {input}\n\t",
                "OUTPUT: {output}\n\t",
                "DITHERER: {dither}\n\t",
                "BIT_DEPTH: {depth}\n\t",
                "COLOR_MODE: {mode}"
            ),
            input = input.display(),
            output = output.display(),
            dither = opts.ditherer,
            depth = opts.bit_depth,
            mode = opts.color_mode,
        );
    }
    let img: Img<RGB<f64>> =
        Img::<RGB<u8>>::load(&input)?.convert_with(|rgb| rgb.convert_with(f64::from));

    if opts.verbose {
        eprintln!("image loaded from \"{}\".\ndithering...", input.display())
    }
    let quantize = dither::create_quantize_n_bits_func(opts.bit_depth)?;

    let output_img = match &opts.color_mode {
        color::Mode::Palette { .. } if opts.bit_depth > 1 => {
            return Err(Error::CustomPaletteIncompatibleWithDepth);
        }

        color::Mode::Color => opts
            .ditherer
            .dither(img, RGB::map_across(quantize))
            .convert_with(|rgb| rgb.convert_with(clamp_f64_to_u8)),

        color::Mode::Palette { palette: p, .. } => opts
            .ditherer
            .dither(img, palette::quantize(p))
            .convert_with(|rgb| rgb.convert_with(clamp_f64_to_u8)),

        color::Mode::BlackAndWhite => {
            let bw_img = img.convert_with(|rgb| rgb.to_chroma_corrected_black_and_white());
            opts.ditherer
                .dither(bw_img, quantize)
                .convert_with(RGB::from_chroma_corrected_black_and_white)
        }

        color::Mode::SingleColor(color) => {
            if opts.verbose {
                eprintln!("single_color mode: {:x}", color)
            }

            let bw_img = img.convert_with(|rgb| rgb.to_chroma_corrected_black_and_white());
            let RGB(r, g, b) = RGB::<u8>::from(*color);

            opts.ditherer
                .dither(bw_img, quantize)
                .convert_with(|x: f64| {
                    RGB(
                        clamp_f64_to_u8(f64::from(r) / 255. * x),
                        clamp_f64_to_u8(f64::from(g) / 255. * x),
                        clamp_f64_to_u8(f64::from(b) / 255. * x),
                    )
                })
        }
    };
    if opts.verbose {
        eprintln!("dithering complete.\nsaving...");
    }
    output_img.save(&output)?;
    if opts.verbose {
        eprintln!("program finished");
    }
    Ok(())
}
