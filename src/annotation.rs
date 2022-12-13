use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Annotation {
    pub folder: String,
    pub filename: String,
    pub objects: Option<Vec<Object>>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Object {
    pub name: String,
    pub bndbox: Bndbox,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Bndbox {
    pub xmin: u32,
    pub ymin: u32,
    pub xmax: u32,
    pub ymax: u32,
}

impl Bndbox {
    pub fn width(&self) -> u32 {
        self.xmax - self.xmin
    }

    pub fn height(&self) -> u32 {
        self.ymax - self.ymin
    }

    pub fn is_empty(&self) -> bool {
        self.width() == 0 || self.height() == 0
    }

    pub fn aspect_ratio(&self) -> f64 {
        let max = self.width().max(self.height());
        let min = self.width().min(self.height());
        max as f64 / min as f64
    }
}

impl Annotation {
    /// Returns a copy with only the objects satisfying to the given labels, if any.
    /// Returns None if no objects are left after filtering.
    pub fn with_filtered_objects(self, labels: &Option<Vec<String>>) -> Option<Annotation> {
        match self.objects {
            Some(objects) => {
                let filtered: Vec<Object> = objects
                    .into_iter()
                    .filter(|object| {
                        if let Some(labels) = labels {
                            labels.contains(&object.name)
                        } else {
                            true
                        }
                    })
                    .collect();
                if filtered.is_empty() {
                    None
                } else {
                    Some(Annotation {
                        objects: Some(filtered),
                        ..self
                    })
                }
            }
            None => None,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct BndboxItem {
    pub img_filename: String,
    pub width: u32,
    pub height: u32,
    pub aspect_ratio: f64,
}

pub struct BndboxItemReporter {
    csv_filename: Option<String>,
    items: Option<Vec<BndboxItem>>,
}

impl BndboxItemReporter {
    /// Reporter becomes a no-op if `csv_filename` is None.
    pub fn new(csv_filename: Option<String>) -> Self {
        csv_filename
            .map(|filename| Self {
                csv_filename: Some(filename),
                items: Some(Vec::new()),
            })
            .unwrap_or(Self {
                csv_filename: None,
                items: None,
            })
    }

    pub fn add_item(&mut self, img_filename: String, object: &Object) {
        if let Some(items) = &mut self.items {
            let item = BndboxItem {
                img_filename,
                width: object.bndbox.width(),
                height: object.bndbox.height(),
                aspect_ratio: object.bndbox.aspect_ratio(),
            };
            items.push(item);
        }
    }

    pub fn save(&mut self) {
        if let Some(items) = &self.items {
            let mut wtr = csv::Writer::from_path(self.csv_filename.as_ref().unwrap()).unwrap();
            for item in items {
                wtr.serialize(item).unwrap();
            }
            wtr.flush().unwrap();
            println!(
                "Wrote bounding box info to {:?}",
                self.csv_filename.as_ref().unwrap()
            );
        }
    }
}
