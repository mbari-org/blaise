use clap::Parser;
use colored::Colorize;
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use log::debug;
use std::collections::{BTreeMap, HashMap};
use std::fs::{create_dir_all, read_to_string};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use walkdir::WalkDir;

use crate::annotation::{Annotation, Bndbox, Object};
use crate::image::{crop_image, load_image, save_image};

mod annotation;
mod image;
mod pascal;

#[derive(clap::StructOpt, Debug)]
#[structopt(global_setting(clap::AppSettings::ColoredHelp))]
#[clap(version, about = "Creates crops from PASCAL files annotated data", long_about = None)]
struct Opts {
    /// Root directory to raw dataset
    #[clap(short, long, parse(from_os_str))]
    data_dir: PathBuf,

    /// Comma separated list of labels to crop. Defaults to everything
    #[clap(short, long, use_delimiter = true)]
    labels: Option<Vec<String>>,

    /// Alternative image directory with input images
    #[clap(short, long, parse(from_os_str))]
    image_dir: Option<PathBuf>,

    /// Path to store image crops
    #[clap(short, long, parse(from_os_str))]
    output_dir: PathBuf,

    /// Verbose output (disables progress bars)
    #[structopt(long)]
    verbose: bool,

    /// Do not show progress bars
    #[structopt(long)]
    npb: bool,

    /// Number of threads to use (by default, all available)
    #[structopt(short = 'j', name = "N")]
    cores: Option<usize>,

    /// Show summary of loaded annotations
    #[structopt(short, long)]
    summary: bool,
}

fn main() {
    let started = Instant::now();
    env_logger::init();
    let opts = Opts::parse();

    let annotations = get_annotations(&opts.data_dir, &opts.labels);
    if !annotations.is_empty() {
        if opts.summary {
            show_annotation_summary(&annotations, &opts);
        }
        let cores = opts.cores.unwrap_or_else(num_cpus::get);
        let cores = cores.min(annotations.len());
        process_annotations(&opts, &annotations, cores);

        let elapsed = started.elapsed();
        if elapsed > Duration::from_secs(1) {
            println!("(Done in {})", HumanDuration(elapsed));
        }
    }
}

fn show_annotation_summary(annotations: &Vec<Annotation>, opts: &Opts) {
    let mut labels: BTreeMap<String, usize> = BTreeMap::new();
    let mut image_paths: BTreeMap<String, usize> = BTreeMap::new();
    let mut total_objects = 0;
    for annotation in annotations {
        if let Some(objects) = &annotation.objects {
            for object in objects {
                let count = labels.entry(object.name.clone()).or_insert(0);
                *count += 1;
                total_objects += 1;
            }
        }
        let count = image_paths
            .entry(
                annotation
                    .get_image_path(&opts.data_dir, &opts.image_dir)
                    .clone(),
            )
            .or_insert(0);
        *count += 1;
    }
    let mut labels: Vec<(&String, &usize)> = labels.iter().collect();
    labels.sort_by(|a, b| b.1.cmp(a.1));

    println!("\nSummary of loaded annotations:");

    println!(
        "  {} annotations with {} objects",
        annotations.len(),
        total_objects
    );
    println!("  {} labels:", labels.len());
    for (label, count) in labels {
        println!("   {:>5} \"{}\"", count, label);
    }

    // if any, show image paths referenced from multiple annotations:
    let image_paths: Vec<(&String, &usize)> = image_paths.iter().collect();
    let multi_images = image_paths.iter().filter(|(_, v)| **v > 1);
    let count = multi_images.clone().count();
    if count > 0 {
        println!("\n  Images referenced in multiple annotations:");
        for (image, count) in multi_images.clone() {
            println!("    {:>5}  {}", count, image);
        }
    }
    println!();
}

fn progress_style() -> ProgressStyle {
    ProgressStyle::with_template("{prefix:.bold.dim} {bar:40.green/yellow} {pos:>7}/{len:7}")
        .unwrap()
}

fn process_annotations(opts: &Opts, annotations: &Vec<Annotation>, cores: usize) {
    debug!("dispatching process in {} threads", cores);

    let cores = cores.min(annotations.len());
    let num_annotations = annotations.len();
    let annotations_per_thread = num_annotations / cores;
    let extra_annotations_last_thread = num_annotations % cores;

    let (tx, rx) = mpsc::channel();
    thread::scope(|s| {
        let m = MultiProgress::new();
        m.set_move_cursor(true);
        m.set_draw_target(indicatif::ProgressDrawTarget::stdout_with_hz(1));
        let sty = progress_style();

        for th in 0..cores {
            let section_lo = th * annotations_per_thread;
            let section_hi = section_lo + annotations_per_thread + {
                if th == cores - 1 {
                    extra_annotations_last_thread
                } else {
                    0
                }
            };

            if section_lo < section_hi {
                let pb = if !opts.verbose && !opts.npb {
                    let pb = m.add(ProgressBar::new((section_hi - section_lo) as u64));
                    pb.set_style(sty.clone());
                    pb.set_prefix(format!("[{:>02}]", th));
                    Some(pb)
                } else {
                    None
                };

                let c_tx = tx.clone();
                s.spawn(move || {
                    let section = &annotations[section_lo..section_hi];
                    let by_label = process_section(opts, section, th, pb);
                    c_tx.send(by_label).unwrap();
                });
            }
        }
    });

    drop(tx);

    // sorted by name
    let mut by_label: BTreeMap<String, usize> = BTreeMap::new();
    let mut sum_crops = 0usize;
    for by_label_child in &rx {
        for (label, count) in by_label_child {
            let entry = by_label.entry(label).or_insert(0);
            *entry += count;
            sum_crops += count;
        }
    }
    println!("\nCompleted a total of {} crops.", sum_crops);
    show_by_label(&by_label);
}

fn process_section(
    opts: &Opts,
    annotations: &[Annotation],
    th: usize,
    pb: Option<ProgressBar>,
) -> HashMap<String, usize> {
    let mut by_label: HashMap<String, usize> = HashMap::new();
    let mut sum_crops = 0usize;

    for (i, annotation) in annotations.iter().enumerate() {
        sum_crops +=
            process_annotation(annotation, opts, &opts.labels, &mut by_label, opts.verbose);

        if let Some(ref pb) = pb {
            pb.inc(1);
        } else if i % 10 == 0 {
            println!(
                "[{:>02}] Processing annotation {} of {}  ({} crops so far)",
                th,
                i + 1,
                annotations.len(),
                sum_crops
            );
        }
    }

    by_label
}

fn show_by_label(by_label: &BTreeMap<String, usize>) {
    println!("Crops by label:");
    let mut tot_crops = 0usize;
    for (label, total) in by_label {
        let quoted = format!("\"{}\"", label);
        println!("  {total:>5} {quoted:<40}");
        tot_crops += total;
    }
    println!("  {tot_crops:>5} total");
}

/// Returns a list of all annotations under the given directory
/// and with the indicated labels, if given.
fn get_annotations(data_dir: &PathBuf, labels: &Option<Vec<String>>) -> Vec<Annotation> {
    println!(
        "Getting annotation files under {:?}, labels: {:?}",
        data_dir, labels
    );
    let mut annotations: Vec<Annotation> = Vec::new();
    let mut skipped = 0u32;
    let mut invalid = 0u32;

    let walker = WalkDir::new(data_dir);
    for entry in walker {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() && path.extension() == Some("xml".as_ref()) {
            let src = read_to_string(entry.path()).unwrap();
            match pascal::parse_xml(src.as_str()) {
                Ok(pascal_voc) => {
                    let annotation: Annotation = pascal_voc.into();
                    match annotation.with_filtered_objects(labels) {
                        Some(annotation) => annotations.push(annotation),
                        None => skipped += 1,
                    }
                }
                Err(_) => invalid += 1,
            }
        }
    }

    println!(
        "Annotation files: {} to be processed, {} skipped, {} invalid",
        annotations.len(),
        skipped,
        invalid
    );

    annotations
}

fn process_annotation(
    annotation: &Annotation,
    opts: &Opts,
    labels: &Option<Vec<String>>,
    by_label: &mut HashMap<String, usize>,
    verbose: bool,
) -> usize {
    let Annotation {
        folder,
        filename,
        objects,
    } = annotation;

    if verbose {
        println!("process_annotation: for image: {}/{}", folder, filename);
    }

    let mut num_crops = 0usize;

    let image_path = annotation.get_image_path(&opts.data_dir, &opts.image_dir);
    let mut img = if let Ok(image) = load_image(&image_path) {
        image
    } else {
        debug!("{}", format!("failed to load image: {}", image_path).red());
        return num_crops;
    };

    let mut process_object = |i: usize, object: &Object| {
        let Object { name, bndbox } = object;
        debug!("object: i={} name={}", i, name);
        let Bndbox {
            xmin,
            ymin,
            xmax,
            ymax,
        } = bndbox;
        let x = *xmin;
        let y = *ymin;
        let width = xmax - xmin;
        let height = ymax - ymin;

        let out_class_dir = opts.output_dir.join(name);
        create_dir_all(&out_class_dir).unwrap();
        let out_path = out_class_dir.join(transform_filename(filename, i));
        if verbose {
            println!(
                "  cropping left {} right {} upper {} lower {}",
                xmin, xmax, ymin, ymax
            );
        }
        let cropped = crop_image(&mut img, x, y, width, height);
        save_image(cropped, out_path);
        num_crops += 1;

        by_label
            .entry(name.to_string())
            .and_modify(|tot| *tot += 1)
            .or_insert(1);
    };

    if let Some(objects) = objects {
        for (i, object) in objects.iter().enumerate() {
            let accept_name = if let Some(labels) = &labels {
                labels.contains(&object.name)
            } else {
                true
            };
            if accept_name {
                process_object(i, object);
            }
        }
    } else {
        debug!("no objects");
    }
    num_crops
}

fn transform_filename(filename: &str, idx: usize) -> String {
    let mut path = PathBuf::from(filename);
    path.set_extension("");
    let adjusted = path.to_str().unwrap();
    debug!(
        "transform_filename: '{}' idx={} => '{}'",
        filename, idx, adjusted
    );
    // Note: not to jpeg as in python version as some input PNGs would trigger:
    //  Unsupported(UnsupportedError { format: Exact(Jpeg), kind: Color(Rgb16) })
    format!("{}_{}.png", adjusted, idx)
}
