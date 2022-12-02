use crate::annotation;
use serde::Deserialize;
use serde_xml_rs::from_str;
use serde_xml_rs::Error;

pub fn parse_xml(src: &str) -> Result<PascalVoc, Error> {
    from_str(src)
}

impl From<PascalVoc> for annotation::Annotation {
    fn from(pascal_voc: PascalVoc) -> Self {
        let folder = pascal_voc.folder;
        let filename = pascal_voc.filename;

        let objects = match pascal_voc.objects {
            Some(objects) => {
                let mut objects: Vec<annotation::Object> = objects
                    .into_iter()
                    .map(|object| annotation::Object {
                        name: object.name,
                        bndbox: annotation::Bndbox {
                            xmin: object.bndbox.xmin.0,
                            ymin: object.bndbox.ymin.0,
                            xmax: object.bndbox.xmax.0,
                            ymax: object.bndbox.ymax.0,
                        },
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

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct PascalVoc {
    pub folder: String,
    pub filename: String,
    pub size: Size,
    #[serde(rename = "object")]
    pub objects: Option<Vec<Object>>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Size {
    pub width: String,
    pub height: String,
    pub depth: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Object {
    pub name: String,
    pub bndbox: Bndbox,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Bndbox {
    pub xmin: CoordVal,
    pub ymin: CoordVal,
    pub xmax: CoordVal,
    pub ymax: CoordVal,
}

/// Bndbox members can be integers or floats, but we always parse them as u32
#[derive(Debug, serde_with::DeserializeFromStr, PartialEq, Eq)]
pub struct CoordVal(pub u32);

impl std::str::FromStr for CoordVal {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let i = if let Ok(v) = s.parse::<u32>() {
            v
        } else {
            s.parse::<f32>().unwrap() as u32
        };
        Ok(CoordVal(i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn filter_objects(
        objects: Option<Vec<Object>>,
        labels: &Option<Vec<String>>,
    ) -> Option<Vec<Object>> {
        match objects {
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
                    Some(filtered)
                }
            }
            None => None,
        }
    }

    const XML0: &str = r#"
        <annotation>
            <folder>imgs</folder>
            <filename>IMG_TEST.png</filename>
            <size>
                <width>400</width>
                <height>300</height>
                <depth>3</depth>
            </size>
        </annotation>
    "#;

    const XML1: &str = r#"
        <annotation>
            <folder>imgs</folder>
            <filename>IMG_TEST.png</filename>
            <size>
                <width>400</width>
                <height>300</height>
                <depth>3</depth>
            </size>
            <object>
                <name>FOO</name>
                <bndbox>
                    <xmin>55.0</xmin>
                    <ymin>145.0</ymin>
                    <xmax>150.0</xmax>
                    <ymax>220.0</ymax>
                </bndbox>
            </object>
        </annotation>
    "#;

    const XML2: &str = r#"
        <annotation>
        <folder>imgs</folder>
        <filename>IMG_TEST.png</filename>
        <size>
            <width>400</width>
            <height>300</height>
            <depth>3</depth>
        </size>
        <object>
            <name>FOO</name>
            <bndbox>
                <xmin>55</xmin>
                <ymin>145</ymin>
                <xmax>150</xmax>
                <ymax>220</ymax>
            </bndbox>
        </object>
        <object>
            <name>PENIAGONE_VITREA</name>
            <pose>Unspecified</pose>
            <truncated>0</truncated>
            <occluded>0</occluded>
            <difficult>0</difficult>
            <bndbox>
                <xmin>55.0</xmin>
                <ymin>145</ymin>
                <xmax>150</xmax>
                <ymax>220.1</ymax>
            </bndbox>
        </object>
    </annotation>
    "#;

    #[inline]
    fn expected_pascal_voc2() -> PascalVoc {
        PascalVoc {
            folder: "imgs".to_string(),
            filename: "IMG_TEST.png".to_string(),
            size: Size {
                width: "400".to_string(),
                height: "300".to_string(),
                depth: "3".to_string(),
            },
            objects: Some(vec![
                Object {
                    name: "FOO".to_string(),
                    bndbox: Bndbox {
                        xmin: CoordVal(55),
                        ymin: CoordVal(145),
                        xmax: CoordVal(150),
                        ymax: CoordVal(220),
                    },
                },
                Object {
                    name: "PENIAGONE_VITREA".to_string(),
                    bndbox: Bndbox {
                        xmin: CoordVal(55),
                        ymin: CoordVal(145),
                        xmax: CoordVal(150),
                        ymax: CoordVal(220),
                    },
                },
            ]),
        }
    }

    const NO_LABELS: Option<Vec<String>> = None;

    #[test]
    fn no_objects() {
        let pascal_voc = parse_xml(XML0).unwrap();
        let objects = filter_objects(pascal_voc.objects, &NO_LABELS);
        assert_eq!(objects, None);
    }

    #[test]
    fn one_object_bndbox_with_floats() {
        let doc: PascalVoc = parse_xml(XML1).unwrap();
        assert_eq!(
            doc,
            PascalVoc {
                folder: "imgs".to_string(),
                filename: "IMG_TEST.png".to_string(),
                size: Size {
                    width: "400".to_string(),
                    height: "300".to_string(),
                    depth: "3".to_string(),
                },
                objects: Some(vec![Object {
                    name: "FOO".to_string(),
                    bndbox: Bndbox {
                        xmin: CoordVal(55),
                        ymin: CoordVal(145),
                        xmax: CoordVal(150),
                        ymax: CoordVal(220),
                    },
                },]),
            }
        );
    }

    #[test]
    fn multiple_objects() {
        let pascal_voc = parse_xml(XML2).unwrap();
        assert_eq!(pascal_voc, expected_pascal_voc2());
    }

    #[test]
    fn filter_objects1() {
        let labels: Option<Vec<String>> = Some(vec!["PENIAGONE_VITREA".to_string()]);
        let pascal_voc = parse_xml(XML2).unwrap();
        let pascal_voc = PascalVoc {
            objects: filter_objects(pascal_voc.objects, &labels),
            ..pascal_voc
        };
        assert_eq!(
            pascal_voc,
            PascalVoc {
                folder: "imgs".to_string(),
                filename: "IMG_TEST.png".to_string(),
                size: Size {
                    width: "400".to_string(),
                    height: "300".to_string(),
                    depth: "3".to_string(),
                },
                objects: Some(vec![Object {
                    name: "PENIAGONE_VITREA".to_string(),
                    bndbox: Bndbox {
                        xmin: CoordVal(55),
                        ymin: CoordVal(145),
                        xmax: CoordVal(150),
                        ymax: CoordVal(220),
                    },
                },]),
            }
        );
    }

    #[test]
    fn filter_objects2() {
        let labels: Option<Vec<String>> =
            Some(vec!["FOO".to_string(), "PENIAGONE_VITREA".to_string()]);
        let pascal_voc = parse_xml(XML2).unwrap();
        let pascal_voc = PascalVoc {
            objects: filter_objects(pascal_voc.objects, &labels),
            ..pascal_voc
        };
        assert_eq!(pascal_voc, expected_pascal_voc2());
    }
}
