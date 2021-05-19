use clap::{App, Arg, ArgMatches};
use glifparser::glif::{
    CapType, InterpolationType, JoinType, MFEKPointData, PatternCopies, PatternSubdivide, VWSHandle,
};
use glifparser::Glif;
use glifparser::Outline;
use glifparser::VWSContour;
use norad::{Font, PointType};
use std::path::Path;
use MFEKmath::pattern_along_path::pattern_along_glif;
use MFEKmath::variable_width_stroke;
use MFEKmath::PatternSettings;
use MFEKmath::Piecewise;
use MFEKmath::VWSSettings;
use MFEKmath::Vector;

#[derive(Debug)]
struct MyVWSSettings {
    cap_start_type: CapType,
    cap_end_type: CapType,
    join_type: JoinType,
    distance: f64,
    angle: f64,
}

fn parse_pattern_settings(matches: &ArgMatches) -> PatternSettings {
    let mut settings = PatternSettings {
        copies: PatternCopies::Repeated,
        subdivide: PatternSubdivide::Off,
        is_vertical: false,
        normal_offset: 0.,
        tangent_offset: 0.,
        center_pattern: true,
        pattern_scale: Vector { x: 1., y: 1. },
        spacing: 0.,
        stretch: false,
        simplify: false,
    };

    if let Some(copies) = matches.value_of("repeatmode") {
        match copies {
            "single" => settings.copies = PatternCopies::Single,
            "repeated" => settings.copies = PatternCopies::Repeated,
            _ => log::warn!("Invalid mode argument. Falling back to default. (Repeated)"),
        }
    }

    if let Some(sx_string) = matches.value_of("scale_x") {
        let sx = sx_string.parse::<f64>();

        match sx {
            Ok(n) => settings.pattern_scale.x = n,
            Err(_e) => log::warn!("Invalid scale x argument. Falling back to default. (1)"),
        }
    }

    if let Some(sy_string) = matches.value_of("scale_y") {
        let sy = sy_string.parse::<f64>();

        match sy {
            Ok(n) => settings.pattern_scale.y = n,
            Err(_e) => log::warn!("Invalid scale y argument. Falling back to default. (1)"),
        }
    }

    if let Some(sub_string) = matches.value_of("subdivide") {
        let subs = sub_string.parse::<usize>();

        match subs {
            Ok(n) => settings.subdivide = PatternSubdivide::Simple(n),
            Err(_e) => log::warn!("Invalid subdivision count. Falling back to default. (0)"),
        }
    }

    if let Some(spacing_string) = matches.value_of("spacing") {
        let spacing = spacing_string.parse::<f64>();

        match spacing {
            Ok(n) => settings.spacing = n,
            Err(_e) => log::warn!("Invalid spacing. Falling back to default. (0)"),
        }
    }

    if let Some(normal_string) = matches.value_of("normal_offset") {
        let spacing = normal_string.parse::<f64>();

        match spacing {
            Ok(n) => settings.normal_offset = n,
            Err(_e) => log::warn!("Invalid normal offset. Falling back to default. (0)"),
        }
    }

    if let Some(tangent_string) = matches.value_of("tangent_offset") {
        let spacing = tangent_string.parse::<f64>();

        match spacing {
            Ok(n) => settings.tangent_offset = n,
            Err(_e) => log::warn!("Invalid tangent offset. Falling back to default. (0)"),
        }
    }

    if let Some(center_string) = matches.value_of("center_pattern") {
        match center_string {
            "true" => settings.center_pattern = true,
            "false" => settings.center_pattern = false,
            _ => log::warn!("Invalid center pattern argument. Falling back to default. (true)"),
        }
    }

    if let Some(simplify_string) = matches.value_of("simplify") {
        match simplify_string {
            "true" => settings.simplify = true,
            "false" => settings.simplify = false,
            _ => log::warn!("Invalid center pattern argument. Falling back to default. (true)"),
        }
    }

    if let Some(stretch_string) = matches.value_of("stretch") {
        match stretch_string {
            "true" => settings.stretch = true,
            "false" => settings.stretch = false,
            _ => log::warn!("Invalid center pattern argument. Falling back to default. (true)"),
        }
    }
    settings
}

type Transformer = dyn Fn(glifparser::Glif<MFEKPointData>) -> glifparser::Glif<MFEKPointData>;

fn transform_ufo(
    layer: &norad::Layer,
    layer_base: &Path,
    output_base: &Path,
    transformer: &Transformer,
) {
    for glif in layer.iter() {
        let mut has_open_contours = false;
        for c in &glif.contours {
            if let Some(first) = c.points.first() {
                if first.typ == PointType::Move {
                    has_open_contours = true;
                    break;
                }
            }
        }
        if !has_open_contours {
            continue;
        }
        log::info!("Stroking glyph {:}", glif.name);
        let input_path = layer_base.join(layer.get_path(&glif.name).unwrap());
        let path = glifparser::read_from_filename(&input_path).expect("Failed to read path file!");
        let output = transformer(path);
        let output_path = output_base.join(layer.get_path(&glif.name).unwrap());
        glifparser::write_to_filename(&output, output_path).expect("Failed to write glyph");
    }
}

fn my_vws_path<U: glifparser::PointData>(
    path: &Glif<U>,
    settings: VWSSettings,
    my_settings: &MyVWSSettings,
) -> Glif<MFEKPointData> {
    // convert our path and pattern to piecewise collections of beziers
    let piece_path = Piecewise::from(path.outline.as_ref().unwrap());
    let mut output_outline: Outline<MFEKPointData> = Vec::new();
    for pwpath_contour in &piece_path.segs {
        let mut vws_contour = VWSContour {
            handles: Vec::new(),
            cap_start_type: my_settings.cap_start_type,
            cap_end_type: my_settings.cap_end_type,
            join_type: my_settings.join_type,
            remove_internal: false, // TODO: Add these to <lib>
            remove_external: false,
        };
        let mut count = pwpath_contour.segs.len() + 1;
        while count > 0 {
            vws_contour.handles.push(VWSHandle {
                left_offset: my_settings.distance,
                right_offset: my_settings.distance,
                tangent_offset: my_settings.angle,
                interpolation: InterpolationType::Linear,
            });
            count -= 1;
        }
        let results = variable_width_stroke(&pwpath_contour, &vws_contour, &settings);
        for result_contour in results.segs {
            output_outline.push(result_contour.to_contour());
        }
    }

    Glif {
        outline: Some(output_outline),
        order: path.order, // default when only corners
        anchors: path.anchors.clone(),
        width: path.width,
        unicode: path.unicode.clone(),
        name: path.name.clone(),
        format: 2,
        lib: None,
        components: path.components.clone(),
        guidelines: path.guidelines.clone(),
        images: path.images.clone(),
        note: path.note.clone(),
        filename: path.filename.clone(),
        private_lib: path.private_lib.clone(),
        private_lib_root: path.private_lib_root,
    }
}

fn main() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );
    let matches = App::new("ufostroker")
        .version("0.0.0")
        .author("Simon Cozens <simon@simon-cozens.org>")
        .about("A utility for applying path effects to UFO files.")
        .subcommand(
        	clap::SubCommand::with_name("noodle").
        		about("Apply a noodle effect to each open path")
        		.arg(Arg::with_name("capstart")
            .long("capstart")
            .takes_value(true)
            .help("<round|circle|square> cap end to add to start of strokes"))
	        .arg(Arg::with_name("capend")
	            .long("capend")
	            .takes_value(true)
	            .help("<round|circle|square> cap end to add to end of strokes"))
        .arg(Arg::with_name("size")
            .long("size")
            .takes_value(true)
            .help("<f64 (10)> size of the noodle"))
        .arg(Arg::with_name("angle")
            .long("angle")
            .takes_value(true)
            .help("<f64 (0)> angle of noodle from tangent"))

      	).subcommand(
      		clap::SubCommand::with_name("pattern").
        		about("Apply a pattern-along-path to each open path")
        .arg(Arg::with_name("pattern_glyph")
            .long("pattern-glyph")
            .short("p")
            .takes_value(true)
            .help("The glyph containing the input pattern.")
		        .required(true))
        .arg(Arg::with_name("repeatmode")
            .short("r")
            .long("repeat-mode")
            .takes_value(true)
            .help("<[single|repeated] (repeated)> set our repeat mode."))
        .arg(Arg::with_name("scale_x")
            .long("sx")
            .takes_value(true)
            .help("<f64 (1)> how much we scale our input pattern on the x-axis."))
        .arg(Arg::with_name("scale_y")
            .long("sy")
            .takes_value(true)
            .help("<f64 (1)> how much we scale our input pattern on the y-axis."))
        .arg(Arg::with_name("subdivide")
            .short("sub")
            .long("subdivide")
            .takes_value(true)
            .help("<f64 (0)> how many times to subdivide the patterns at their midpoint."))
        .arg(Arg::with_name("spacing")
            .long("spacing")
            .takes_value(true)
            .help("<f64 (0)> how much padding to trail each copy with."))
        .arg(Arg::with_name("normal_offset")
            .long("noffset")
            .takes_value(true)
            .help("<f64 (0)> how much to offset the pattern along the normal of the path."))
        .arg(Arg::with_name("tangent_offset")
            .long("toffset")
            .takes_value(true)
            .help("<f64 (0)> how much to offset the pattern along the tangent of the path."))
        .arg(Arg::with_name("stretch")
            .long("stretch")
            .takes_value(true)
            .help("<boolean (false)> whether to stretch the input pattern or not."))
        .arg(Arg::with_name("simplify")
            .long("simplify")
            .takes_value(true)
            .help("<boolean (false)> if we should run the result through a simplify routine."))
        .arg(Arg::with_name("center_pattern")
            .long("center_pattern")
            .takes_value(true)
            .help("<boolean (true)> if you want to align a pattern manually you can change this to false."))
        ).arg(Arg::with_name("ufo")
            .long("ufo")
            .short("i")
            .takes_value(true)
            .help("The input UFO file.")
            .required(true))
        .arg(Arg::with_name("output")
            .long("output")
            .short("o")
            .takes_value(true)
            .help("The output UFO file.")
            )
        .get_matches();

    let ufo_file = matches.value_of("ufo").unwrap(); // required options shouldn't panic?
    let font_obj = Font::load(ufo_file).expect("failed to load font");
    let layer = font_obj.default_layer();
    let layer_base = Path::new(ufo_file).join(layer.path());
    let output_base = if let Some(output) = matches.value_of("output") {
        dircpy::copy_dir(ufo_file, output).expect("Could not write output UFO");
        Path::new(output).join(layer.path())
    } else {
        layer_base.clone()
    };

    match matches.subcommand() {
        ("pattern", Some(pattern_matches)) => {
            let pattern_glif = pattern_matches.value_of("pattern_glyph").unwrap();
            if !layer.contains_glyph(pattern_glif) {
                log::error!("Glyph '{:}' not found in font", pattern_glif);
                return;
            }
            let pattern_string = layer_base.join(
                layer
                    .get_path(&pattern_glif)
                    .expect("Couldn't open pattern glyph"),
            );
            log::info!("Opening pattern file {:?}", pattern_string);
            let pattern: glifparser::Glif<MFEKPointData> =
                glifparser::read_from_filename(pattern_string)
                    .expect("Could not read pattern file");
            let settings = parse_pattern_settings(&pattern_matches);
            let closure = move |path| pattern_along_glif(&path, &pattern, &settings);
            transform_ufo(&layer, &layer_base, &output_base, &closure);
        }
        ("noodle", Some(noodle_matches)) => {
            let round_str = "round".to_string();
            let cap_start = noodle_matches.value_of("capstart").unwrap_or(&round_str);
            let cap_end = noodle_matches.value_of("capend").unwrap_or(&round_str);
            let join = noodle_matches.value_of("join").unwrap_or(&round_str);

            let cap_start_type = match cap_start {
                "round" => CapType::Round,
                "circle" => CapType::Circle,
                "square" => CapType::Square,
                "custom" => CapType::Custom,
                _ => panic!("Invalid cap type!"),
            };

            let cap_end_type = match cap_end {
                "round" => CapType::Round,
                "circle" => CapType::Circle,
                "square" => CapType::Square,
                "custom" => CapType::Custom,
                _ => panic!("Invalid cap type!"),
            };

            let join_type = match join {
                "round" => JoinType::Round,
                "circle" => JoinType::Circle,
                "miter" => JoinType::Miter,
                "bevel" => JoinType::Bevel,
                _ => panic!("Invalid join type!"),
            };

            let mut my_settings = MyVWSSettings {
                join_type,
                cap_start_type,
                cap_end_type,
                distance: 10.0,
                angle: 0.0,
            };

            if let Some(size_string) = noodle_matches.value_of("size") {
                let sx = size_string.parse::<f64>();
                match sx {
                    Ok(n) => my_settings.distance = n,
                    Err(_e) => log::warn!("Invalid size argument. Falling back to default. (1)"),
                }
            }
            if let Some(angle_string) = noodle_matches.value_of("angle") {
                let sx = angle_string.parse::<f64>();
                match sx {
                    Ok(n) => my_settings.angle = n,
                    Err(_e) => log::warn!("Invalid angle argument. Falling back to default. (1)"),
                }
            }

            let closure = move |path| {
                my_vws_path(
                    &path,
                    VWSSettings {
                        cap_custom_end: None,
                        cap_custom_start: None,
                    },
                    &my_settings,
                )
            };
            transform_ufo(&layer, &layer_base, &output_base, &closure);
        }
        _ => {
            log::error!("Unknown mode");
        }
    }
}
