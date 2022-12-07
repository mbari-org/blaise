use clap::Parser;
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use log::debug;
use std::collections::{BTreeMap, HashMap};
use std::fs::{create_dir_all, read_to_string};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use walkdir::{DirEntry, WalkDir};

use crate::annotation::{Annotation, Bndbox, Object};
use crate::image::{crop_image, load_image, resize_image, save_image};

mod annotation;
mod image;
mod pascal;
mod yolo;

#[derive(clap::Parser, Debug)]
#[structopt(global_setting(clap::AppSettings::ColoredHelp))]
#[clap(version, about = "Creates image crops for given annotations", long_about = None)]
struct Opts {
    /// Base directory to scan for pascal voc annotations
    #[clap(short, long, value_name = "dir", parse(from_os_str))]
    pascal: Option<PathBuf>,

    /// Use yolo annotations
    #[clap(short, long, value_names = &["image-dir", "label-dir", "names-file"], number_of_values = 3, parse(from_os_str))]
    yolo: Option<Vec<PathBuf>>,

    /// Image base directory
    #[clap(short, long, value_name = "dir", parse(from_os_str))]
    image_dir: Option<PathBuf>,

    /// Resize the resulting crops (aspect ratio not necessarily preserved)
    #[clap(short, long, value_names = &["width", "height"], number_of_values = 2)]
    resize: Option<Vec<u32>>,

    /// Comma separated list of labels to crop. Defaults to everything
    #[clap(short = 'L', long, value_name = "labels", use_delimiter = true)]
    select_labels: Option<Vec<String>>,

    /// Path to store image crops
    #[clap(short, long, value_name = "dir", parse(from_os_str))]
    output_dir: PathBuf,

    /// Verbose output (disables progress bars)
    #[clap(long)]
    verbose: bool,

    /// Do not show progress bars
    #[clap(long)]
    npb: bool,

    /// Number of threads to use (by default, all available)
    #[clap(short = 'j', name = "N")]
    cores: Option<usize>,
}

fn main() {
    let started = Instant::now();
    env_logger::init();
    let opts = Opts::parse();

    let annotations = get_annotations(&opts);
    if !annotations.is_empty() {
        show_annotation_summary(&annotations, &opts);
        process_annotations(&opts, &annotations, started);
    }
}

/// Returns a list of all annotations according to options.
fn get_annotations(opts: &Opts) -> Vec<Annotation> {
    let mut annotations: Vec<Annotation> = Vec::new();
    if opts.pascal.is_some() {
        get_pascal_annotations(opts, &mut annotations);
    } else {
        get_yolo_annotations(opts, &mut annotations);
    }
    annotations
}

fn get_pascal_annotations(opts: &Opts, annotations: &mut Vec<Annotation>) {
    let data_dir = &opts.pascal.as_ref().unwrap();
    let labels = &opts.select_labels;
    println!(
        "getting pascal annotations under {:?}, labels: {:?}",
        data_dir, labels
    );
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
        "Pascal annotation files: {} to be processed, {} skipped, {} invalid",
        annotations.len(),
        skipped,
        invalid
    );
}

fn get_yolo_annotations(opts: &Opts, annotations: &mut Vec<Annotation>) {
    let yolo = opts.yolo.as_ref().unwrap();
    let image_dir = yolo.get(0).unwrap();
    let yolo_dir = yolo.get(1).unwrap();
    let yolo_names_filename = yolo.get(2).unwrap();
    println!(
        "processing yolo annotations with:
          image_dir:  {:?}
          yolo_dir:   {:?}
          yolo_names: {:?}",
        image_dir, yolo_dir, yolo_names_filename
    );

    let yolo_names: Vec<String> = read_to_string(yolo_names_filename)
        .unwrap()
        .split('\n')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    println!("yolo names loaded: {}", yolo_names.len());

    debug!(
        "yolo_names({}): first few={:?}",
        yolo_names.len(),
        &yolo_names[0..5.min(yolo_names.len())]
    );

    let class_id_to_name = |class_id: u32| -> String {
        if class_id < yolo_names.len() as u32 {
            yolo_names[class_id as usize].clone()
        } else {
            format!("class_{}", class_id)
        }
    };

    let labels = &opts.select_labels;
    println!(
        "getting yolo annotations based on image_dir {:?}",
        image_dir
    );

    fn is_image(path: &DirEntry) -> bool {
        static X: [&str; 3] = ["png", "jpg", "jpeg"];
        let path = path.path();
        path.is_file()
            && match path.extension() {
                Some(extension) => X.contains(&extension.to_str().unwrap()),
                None => false,
            }
    }

    let image_entries: Vec<DirEntry> = WalkDir::new(image_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(is_image)
        .collect();
    println!("image files: {}", image_entries.len());

    let image_filenames: Vec<(String, imagesize::ImageSize)> = image_entries
        .into_iter()
        .map(|e| {
            let filename = e.file_name().to_string_lossy().into_owned();
            let size = imagesize::size(e.path()).unwrap();
            (filename, size)
        })
        .collect();

    debug!(
        "image_filenames({}): first few={:?}",
        image_filenames.len(),
        &image_filenames[0..5.min(image_filenames.len())]
    );

    fn replace_to_txt(e: &str) -> String {
        let base = e
            .rfind('.')
            .map(|i| e[..i].to_string())
            .unwrap_or_else(|| e.to_string());
        base + ".txt"
    }

    let mut yolos: Vec<yolo::Yolo> = Vec::new();
    let mut invalid = 0u32;
    for (image_filename, image_size) in &image_filenames {
        let yolo_filename = replace_to_txt(image_filename);
        let path = yolo_dir.join(yolo_filename);
        let src = read_to_string(path).unwrap();
        match yolo::parse_yolo(
            yolo_dir.to_string_lossy().as_ref(),
            image_filename.as_str(),
            image_size,
            class_id_to_name,
            src.as_str(),
        ) {
            Ok(yolo) => yolos.push(yolo),
            Err(_) => invalid += 1,
        }
    }
    debug!("yolos={:?}", yolos);

    let mut skipped = 0u32;
    for yolo in yolos {
        let annotation: Annotation = yolo.into();
        match annotation.with_filtered_objects(labels) {
            Some(annotation) => annotations.push(annotation),
            None => skipped += 1,
        }
    }

    println!(
        "Yolo annotation files: {} to be processed, {} skipped, {} invalid",
        annotations.len(),
        skipped,
        invalid
    );
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
            .entry(get_image_path(annotation, opts).clone())
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

fn get_image_path(annotation: &Annotation, opts: &Opts) -> String {
    let image_dir: String = match &opts.image_dir {
        Some(dir) => dir.to_str().unwrap().to_string(),
        None => match &opts.yolo {
            Some(yolo) => yolo.get(0).unwrap().to_str().unwrap().to_string(),
            None => {
                let pascal_dir = opts.pascal.as_ref().unwrap();
                format!("{}/{}", pascal_dir.to_str().unwrap(), annotation.folder)
            }
        },
    };
    format!("{}/{}", image_dir, annotation.filename)
}

fn progress_style() -> ProgressStyle {
    ProgressStyle::with_template("{prefix:.bold.dim} {bar:40.green/yellow} {pos:>7}/{len:7}")
        .unwrap()
}

fn process_annotations(opts: &Opts, annotations: &Vec<Annotation>, started: Instant) {
    let cores = opts.cores.unwrap_or_else(num_cpus::get);
    let cores = cores.min(annotations.len());
    do_process_annotations(opts, annotations, cores);
    let elapsed = started.elapsed();
    if elapsed > Duration::from_secs(1) {
        println!("(Done in {})", HumanDuration(elapsed));
    }
}

fn do_process_annotations(opts: &Opts, annotations: &Vec<Annotation>, cores: usize) {
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
        sum_crops += process_annotation(
            annotation,
            opts,
            &opts.select_labels,
            &mut by_label,
            opts.verbose,
        );

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

    let image_path = get_image_path(annotation, opts);
    let mut img = match load_image(&image_path) {
        Ok(image) => image,
        Err(e) => {
            eprintln!("ERROR: failed to load image {}: {:?}", image_path, e);
            return num_crops;
        }
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
        if let Some(r) = &opts.resize {
            let width = *r.first().unwrap();
            let height = *r.get(1).unwrap();
            if let Some(resized) = resize_image(&cropped, width, height) {
                save_image(resized, &out_path);
            } else {
                eprintln!("WARN: not resizing empty image: {:?}", out_path);
            }
        } else {
            save_image(cropped, out_path);
        }
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
