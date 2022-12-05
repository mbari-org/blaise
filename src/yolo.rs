use crate::annotation;
use imagesize::ImageSize;
use std::error::Error;
use std::str::FromStr;

type Res<T> = Result<T, Box<dyn Error>>;

pub fn parse_yolo(
    folder: &str,
    filename: &str,
    image_size: &ImageSize,
    class_id_to_name: impl Fn(u32) -> String,
    src: &str,
) -> Res<Yolo> {
    fn parse<F: FromStr>(s: Option<&str>) -> Res<F> {
        let s = s.ok_or("expected a string")?;
        s.parse::<F>()
            .map_err(|_| format!("cannot parse '{}'", s).into())
    }

    let parse_object = |line: &str| -> Res<Object> {
        let mut parts = line.split_whitespace();
        let class_id: u32 = parse(parts.next())?;
        Ok(Object {
            name: class_id_to_name(class_id),
            x: parse(parts.next())?,
            y: parse(parts.next())?,
            width: parse(parts.next())?,
            height: parse(parts.next())?,
        })
    };

    let objects: Vec<Object> = src
        .split('\n')
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(parse_object)
        .collect::<Vec<Res<Object>>>()
        .into_iter()
        .collect::<Res<Vec<_>>>()?;

    Ok(Yolo {
        folder: folder.to_string(),
        filename: filename.to_string(),
        image_size: *image_size,
        objects: if objects.is_empty() {
            None
        } else {
            Some(objects)
        },
    })
}

impl From<Yolo> for annotation::Annotation {
    fn from(yolo: Yolo) -> Self {
        let Yolo {
            folder,
            filename,
            image_size,
            objects,
        } = yolo;

        let image_width = image_size.width as f64;
        let image_height = image_size.height as f64;

        let objects = match objects {
            Some(objects) => {
                let mut objects: Vec<annotation::Object> = objects
                    .into_iter()
                    .map(|object| {
                        let Object {
                            name,
                            mut x,
                            mut y,
                            mut width,
                            mut height,
                        } = object;

                        // Per https://bitbucket.org/mbari/m3-download/src/main/scripts/yolo_to_voc.py:
                        // Shift x, y from center to upper-left
                        x -= width / 2.;
                        y -= height / 2.;
                        // Scale
                        x *= image_width;
                        y *= image_height;
                        width *= image_width;
                        height *= image_height;

                        let xmin = x.round() as u32;
                        let ymin = y.round() as u32;
                        let xmax = xmin + width.round() as u32;
                        let ymax = ymin + height.round() as u32;

                        annotation::Object {
                            name,
                            bndbox: annotation::Bndbox {
                                xmin,
                                ymin,
                                xmax,
                                ymax,
                            },
                        }
                    })
                    .collect();
                objects.sort_by(|a, b| a.name.cmp(&b.name));
                Some(objects)
            }
            None => None,
        };

        annotation::Annotation {
            folder,
            filename,
            objects,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Yolo {
    pub folder: String,
    pub filename: String,
    pub image_size: ImageSize,
    pub objects: Option<Vec<Object>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Object {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use pretty_assertions::assert_eq;

    fn class_id_to_name(class_id: u32) -> String {
        format!("class_{}", class_id)
    }

    fn assert_eq_objects(obj: &Object, exp: &Object) {
        assert_eq!(obj.name, exp.name);
        assert_relative_eq!(obj.x, exp.x);
        assert_relative_eq!(obj.y, exp.y);
        assert_relative_eq!(obj.width, exp.width);
        assert_relative_eq!(obj.height, exp.height);
    }

    const YOLO0: &str = "";

    const YOLO1: &str = r#"
        42 0.38 0.33 0.07 0.13
    "#;

    const YOLO2: &str = r#"
        3 0.2265625 0.8189814814814815 0.027083333333333334 0.08981481481481482
        5 0.3848958333333333 0.33611111111111114 0.07916666666666666 0.1388888888888889
    "#;

    const IMAGE_SIZE: ImageSize = ImageSize {
        width: 640,
        height: 480,
    };

    #[test]
    fn yolo0() {
        let yolo = parse_yolo("D", "FN", &IMAGE_SIZE, class_id_to_name, YOLO0).unwrap();
        assert_eq!(yolo.objects, None);
    }

    #[test]
    fn yolo1() {
        let yolo = parse_yolo("D", "FN", &IMAGE_SIZE, class_id_to_name, YOLO1).unwrap();
        let expected_objects = vec![Object {
            name: "class_42".to_string(),
            x: 0.38,
            y: 0.33,
            width: 0.07,
            height: 0.13,
        }];
        let objects = yolo.objects.unwrap();
        let objects = objects.as_slice();
        println!("objects={:?}", objects);
        assert_eq!(objects.len(), expected_objects.len());
        assert_eq_objects(&objects[0], &expected_objects[0]);
    }

    #[test]
    fn yolo2() {
        let yolo = parse_yolo("D", "FN", &IMAGE_SIZE, class_id_to_name, YOLO2).unwrap();
        let expected_objects = vec![
            Object {
                name: "class_3".to_string(),
                x: 0.2265625,
                y: 0.8189814814814815,
                width: 0.027083333333333334,
                height: 0.08981481481481482,
            },
            Object {
                name: "class_5".to_string(),
                x: 0.3848958333333333,
                y: 0.33611111111111114,
                width: 0.07916666666666666,
                height: 0.1388888888888889,
            },
        ];
        let objects = yolo.objects.clone().unwrap();
        let objects = objects.as_slice();
        assert_eq!(objects.len(), expected_objects.len());
        assert_eq_objects(&objects[0], &expected_objects[0]);
        assert_eq_objects(&objects[1], &expected_objects[1]);

        // as annotation:
        let annotation: annotation::Annotation = yolo.into();
        let ann_objects = annotation.objects.unwrap();
        assert_eq!(
            ann_objects,
            vec![
                annotation::Object {
                    name: "class_3".to_string(),
                    bndbox: annotation::Bndbox {
                        xmin: 136,
                        ymin: 372,
                        xmax: 153,
                        ymax: 415,
                    },
                },
                annotation::Object {
                    name: "class_5".to_string(),
                    bndbox: annotation::Bndbox {
                        xmin: 221,
                        ymin: 128,
                        xmax: 272,
                        ymax: 195,
                    },
                },
            ]
        );
    }
}
