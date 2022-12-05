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
